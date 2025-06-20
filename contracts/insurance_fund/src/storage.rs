use paste::paste;
use soroban_sdk::token::TokenClient as SorobanTokenClient;
use soroban_sdk::{ contracttype, panic_with_error, Address, Env };
use utils::bump::{ bump_instance };
use utils::constant::THIRTEEN_DAY;
use utils::errors::storage_errors::StorageError;
use utils::{
    generate_instance_storage_getter_and_setter_with_default,
    generate_instance_storage_getter_with_default,
    generate_instance_storage_getter_and_setter,
    generate_instance_storage_setter,
    generate_instance_storage_getter,
};

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Token, // the token address of supported deposits.
    Router, // the address of the Pool Router.
    UnstakingPeriod, // a period of time stakers must wait once requesting withdrawal to actually withdraw.

    // insurance
    OptimalCoverage, // the maximum amount of insurance (in Token amount) to adequately insure the protocol.
    CoverageBuffer, // an optional cushion allowing more than the optimal coverage to be raised by LPs.

    // shares
    TotalShares, // the total amount of issued shares.
    SharesBase, // exponent for lp shares (for rebasing).

    // TODO: must we track the interest paid to avoid counting uncollected interest as coverage?

    // interest
    OptimalUtilization, // the optimal utilization point (utilization = vault amount / optimal coverage)
    BaseRate, // the base interest rate when utilization is 0%
    RateSlopeA, // the slope before hitting optimal utilization (gradual increase)
    RateSlopeB, // the slope after optimal utilization (steep increase)

    // paused ops
    IsKilledDeposit,
    IsKilledRequestWithdraw,
    IsKilledWithdraw,
}

// Config
generate_instance_storage_getter_and_setter!(token, DataKey::Token, Address);
generate_instance_storage_getter_and_setter!(router, DataKey::Router, Address);
generate_instance_storage_getter_and_setter_with_default!(
    unstaking_period,
    DataKey::UnstakingPeriod,
    u64,
    THIRTEEN_DAY
);

// Coverage
generate_instance_storage_getter_and_setter_with_default!(
    optimal_coverage,
    DataKey::OptimalCoverage,
    u128,
    0
);
generate_instance_storage_getter_and_setter_with_default!(
    coverage_buffer,
    DataKey::CoverageBuffer,
    u128,
    0
);

// Shares
generate_instance_storage_getter_and_setter_with_default!(
    total_shares,
    DataKey::TotalShares,
    u128,
    0
);
generate_instance_storage_getter_and_setter_with_default!(
    shares_base,
    DataKey::SharesBase,
    u128,
    0
);

// Interest
generate_instance_storage_getter_and_setter!(optimal_utilization, DataKey::OptimalUtilization, u32);
generate_instance_storage_getter_and_setter!(base_rate, DataKey::BaseRate, i32);
generate_instance_storage_getter_and_setter!(rate_slope_a, DataKey::RateSlopeA, u32);
generate_instance_storage_getter_and_setter!(rate_slope_b, DataKey::RateSlopeB, u32);

// Paused Ops
generate_instance_storage_getter_and_setter_with_default!(
    is_killed_deposit,
    DataKey::IsKilledDeposit,
    bool,
    false
);
generate_instance_storage_getter_and_setter_with_default!(
    is_killed_request_withdraw,
    DataKey::IsKilledRequestWithdraw,
    bool,
    false
);
generate_instance_storage_getter_and_setter_with_default!(
    is_killed_withdraw,
    DataKey::IsKilledWithdraw,
    bool,
    false
);

// Utils
pub fn get_insurance_vault_amount(e: &Env) -> u128 {
    SorobanTokenClient::new(e, &get_token(e)).balance(&e.current_contract_address()) as u128
}
