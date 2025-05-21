use crate::errors::LiquidityPoolRouterError;
use crate::events::{ Events, LiquidityPoolRouterEvents };
use crate::liquidity_calculator::LiquidityCalculatorClient;
use crate::rewards::get_rewards_manager;
use crate::storage::{
    add_pool,
    add_tokens_set,
    get_constant_product_pool_hash,
    get_pool_next_counter,
    get_pool_plane,
    get_pools_plain,
    get_token_hash,
    LiquidityPoolType,
};
use access_control::access::AccessControl;
use access_control::management::{ MultipleAddressesManagementTrait, SingleAddressManagementTrait };
use access_control::role::Role;
use rewards::storage::{ BoostFeedStorageTrait, BoostTokenStorageTrait, RewardTokenStorageTrait };
use sep_40_oracle::Asset;
use soroban_sdk::{ panic_with_error, String };
use soroban_sdk::{
    symbol_short,
    xdr::ToXdr,
    Address,
    Bytes,
    BytesN,
    Env,
    IntoVal,
    Map,
    Symbol,
    Val,
    Vec,
    U256,
};
use utils::storage::{
    InitializeAllParams,
    InitializeParams,
    OraclePair,
    PrivilegedAddresses,
    RewardConfig,
    TokenInitInfo,
};

pub fn get_standard_pool_salt(e: &Env, fee_fraction: &u32) -> BytesN<32> {
    let mut salt = Bytes::new(e);
    salt.append(&symbol_short!("standard").to_xdr(e));
    salt.append(&symbol_short!("0x00").to_xdr(e));
    salt.append(&fee_fraction.to_xdr(e));
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
    oracles: &OraclePair,
    target_asset: &Asset,
    lp_token_name: &String,
    lp_token_symbol: &String,
    fee_fraction: u32
) -> (BytesN<32>, Address) {
    let tokens_salt = get_tokens_salt(e, tokens);
    let liquidity_pool_wasm_hash = get_constant_product_pool_hash(e);
    let subpool_salt = get_standard_pool_salt(e, &fee_fraction);

    let pool_contract_id = e
        .deployer()
        .with_current_contract(
            merge_salt(
                e,
                merge_salt(e, tokens_salt.clone(), subpool_salt.clone()),
                get_pool_counter_salt(e)
            )
        )
        .deploy_v2(liquidity_pool_wasm_hash, ());
    init_standard_pool(
        e,
        tokens,
        oracles,
        target_asset,
        &pool_contract_id,
        lp_token_name,
        lp_token_symbol,
        fee_fraction
    );

    add_tokens_set(e, tokens);
    add_pool(
        e,
        tokens_salt,
        subpool_salt.clone(),
        LiquidityPoolType::ConstantProduct,
        pool_contract_id.clone()
    );

    Events::new(e).add_pool(
        tokens.clone(),
        pool_contract_id.clone(),
        symbol_short!("constant"),
        subpool_salt.clone(),
        Vec::<Val>::from_array(e, [fee_fraction.into_val(e)])
    );

    (subpool_salt, pool_contract_id)
}

fn init_standard_pool(
    e: &Env,
    tokens: &Vec<Address>,
    oracles: &OraclePair,
    target_asset: &Asset,
    pool_contract_id: &Address,
    lp_token_name: &String,
    lp_token_symbol: &String,
    fee_fraction: u32
) {
    let token_wasm_hash = get_token_hash(e);
    let rewards = get_rewards_manager(e);
    let reward_token = rewards.storage().get_reward_token();
    let reward_boost_token = rewards.storage().get_reward_boost_token();
    let reward_boost_feed = rewards.storage().get_reward_boost_feed();
    let access_control = AccessControl::new(e);

    // privileged users
    let admin = access_control.get_role(&Role::Admin);
    let emergency_admin = access_control
        .get_role_safe(&Role::EmergencyAdmin)
        .unwrap_or(admin.clone());
    let rewards_admin = access_control.get_role_safe(&Role::RewardsAdmin).unwrap_or(admin.clone());
    let operations_admin = access_control
        .get_role_safe(&Role::OperationsAdmin)
        .unwrap_or(admin.clone());
    let pause_admin = access_control.get_role_safe(&Role::PauseAdmin).unwrap_or(admin.clone());
    let emergency_pause_admins = access_control.get_role_addresses(&Role::EmergencyPauseAdmin);

    let plane = get_pool_plane(e);

    let params = InitializeAllParams {
        base: InitializeParams {
            admin,
            privileged_addrs: PrivilegedAddresses {
                emergency_admin,
                rewards_admin,
                operations_admin,
                pause_admin,
                emergency_pause_admins,
            },
            router: e.current_contract_address(),
            oracles: oracles.clone(),
            target_asset: target_asset.clone(),
            tokens: tokens.clone(),
            lp_token_info: TokenInitInfo {
                token_wasm_hash: token_wasm_hash.into_val(e),
                name: lp_token_name.clone(),
                symbol: lp_token_symbol.clone(),
            },
            fee_fraction,
        },
        reward_config: RewardConfig {
            reward_token,
            reward_boost_token,
            reward_boost_feed,
        },
        plane,
    };

    e.invoke_contract::<()>(
        pool_contract_id,
        &Symbol::new(e, "initialize_all"),
        Vec::from_array(e, [params.into_val(e)])
    );
}

pub fn assert_tokens_sorted(e: &Env, tokens: &Vec<Address>) {
    for i in 0..tokens.len() - 1 {
        let left = tokens.get_unchecked(i);
        let right = tokens.get_unchecked(i + 1);
        if left > right {
            panic_with_error!(e, LiquidityPoolRouterError::TokensNotSorted);
        }
        if left == right {
            panic_with_error!(e, LiquidityPoolRouterError::DuplicatesNotAllowed);
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
    calculator: Address
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
