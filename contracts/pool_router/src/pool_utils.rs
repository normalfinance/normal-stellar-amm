use crate::events::{Events, PoolRouterEvents};
use crate::liquidity_calculator::LiquidityCalculatorClient;
use crate::rewards::get_rewards_manager;
use crate::storage::{
    get_insurance_fund, get_oracle_registry, get_pool, get_pool_hash, get_pool_plane,
    get_pools_vec, get_token_share_hash, put_pool, set_pools_vec,
};
use access_control::access::AccessControl;
use access_control::management::{MultipleAddressesManagementTrait, SingleAddressManagementTrait};
use access_control::role::Role;
use rewards::storage::RewardTokenStorageTrait;
use soroban_sdk::{
    symbol_short, xdr::ToXdr, Address, Bytes, BytesN, Env, IntoVal, Symbol, Val, Vec,
};
use soroban_sdk::{String, U256};
use utils::state::{
    access::PrivilegedAddresses,
    pool::{InitializeAllParams, PoolConfig, PoolStatus, PoolTier, RewardConfig},
    token::TokenInitInfo,
};

/* Salt Methodology

Most AMMs use a merged salt consisting of two parts:
1) A token salt (token_a and token_b)
2) A fee percentage salt

This combined salt ensures only a single pool for each fee tier for each token pair
may be created. This structure works fine for traditional AMMs, but does not satisfy
our desired requirements.

Instead, the Normal pool salt is solely composed of a base asset symbol (the symbol
of the asset being synthetically tracked, i.e. "BTC"). This symbol correlates to a
single oracle registered with the Oracle Registry contract which also enforces the
same salt. This ensures only a single pool for each synthetically tracked asset
may be created.

This design is intentional for three reasons:
1)  There are limited quote assets with sufficient behavior to collateralize a synthetic asset.

    Tokens like XLM have deep liquidity, broad support, and most importantly, similar price
    volatility to the assets being synthetically tracked. This volatility correlation is
    deeply important to the protocol so that liquidity imbalances do not grow overly large.
    Assets like USDC (and stablecoins in general) or other more/less volatile assets do not
    have a reliably similar volatility which creates too much risk of large liquidity imbalance.

2)  To consolidate liquidity into a single, higher quality primary market pool.

    If multiple pools were supported for each synthetic asset, liquidity would naturally be
    split amongst them all. This may give traders favorable swap paths, but it creates
    larger slippage and price impact per trade.

3)  To decentralize secondary market creation and promote arbitrage accordingly.

    Normal's only goal is to create and maintain a reliable protocol with pools that have deep
    liquidity and a secure price peg. 3rd party users are encouraged to create pools for
    Normal Tokens with their desired quote token(s) on secondary markets such as Aquarius,
    Soroswap, Phoenix, and more. This helps achieve more sufficient decentralization and
    creates better trading access and conditions given the natural arbitrage that follows
    primary to secondary market dynamics.
*/
fn get_pool_salt(e: &Env, asset: &Symbol) -> BytesN<32> {
    let mut salt = Bytes::new(e);
    salt.append(&symbol_short!("0x00").to_xdr(e));
    salt.append(&asset.to_xdr(e));
    salt.append(&symbol_short!("0x00").to_xdr(e));
    e.crypto().sha256(&salt).to_bytes()
}

pub fn deploy_pool(
    e: &Env,
    token_b: &Address,
    assets: &(Symbol, Symbol),
    token_a_sac_address: &Address,
    share_token_info: &(String, String),
    fees_config: &(u32, u32),
    tier: &PoolTier,
    max_insurance: u128,
) -> Address {
    let pool_wasm_hash = get_pool_hash(e);
    let (base_asset, _) = assets;

    let pool_contract_id = e
        .deployer()
        .with_current_contract(get_pool_salt(e, &base_asset))
        .deploy_v2(pool_wasm_hash, ());

    init_pool(
        e,
        token_b,
        assets,
        &pool_contract_id,
        token_a_sac_address,
        share_token_info,
        fees_config,
        tier,
        max_insurance,
    );

    // Add pool contract address to Map<Symbol, Address>
    put_pool(e, base_asset.clone(), &pool_contract_id.clone());

    // Add pool contract address to Vec<Address>
    let mut pools_vec = get_pools_vec(&e);
    pools_vec.push_back(pool_contract_id.clone());
    set_pools_vec(&e, &pools_vec);

    Events::new(e).add_pool(
        base_asset.clone(),
        token_b.clone(),
        pool_contract_id.clone(),
        Vec::<Val>::from_array(
            e,
            [
                fees_config.0.into_val(e),
                tier.into_val(e),
                max_insurance.into_val(e),
            ],
        ),
    );

    pool_contract_id
}

fn init_pool(
    e: &Env,
    token_b: &Address,
    assets: &(Symbol, Symbol),
    pool_contract_id: &Address,
    token_a_sac_address: &Address,
    share_token_info: &(String, String),
    fees_config: &(u32, u32),
    tier: &PoolTier,
    max_insurance: u128,
) {
    let (fee_fraction, protocol_fee_fraction) = *fees_config;
    let share_token_wasm_hash = get_token_share_hash(e);
    let rewards = get_rewards_manager(e);
    let reward_token = rewards.storage().get_reward_token();
    let access_control = AccessControl::new(e);

    // privileged users
    let admin = access_control.get_role(&Role::Admin);
    let emergency_admin = access_control
        .get_role_safe(&Role::EmergencyAdmin)
        .unwrap_or(admin.clone());
    let rewards_admin = access_control
        .get_role_safe(&Role::RewardsAdmin)
        .unwrap_or(admin.clone());
    let operations_admin = access_control
        .get_role_safe(&Role::OperationsAdmin)
        .unwrap_or(admin.clone());
    let pause_admin = access_control
        .get_role_safe(&Role::PauseAdmin)
        .unwrap_or(admin.clone());
    let emergency_pause_admins = access_control.get_role_addresses(&Role::EmergencyPauseAdmin);

    let plane = get_pool_plane(e);

    let params = InitializeAllParams {
        config: PoolConfig {
            admin,
            privileged_addrs: PrivilegedAddresses {
                emergency_admin,
                rewards_admin,
                operations_admin,
                pause_admin,
                emergency_pause_admins,
            },
            router: e.current_contract_address(),
            oracle_registry: get_oracle_registry(e),
            insurance_fund: get_insurance_fund(e),
            assets: assets.clone(),
            token_b: token_b.clone(),
            token_a_sac_address: token_a_sac_address.clone(),
            share_token_info: TokenInitInfo {
                token_wasm_hash: share_token_wasm_hash.into_val(e),
                name: share_token_info.0.clone(),
                symbol: share_token_info.1.clone(),
            },
            fee_fraction,
            protocol_fee_fraction,
            status: PoolStatus::Initialized,
            tier: tier.clone(),
            max_insurance,
        },
        reward_config: RewardConfig { reward_token },
        plane,
    };

    e.invoke_contract::<()>(
        pool_contract_id,
        &Symbol::new(e, "initialize_all"),
        Vec::from_array(e, [params.into_val(e)]),
    );
}

pub fn get_total_liquidity(e: &Env, asset: Symbol, calculator: Address) -> U256 {
    let pool = get_pool(e, &asset);

    let pools_liquidity =
        LiquidityCalculatorClient::new(&e, &calculator).get_liquidity(&Vec::from_array(e, [pool]));

    pools_liquidity.get(0).unwrap()
}
