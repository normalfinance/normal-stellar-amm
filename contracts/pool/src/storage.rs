use paste::paste;
use soroban_sdk::{contracttype, panic_with_error, Address, BytesN, Env, Symbol, Vec};
pub use utils::bump::bump_instance;
use utils::errors::storage_errors::StorageError;
use utils::state::pool::{InsuranceClaim, PoolStatus, PoolTier};
use utils::{
    generate_instance_storage_getter, generate_instance_storage_getter_and_setter,
    generate_instance_storage_getter_and_setter_with_default,
    generate_instance_storage_getter_with_default, generate_instance_storage_setter,
};

#[derive(Clone)]
#[contracttype]
enum DataKey {
    TokenA,
    TotalSynthetics,
    TokenB,     // the quote token address (always XLM).
    ReserveA,   // total token_a amount in the pool (x in the constant product formula)
    ReserveB,   // total token_b amount in the pool (y in the constant product formula)
    BaseAsset,  // the Symbol of the base (synthetic) asset (i.e. nBTC).
    QuoteAsset, // the Symbol of the quote asset (TokenB).
    Tier,
    Status,

    Plane,          // the address of the pool plane.
    Router,         // the Pool Router contract address
    OracleRegistry, // the Oracle Registry contract address
    InsuranceFund,
    TokenInsurance,

    MintCapFraction, // a bps cap on how much token_a can be minted when the pool is in reduce only mode

    // fees
    FeeFraction,         // 1 = 0.01%
    ProtocolFeeFraction, // part of the fee that goes to the protocol, 5000 = 50% of the fee goes to the protocol
    ProtocolFeeA,
    ProtocolFeeB,

    // insurance
    InsuranceClaim, // the pool's claim on the insurance fund.
    // The max liquidity imbalance before price premiums are added and/or the Insurance Fund is used
    // liquidity imbalance is the difference between quote token and base token value. When it's less than 0,
    // the pool does not have enough liquidity to fill all orders and will apply a price premium to new swaps.
    MinCollateralFraction,

    // metrics
    LastTradeTs,     // the blockchain unix timestamp at the time of the last trade
    Volume30d,       // estimated total of volume in market
    RebalanceMinted, // The outstanding number of token_a minted by the Pool to maintain price peg

    TokenFutureWASM,
}

// Numbers
generate_instance_storage_getter_and_setter_with_default!(reserve_a, DataKey::ReserveA, u128, 0);
generate_instance_storage_getter_and_setter_with_default!(reserve_b, DataKey::ReserveB, u128, 0);
generate_instance_storage_getter_and_setter_with_default!(
    min_collateral_fraction,
    DataKey::MinCollateralFraction,
    u32,
    7000 // 70%
);
generate_instance_storage_getter_and_setter_with_default!(
    total_synthetics,
    DataKey::TotalSynthetics,
    u128,
    0
);

// Assets
generate_instance_storage_getter_and_setter!(base_asset, DataKey::BaseAsset, Symbol);
generate_instance_storage_getter_and_setter_with_default!(
    quote_asset,
    DataKey::QuoteAsset,
    Symbol,
    Symbol::new(&Env::default(), "XLM")
);

// Other
generate_instance_storage_getter_and_setter_with_default!(
    tier,
    DataKey::Tier,
    PoolTier,
    PoolTier::A
);
generate_instance_storage_getter_and_setter_with_default!(
    status,
    DataKey::Status,
    PoolStatus,
    PoolStatus::Initialized
);
generate_instance_storage_getter_and_setter_with_default!(
    insurance_claim,
    DataKey::InsuranceClaim,
    InsuranceClaim,
    InsuranceClaim::default()
);

// Addresses
generate_instance_storage_getter_and_setter!(token_a, DataKey::TokenA, Address);
generate_instance_storage_getter_and_setter!(token_b, DataKey::TokenB, Address);
generate_instance_storage_getter_and_setter!(plane, DataKey::Plane, Address);
generate_instance_storage_getter_and_setter!(router, DataKey::Router, Address);
generate_instance_storage_getter_and_setter!(oracle_registry, DataKey::OracleRegistry, Address);
generate_instance_storage_getter_and_setter!(insurance_fund, DataKey::InsuranceFund, Address);
generate_instance_storage_getter_and_setter!(token_insurance, DataKey::TokenInsurance, Address);

// Fees
generate_instance_storage_getter_and_setter_with_default!(
    fee_fraction,
    DataKey::FeeFraction,
    u32,
    0
);
generate_instance_storage_getter_and_setter_with_default!(
    protocol_fee_fraction,
    DataKey::ProtocolFeeFraction,
    u32,
    0
);
generate_instance_storage_getter_and_setter_with_default!(
    protocol_fee_a,
    DataKey::ProtocolFeeA,
    u128,
    0
);
generate_instance_storage_getter_and_setter_with_default!(
    protocol_fee_b,
    DataKey::ProtocolFeeB,
    u128,
    0
);

// Metrics
generate_instance_storage_getter_and_setter_with_default!(volume_30d, DataKey::Volume30d, u128, 0);
generate_instance_storage_getter_and_setter_with_default!(
    last_trade_ts,
    DataKey::LastTradeTs,
    u64,
    0
);
generate_instance_storage_getter_and_setter_with_default!(
    rebalance_minted,
    DataKey::RebalanceMinted,
    u128,
    0
);
generate_instance_storage_getter_and_setter_with_default!(
    mint_cap_fraction,
    DataKey::MintCapFraction,
    u32,
    10 // 0.1%
);

pub(crate) fn has_plane(e: &Env) -> bool {
    let key = DataKey::Plane;
    e.storage().instance().has(&key)
}

pub(crate) fn set_token_future_wasm(e: &Env, value: &BytesN<32>) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::TokenFutureWASM, value)
}

pub(crate) fn get_token_future_wasm(e: &Env) -> BytesN<32> {
    bump_instance(e);
    match e.storage().instance().get(&DataKey::TokenFutureWASM) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}
