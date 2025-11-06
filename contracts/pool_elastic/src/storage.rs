use paste::paste;
use soroban_sdk::{contracttype, panic_with_error, Address, BytesN, Env, Symbol, Vec};
pub use utils::bump::bump_instance;
use utils::bump::bump_persistent;
use utils::errors::storage_errors::StorageError;
use utils::generate_instance_storage_getter;
use utils::state::oracle::{HistoricalOracleData, OracleGuardRails};
use utils::{
    generate_instance_storage_getter_and_setter,
    generate_instance_storage_getter_and_setter_with_default,
    generate_instance_storage_getter_with_default, generate_instance_storage_setter,
};

// Rate Table Entry for configurable tax/bonus tables
#[derive(Clone)]
#[contracttype]
pub struct RateTableEntry {
    pub deviation: u128, // Price deviation scaled by PRICE_PRECISION
    pub rate: u32,       // Tax/bonus rate fraction
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    TokenA,
    TokenB,
    BaseAsset, // symbol
    QuoteAsset,
    ReserveA,
    ReserveB,

    // Contract
    Plane,
    Router,
    Sink,

    // Oracle
    Oracle,
    OracleGuardRails, // a set of oracle price data validations and protections.
    HistoricalOracleData(Symbol),

    // Fee
    FeeFraction, // 1 = 0.01%
    FeeRebateFraction,
    ProtocolFeeFraction, // part of the fee that goes to the protocol, 5000 = 50% of the fee goes to the protocol
    ProtocolFeeA,
    ProtocolFeeB,

    // Tax
    BaseTaxFraction,
    TaxIncline, // steepness
    MaxTaxFraction,
    ProtocolTaxB,
    TaxRateTable, // Configurable table of deviation -> tax rate mappings

    // Bonus
    MaxBonusFraction,
    BonusVestingPeriod,
    BonusEscrow(Address),
    BonusReserveB,
    BonusRateTable, // Configurable table of deviation -> bonus rate mappings

    // Paused Ops
    IsKilledSwap,
    IsKilledDeposit,
    IsKilledClaim,
    IsKilledTax,
    IsKilledBonus,

    // Wasm
    TokenFutureWASM,
    GaugeFutureWASM,
}

// Reserve
generate_instance_storage_getter_and_setter_with_default!(reserve_a, DataKey::ReserveA, u128, 0);
generate_instance_storage_getter_and_setter_with_default!(reserve_b, DataKey::ReserveB, u128, 0);

// Paused Ops
generate_instance_storage_getter_and_setter_with_default!(
    is_killed_swap,
    DataKey::IsKilledSwap,
    bool,
    false
);
generate_instance_storage_getter_and_setter_with_default!(
    is_killed_deposit,
    DataKey::IsKilledDeposit,
    bool,
    false
);
generate_instance_storage_getter_and_setter_with_default!(
    is_killed_claim,
    DataKey::IsKilledClaim,
    bool,
    false
);
generate_instance_storage_getter_and_setter_with_default!(
    is_killed_tax,
    DataKey::IsKilledTax,
    bool,
    false
);
generate_instance_storage_getter_and_setter_with_default!(
    is_killed_bonus,
    DataKey::IsKilledBonus,
    bool,
    false
);

// Fee
generate_instance_storage_getter_and_setter_with_default!(
    fee_fraction,
    DataKey::FeeFraction,
    u32,
    30 // 0.30%
);
generate_instance_storage_getter_and_setter_with_default!(
    fee_rebate_fraction,
    DataKey::FeeRebateFraction,
    u32,
    10000 // 100% (zero rebate)
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

// Assets
generate_instance_storage_getter_and_setter!(base_asset, DataKey::BaseAsset, Symbol);
generate_instance_storage_getter_and_setter!(quote_asset, DataKey::QuoteAsset, Symbol);

generate_instance_storage_getter_and_setter_with_default!(
    oracle_guard_rails,
    DataKey::OracleGuardRails,
    OracleGuardRails,
    OracleGuardRails::default()
);

// Tax
generate_instance_storage_getter_and_setter_with_default!(
    base_tax_fraction,
    DataKey::BaseTaxFraction,
    u32,
    100 // 0.10%
);
generate_instance_storage_getter_and_setter_with_default!(
    tax_incline,
    DataKey::TaxIncline,
    u32,
    25
);
generate_instance_storage_getter_and_setter_with_default!(
    max_tax_fraction,
    DataKey::MaxTaxFraction,
    u32,
    50000 // 50%
);
generate_instance_storage_getter_and_setter_with_default!(
    protocol_tax_b,
    DataKey::ProtocolTaxB,
    u128,
    0
);

// Bonus
generate_instance_storage_getter_and_setter_with_default!(
    max_bonus_fraction,
    DataKey::MaxBonusFraction,
    u32,
    25000 // 25% (half of max tax)
);
generate_instance_storage_getter_and_setter_with_default!(
    bonus_vesting_period,
    DataKey::BonusVestingPeriod,
    u64,
    3600 // 1 hour in seconds
);
generate_instance_storage_getter_and_setter_with_default!(
    bonus_reserve_b,
    DataKey::BonusReserveB,
    u128,
    0
);

// Rate Tables
pub fn get_tax_rate_table(e: &Env) -> Vec<RateTableEntry> {
    bump_instance(e);
    e.storage()
        .instance()
        .get(&DataKey::TaxRateTable)
        .unwrap_or(Vec::new(e))
}

pub fn set_tax_rate_table(e: &Env, table: &Vec<RateTableEntry>) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::TaxRateTable, table);
}

pub fn get_bonus_rate_table(e: &Env) -> Vec<RateTableEntry> {
    bump_instance(e);
    e.storage()
        .instance()
        .get(&DataKey::BonusRateTable)
        .unwrap_or(Vec::new(e))
}

pub fn set_bonus_rate_table(e: &Env, table: &Vec<RateTableEntry>) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::BonusRateTable, table);
}

// Addresses
generate_instance_storage_getter_and_setter!(router, DataKey::Router, Address);
generate_instance_storage_getter_and_setter!(plane, DataKey::Plane, Address);
generate_instance_storage_getter_and_setter!(oracle, DataKey::Oracle, Address);
generate_instance_storage_getter_and_setter!(
    token_future_wasm,
    DataKey::TokenFutureWASM,
    BytesN<32>
);
generate_instance_storage_getter_and_setter!(
    gauge_future_wasm,
    DataKey::GaugeFutureWASM,
    BytesN<32>
);

pub fn get_token_a(e: &Env) -> Address {
    bump_instance(e);
    match e.storage().instance().get(&DataKey::TokenA) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub fn get_token_b(e: &Env) -> Address {
    bump_instance(e);
    match e.storage().instance().get(&DataKey::TokenB) {
        Some(v) => v,
        None => panic_with_error!(e, StorageError::ValueNotInitialized),
    }
}

pub fn put_token_a(e: &Env, contract: Address) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::TokenA, &contract)
}

pub fn put_token_b(e: &Env, contract: Address) {
    bump_instance(e);
    e.storage().instance().set(&DataKey::TokenB, &contract)
}

pub(crate) fn has_plane(e: &Env) -> bool {
    let key = DataKey::Plane;
    e.storage().instance().has(&key)
}

// Historical Oracle Data

pub(crate) fn get_historical_oracle_data(e: &Env, asset: &Symbol) -> HistoricalOracleData {
    let key = DataKey::HistoricalOracleData(asset.clone());
    match e.storage().persistent().get(&key) {
        Some(value) => {
            bump_persistent(e, &key);
            value
        }
        None => HistoricalOracleData::default_quote_oracle(),
    }
}

pub(crate) fn put_historical_oracle_data(
    e: &Env,
    asset: &Symbol,
    oracle_data: &HistoricalOracleData,
) {
    let key = DataKey::HistoricalOracleData(asset.clone());
    e.storage().persistent().set(&key, oracle_data);
    bump_persistent(e, &key);
}

// Bonus Escrow

#[derive(Clone)]
#[contracttype]
pub struct BonusEscrow {
    pub amount: u128,        // bonus amount in token units
    pub updated_at: u64,     // ledger timestamp when recorded
    pub valid_after: u64,    // timestamp when bonus becomes claimable
}

impl BonusEscrow {
    pub fn new() -> Self {
        BonusEscrow {
            amount: 0,
            updated_at: 0,
            valid_after: 0,
        }
    }
}

pub(crate) fn get_bonus_escrow(e: &Env, user: &Address) -> BonusEscrow {
    let key = DataKey::BonusEscrow(user.clone());
    match e.storage().persistent().get(&key) {
        Some(value) => {
            bump_persistent(e, &key);
            value
        }
        None => BonusEscrow::new(),
    }
}

pub(crate) fn put_bonus_escrow(e: &Env, user: &Address, bonus_escrow: &BonusEscrow) {
    let key = DataKey::BonusEscrow(user.clone());
    e.storage().persistent().set(&key, bonus_escrow);
    bump_persistent(e, &key);
}

pub(crate) fn delete_bonus_escrow(e: &Env, user: &Address) {
    let key = DataKey::BonusEscrow(user.clone());
    e.storage().persistent().remove(&key);
}
