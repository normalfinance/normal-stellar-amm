use paste::paste;
use soroban_sdk::{contracttype, panic_with_error, Address, Env};
use utils::bump::bump_instance;
use normal_rust_types::StorageError;
use utils::{
    generate_instance_storage_getter, generate_instance_storage_getter_and_setter,
    generate_instance_storage_getter_and_setter_with_default,
    generate_instance_storage_getter_with_default, generate_instance_storage_setter,
};

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Router,            // Address of the Pool Router.
    InsuranceFund,     // Address of the Insurance Fund.
    FeeDestination,    // Fee destination address
    LPRevenueFraction, // The portion of fees to give back to LPs as yield in basis points (100 = 1%)

    // metrics
    LastTradeTs, // the timestamp of the last swap.
    Volume24h,   // rolling total swap volume over the past 24 hours.
    Volume7d,    // rolling total swap volume over the pst 7 days.
    Volume30d, // rolling total swap volume over the pst 30 days (used to estimate insurance premium).
}

generate_instance_storage_getter_and_setter!(router, DataKey::Router, Address);
generate_instance_storage_getter_and_setter!(insurance_fund, DataKey::InsuranceFund, Address);
generate_instance_storage_getter_and_setter!(fee_destination, DataKey::FeeDestination, Address);
generate_instance_storage_getter_and_setter_with_default!(
    lp_revenue_fraction,
    DataKey::LPRevenueFraction,
    u32,
    5000 // 50%
);
generate_instance_storage_getter_and_setter_with_default!(
    last_trade_ts,
    DataKey::LastTradeTs,
    u64,
    0
);
generate_instance_storage_getter_and_setter_with_default!(volume_24h, DataKey::Volume24h, u128, 0);
generate_instance_storage_getter_and_setter_with_default!(volume_7d, DataKey::Volume7d, u128, 0);
generate_instance_storage_getter_and_setter_with_default!(volume_30d, DataKey::Volume30d, u128, 0);
