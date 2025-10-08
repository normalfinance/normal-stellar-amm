use crate::errors::PoolRouterError;
use crate::events::{Events, PoolRouterEvents};
use crate::liquidity_calculator::LiquidityCalculatorClient;
use crate::rewards::get_rewards_manager;
use crate::storage::{
    add_pool, add_tokens_set, get_constant_product_pool_hash, get_elastic_pool_hash,
    get_pool_next_counter, get_pool_plane, get_pools_plain, get_protocol_fee_fraction,
    get_token_hash, LiquidityPoolType,
};
use access_control::access::AccessControl;
use access_control::management::{MultipleAddressesManagementTrait, SingleAddressManagementTrait};
use access_control::role::Role;
use pool_config_storage::operations::get_config_storage;
use rewards::storage::RewardTokenStorageTrait;
use soroban_sdk::token::Client as SorobanTokenClient;
use soroban_sdk::{
    panic_with_error, symbol_short, xdr::ToXdr, Address, Bytes, BytesN, Env, IntoVal, Map, Symbol,
    Val, Vec, U256,
};

pub fn get_standard_pool_salt(e: &Env, fee_fraction: &u32) -> BytesN<32> {
    let mut salt = Bytes::new(e);
    salt.append(&symbol_short!("standard").to_xdr(e));
    salt.append(&symbol_short!("0x00").to_xdr(e));
    salt.append(&fee_fraction.to_xdr(e));
    salt.append(&symbol_short!("0x00").to_xdr(e));
    e.crypto().sha256(&salt).to_bytes()
}

pub fn get_elastic_pool_salt(e: &Env) -> BytesN<32> {
    let mut salt = Bytes::new(e);
    salt.append(&symbol_short!("elastic").to_xdr(e));
    salt.append(&symbol_short!("0x00").to_xdr(e));
    // no constant pool parameters, though hash should be different, so we add pool counter
    salt.append(&get_pool_next_counter(e).to_xdr(e));
    salt.append(&symbol_short!("0x00").to_xdr(e));
    e.crypto().sha256(&salt).to_bytes()
}

pub fn get_pool_counter_salt(e: &Env) -> BytesN<32> {
    let mut salt = Bytes::new(e);
    salt.append(&symbol_short!("0x00").to_xdr(e));
    salt.append(&get_pool_next_counter(e).to_xdr(e));
    salt.append(&symbol_short!("0x00").to_xdr(e));
    e.crypto().sha256(&salt).to_bytes()
}

pub fn merge_salt(e: &Env, left: BytesN<32>, right: BytesN<32>) -> BytesN<32> {
    let mut salt = Bytes::new(e);
    salt.append(&left.to_xdr(e));
    salt.append(&right.to_xdr(e));
    e.crypto().sha256(&salt).to_bytes()
}

pub fn deploy_standard_pool(
    e: &Env,
    tokens: &Vec<Address>,
    fee_fraction: u32,
) -> (BytesN<32>, Address) {
    let tokens_salt = get_tokens_salt(e, tokens);
    let liquidity_pool_wasm_hash = get_constant_product_pool_hash(e);
    let subpool_salt = get_standard_pool_salt(e, &fee_fraction);

    let pool_contract_id = e
        .deployer()
        .with_current_contract(merge_salt(
            e,
            merge_salt(e, tokens_salt.clone(), subpool_salt.clone()),
            get_pool_counter_salt(e),
        ))
        .deploy_v2(liquidity_pool_wasm_hash, ());
    init_standard_pool(e, tokens, &pool_contract_id, fee_fraction);

    add_tokens_set(e, tokens);
    add_pool(
        e,
        tokens_salt,
        subpool_salt.clone(),
        LiquidityPoolType::ConstantProduct,
        pool_contract_id.clone(),
    );

    Events::new(e).add_pool(
        tokens.clone(),
        pool_contract_id.clone(),
        symbol_short!("constant"),
        subpool_salt.clone(),
        Vec::<Val>::from_array(e, [fee_fraction.into_val(e)]),
    );

    (subpool_salt, pool_contract_id)
}

pub fn deploy_elastic_pool(
    e: &Env,
    tokens: &Vec<Address>,
    fee_fraction: u32,
    oracle: &Address,
) -> (BytesN<32>, Address) {
    let tokens_salt = get_tokens_salt(e, tokens);

    let liquidity_pool_wasm_hash = get_elastic_pool_hash(e);
    let subpool_salt = get_elastic_pool_salt(e);

    // pools counter already incorporated into subpool_salt - no need to add it again
    let pool_contract_id = e
        .deployer()
        .with_current_contract(merge_salt(e, tokens_salt.clone(), subpool_salt.clone()))
        .deploy_v2(liquidity_pool_wasm_hash, ());
    init_elastic_pool(e, tokens, &pool_contract_id, fee_fraction, oracle);

    add_tokens_set(e, tokens);
    add_pool(
        e,
        tokens_salt,
        subpool_salt.clone(),
        LiquidityPoolType::ElasticSupply,
        pool_contract_id.clone(),
    );

    Events::new(e).add_pool(
        tokens.clone(),
        pool_contract_id.clone(),
        symbol_short!("elastic"),
        subpool_salt.clone(),
        Vec::<Val>::from_array(e, [fee_fraction.into_val(e)]),
    );

    (subpool_salt, pool_contract_id)
}

fn init_standard_pool(
    e: &Env,
    tokens: &Vec<Address>,
    pool_contract_id: &Address,
    fee_fraction: u32,
) {
    let token_wasm_hash = get_token_hash(e);
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
    let system_fee_admin = access_control
        .get_role_safe(&Role::SystemFeeAdmin)
        .unwrap_or(admin.clone());
    let emergency_pause_admins = access_control.get_role_addresses(&Role::EmergencyPauseAdmin);

    let plane = get_pool_plane(e);
    let storage_config = get_config_storage(e);

    let protocol_fee_fraction = get_protocol_fee_fraction(&e);

    e.invoke_contract::<()>(
        pool_contract_id,
        &Symbol::new(e, "initialize_all"),
        Vec::from_array(
            e,
            [
                admin.into_val(e),
                (
                    emergency_admin,
                    rewards_admin,
                    operations_admin,
                    pause_admin,
                    emergency_pause_admins,
                    system_fee_admin,
                )
                    .into_val(e),
                e.current_contract_address().to_val(),
                token_wasm_hash.into_val(e),
                tokens.clone().into_val(e),
                (
                    <u32 as IntoVal<Env, u32>>::into_val(&fee_fraction, e),
                    <u32 as IntoVal<Env, u32>>::into_val(&protocol_fee_fraction, e),
                )
                    .into_val(e),
                reward_token.to_val().into_val(e),
                plane.into_val(e),
                storage_config.into_val(e),
            ],
        ),
    );
}

fn init_elastic_pool(
    e: &Env,
    tokens: &Vec<Address>,
    pool_contract_id: &Address,
    fee_fraction: u32,
    oracle: &Address,
) {
    let token_wasm_hash = get_token_hash(e);
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
    let system_fee_admin = access_control
        .get_role_safe(&Role::SystemFeeAdmin)
        .unwrap_or(admin.clone());
    let emergency_pause_admins = access_control.get_role_addresses(&Role::EmergencyPauseAdmin);

    let plane = get_pool_plane(e);
    let storage_config = get_config_storage(e);

    let protocol_fee_fraction = get_protocol_fee_fraction(&e);

    e.invoke_contract::<()>(
        pool_contract_id,
        &Symbol::new(e, "initialize_all"),
        Vec::from_array(
            e,
            [
                admin.into_val(e),
                (
                    emergency_admin,
                    rewards_admin,
                    operations_admin,
                    pause_admin,
                    emergency_pause_admins,
                    system_fee_admin,
                )
                    .into_val(e),
                e.current_contract_address().to_val(),
                oracle.to_val(),
                token_wasm_hash.into_val(e),
                tokens.clone().into_val(e),
                (
                    <u32 as IntoVal<Env, u32>>::into_val(&fee_fraction, e),
                    <u32 as IntoVal<Env, u32>>::into_val(&protocol_fee_fraction, e),
                )
                    .into_val(e),
                reward_token.to_val().into_val(e),
                plane.into_val(e),
                storage_config.into_val(e),
            ],
        ),
    );
}

pub fn assert_tokens_sorted(e: &Env, tokens: &Vec<Address>) {
    for i in 0..tokens.len() - 1 {
        let left = tokens.get_unchecked(i);
        let right = tokens.get_unchecked(i + 1);
        if left > right {
            panic_with_error!(e, PoolRouterError::TokensNotSorted);
        }
        if left == right {
            panic_with_error!(e, PoolRouterError::DuplicatesNotAllowed);
        }
    }
}

pub fn get_tokens_salt(e: &Env, tokens: &Vec<Address>) -> BytesN<32> {
    let mut salt = Bytes::new(e);
    for token in tokens.iter() {
        salt.append(&token.to_xdr(e));
    }
    e.crypto().sha256(&salt).to_bytes()
}

pub fn validate_tokens_contracts(e: &Env, tokens: &Vec<Address>) {
    // call token contract to check if token exists & it's alive
    for token in tokens.iter() {
        SorobanTokenClient::new(e, &token).balance(&e.current_contract_address());
    }
}

pub fn get_total_liquidity(
    e: &Env,
    tokens: &Vec<Address>,
    calculator: Address,
) -> (Map<BytesN<32>, U256>, U256) {
    let tokens_salt = get_tokens_salt(e, tokens);
    let pools = get_pools_plain(&e, tokens_salt);
    let pools_count = pools.len();
    let mut pools_map: Map<BytesN<32>, U256> = Map::new(&e);

    let mut pools_vec: Vec<Address> = Vec::new(&e);
    let mut hashes_vec: Vec<BytesN<32>> = Vec::new(&e);
    for (key, value) in pools {
        pools_vec.push_back(value.clone());
        hashes_vec.push_back(key.clone());
    }

    let pools_liquidity = LiquidityCalculatorClient::new(&e, &calculator).get_liquidity(&pools_vec);
    let mut result = U256::from_u32(&e, 0);
    for i in 0..pools_count {
        let value = pools_liquidity.get(i).unwrap();
        pools_map.set(hashes_vec.get(i).unwrap(), value.clone());
        result = result.add(&value);
    }
    (pools_map, result)
}

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
// fn get_pool_salt(e: &Env, asset: &Symbol) -> BytesN<32> {
//     let mut salt = Bytes::new(e);
//     salt.append(&symbol_short!("0x00").to_xdr(e));
//     salt.append(&asset.to_xdr(e));
//     salt.append(&symbol_short!("0x00").to_xdr(e));
//     e.crypto().sha256(&salt).to_bytes()
// }

// pub fn deploy_pool(
//     e: &Env,
//     token_b: &Address,
//     assets: &(Symbol, Symbol),
//     synthetic_sac_address: &Address,
//     lp_token_info: &(String, String),
//     fee_fraction: u32,
//     tier: &PoolTier,
//     quote_max_insurance: u128,
// ) -> Address {
//     let pool_wasm_hash = get_pool_hash(e);
//     let (base_asset, _) = assets;

//     let pool_contract_id = e
//         .deployer()
//         .with_current_contract(get_pool_salt(e, &base_asset))
//         .deploy_v2(pool_wasm_hash, ());

//     init_pool(
//         e,
//         token_b,
//         assets,
//         &pool_contract_id,
//         synthetic_sac_address,
//         lp_token_info,
//         fee_fraction,
//         tier,
//         quote_max_insurance,
//     );

//     // Add pool contract address to Map<Symbol, Address>
//     put_pool(e, base_asset.clone(), &pool_contract_id.clone());

//     // Add pool contract address to Vec<Address>
//     let mut pools_vec = get_pools_vec(&e);
//     pools_vec.push_back(pool_contract_id.clone());
//     set_pools_vec(&e, &pools_vec);

//     Events::new(e).add_pool(
//         base_asset.clone(),
//         token_b.clone(),
//         pool_contract_id.clone(),
//         Vec::<Val>::from_array(
//             e,
//             [
//                 fee_fraction.into_val(e),
//                 tier.into_val(e),
//                 quote_max_insurance.into_val(e),
//             ],
//         ),
//     );

//     pool_contract_id
// }

// fn init_pool(
//     e: &Env,
//     token_b: &Address,
//     assets: &(Symbol, Symbol),
//     pool_contract_id: &Address,
//     synthetic_sac_address: &Address,
//     lp_token_info: &(String, String),
//     fee_fraction: u32,
//     tier: &PoolTier,
//     quote_max_insurance: u128,
// ) {
//     let lp_token_wasm_hash = get_lp_token_hash(e);
//     let incentives = get_incentives_manager(e);
//     let reward_token = incentives.storage().get_reward_token();
//     let access_control = AccessControl::new(e);

//     // privileged users
//     let admin = access_control.get_role(&Role::Admin);
//     let emergency_admin = access_control
//         .get_role_safe(&Role::EmergencyAdmin)
//         .unwrap_or(admin.clone());
//     let rewards_admin = access_control
//         .get_role_safe(&Role::RewardsAdmin)
//         .unwrap_or(admin.clone());
//     let operations_admin = access_control
//         .get_role_safe(&Role::OperationsAdmin)
//         .unwrap_or(admin.clone());
//     let pause_admin = access_control
//         .get_role_safe(&Role::PauseAdmin)
//         .unwrap_or(admin.clone());
//     let emergency_pause_admins = access_control.get_role_addresses(&Role::EmergencyPauseAdmin);

//     let plane = get_pool_plane(e);

//     let params = InitializeAllParams {
//         base: InitializeParams {
//             admin,
//             privileged_addrs: PrivilegedAddresses {
//                 emergency_admin,
//                 rewards_admin,
//                 operations_admin,
//                 pause_admin,
//                 emergency_pause_admins,
//             },
//             router: e.current_contract_address(),
//             oracle_registry: get_oracle_registry(e),
//             assets: assets.clone(),
//             token_b: token_b.clone(),
//             synthetic_sac_address: synthetic_sac_address.clone(),
//             lp_token_info: TokenInitInfo {
//                 token_wasm_hash: lp_token_wasm_hash.into_val(e),
//                 name: lp_token_info.0.clone(),
//                 symbol: lp_token_info.1.clone(),
//             },
//             fee_fraction,
//             tier: tier.clone(),
//             quote_max_insurance,
//         },
//         reward_config: RewardConfig { reward_token },
//         plane,
//     };

//     e.invoke_contract::<()>(
//         pool_contract_id,
//         &Symbol::new(e, "initialize_all"),
//         Vec::from_array(e, [params.into_val(e)]),
//     );
// }
