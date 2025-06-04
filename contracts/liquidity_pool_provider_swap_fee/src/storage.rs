use paste::paste;
use soroban_sdk::{ contracttype, panic_with_error, Address, Env, Map };
use utils::bump::bump_instance;
use utils::errors::storage_errors::StorageError;
use utils::{
    generate_instance_storage_getter,
    generate_instance_storage_getter_and_setter,
    generate_instance_storage_setter,
};

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Router, // Address of the AMM router.
    Operator, // Address of the operator. Operator is capable to configure fees and claim them.
    InsuranceFund, // Address of the Insurance Fund.
    FeeDestination, // Fee destination address
    MaxSwapFeeFraction, // Max swap fee in basis points (100 = 1%)
    MinFeeInsuranceFraction, //
    UserFeeCheckpointA,
}

generate_instance_storage_getter_and_setter!(router, DataKey::Router, Address);
generate_instance_storage_getter_and_setter!(operator, DataKey::Operator, Address);
generate_instance_storage_getter_and_setter!(insurance_fund, DataKey::InsuranceFund, Address);
generate_instance_storage_getter_and_setter!(fee_destination, DataKey::FeeDestination, Address);
generate_instance_storage_getter_and_setter!(
    max_swap_fee_fraction,
    DataKey::MaxSwapFeeFraction,
    u32
);
generate_instance_storage_getter_and_setter!(
    min_fee_insurance_fraction,
    DataKey::MinFeeInsuranceFraction,
    u32
);
generate_instance_storage_getter_and_setter!(
    user_fee_checkpoint_a,
    DataKey::UserFeeCheckpointA,
    Map<Address, u128>
);
