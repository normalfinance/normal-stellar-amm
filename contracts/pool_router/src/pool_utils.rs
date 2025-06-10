use crate::errors::PoolRouterError;
use crate::events::{ Events, PoolRouterEvents };
use crate::incentives::get_incentives_manager;
use crate::storage::{
    add_pool,
    add_tokens_set,
    get_constant_product_pool_hash,
    get_pool_next_counter,
    get_token_hash,
    PoolType,
};
use access_control::access::AccessControl;
use access_control::management::{ MultipleAddressesManagementTrait, SingleAddressManagementTrait };
use access_control::role::Role;
use incentives::storage::{  RewardTokenStorageTrait };
use sep_40_oracle::Asset;
use soroban_sdk::token::Client as SorobanTokenClient;
use soroban_sdk::{ panic_with_error, String };
use soroban_sdk::{
    symbol_short,
    xdr::ToXdr,
    Address,
    Bytes,
    BytesN,
    Env,
    IntoVal,
    Symbol,
    Val,
    Vec,
};
use utils::storage::{
    InitializeAllParams, InitializeParams, PoolTier, PrivilegedAddresses, RewardConfig, TokenInitInfo
};

pub fn get_pool_salt(e: &Env, fee_fraction: &u32) -> BytesN<32> {
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

pub fn deploy_pool(
    e: &Env,
    tokens: &Vec<Address>,
    base_oracle_registry_id: &Symbol,
    quote_oracle_registry_id: &Symbol,
    asset: &Address,
    lp_token_name: &String,
    lp_token_symbol: &String,
    fee_fraction: u32,
    tier: &PoolTier,
    quote_max_insurance: u128,
    oracle_registry: &Address
) -> (BytesN<32>, Address) {
    let tokens_salt = get_tokens_salt(e, tokens);
    let pool_wasm_hash = get_constant_product_pool_hash(e);
    let subpool_salt = get_pool_salt(e, &fee_fraction);

    let pool_contract_id = e
        .deployer()
        .with_current_contract(
            merge_salt(
                e,
                merge_salt(e, tokens_salt.clone(), subpool_salt.clone()),
                get_pool_counter_salt(e)
            )
        )
        .deploy_v2(pool_wasm_hash, ());
    init_pool(
        e,
        tokens,
        base_oracle_registry_id,
        quote_oracle_registry_id,
        asset,
        &pool_contract_id,
        lp_token_name,
        lp_token_symbol,
        fee_fraction,
        tier,
        quote_max_insurance,
        oracle_registry
    );

    add_tokens_set(e, tokens);
    add_pool(
        e,
        tokens_salt,
        subpool_salt.clone(),
        PoolType::ConstantProduct,
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

fn init_pool(
    e: &Env,
    tokens: &Vec<Address>,
    base_oracle_registry_id: &Symbol,
    quote_oracle_registry_id: &Symbol,
    asset: &Address,
    pool_contract_id: &Address,
    lp_token_name: &String,
    lp_token_symbol: &String,
    fee_fraction: u32,
    tier: &PoolTier,
    quote_max_insurance: u128,
    oracle_registry: &Address
) {
    let token_wasm_hash = get_token_hash(e);
    let incentives = get_incentives_manager(e);
    let reward_token = incentives.storage().get_reward_token();
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
            base_asset_id: base_oracle_registry_id.clone(),
            quote_asset_id: quote_oracle_registry_id.clone(),
            asset: asset.clone(),
            tokens: tokens.clone(),
            lp_token_info: TokenInitInfo {
                token_wasm_hash: token_wasm_hash.into_val(e),
                name: lp_token_name.clone(),
                symbol: lp_token_symbol.clone(),
            },
            fee_fraction,
            tier: tier.clone(),
            quote_max_insurance,
            oracle_registry: oracle_registry.clone(),
        },
        reward_config: RewardConfig { reward_token },
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
