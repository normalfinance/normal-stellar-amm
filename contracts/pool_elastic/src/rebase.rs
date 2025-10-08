use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::{panic_with_error, Env, Symbol, Vec};

use crate::storage::{get_sink, put_reserve_a, put_reserve_b};

pub fn rebase(
    e: &Env,
    reserve_a: &u128,
    reserve_b: &u128,
    pool_price: u128,
    oracle_price: u128,
) -> (i128, i128) {
    (0, 0)

    // if pool_price > oracle_price {
    //     over(e, reserve_a, reserve_b)
    // } else {
    //     under(e, reserve_a, reserve_b)
    // }
}

//
pub fn over(e: &Env, reserve_a: u128, reserve_b: u128) -> (u128, u128) {
    // Calculate how much Token A we need to mint
    let token_a_to_mint = 0;

    (0, 0)

    // // Mint Token A
    // mint_synthetic_tokens(&e, &e.current_contract_address(), token_a_to_mint);

    // // Update Reserve A
    // put_reserve_a(&e, &(reserve_a + (token_a_to_mint as u128)));

    // // Calculate how much Token B to remove
    // let token_b_to_remove = 0;

    // match e.try_invoke_contract::<u32, soroban_sdk::Error>(
    //     &get_sink(e),
    //     &Symbol::new(e, "deposit"),
    //     Vec::from_array(e, [user.clone().into_val(e)]),
    // ) {
    //     Ok(Ok(deposit_amount)) => {
    //         // Update Reserve B
    //         put_reserve_b(&e, &(reserve_b - (deposit_amount as u128)));

    //         (token_a_to_mint, deposit_amount)
    //     }
    //     Ok(Err(_)) | Err(_) => {
    //         panic_with_error!(e);
    //     }
    // }
}

//
pub fn under(e: &Env, desired_a: u128) -> (u128, u128) {
    // Calculate how much Token A we need to burh
    let token_a_to_burn = 0;

    (0, 0)

    // // Burn Token A
    // burn_synthetic_tokens(&e, &e.current_contract_address(), token_a_to_burn);

    // // Update Reserve A
    // put_reserve_a(&e, &(reserve_a - (token_a_to_burn as u128)));

    // // Calculate how much Token B to deposit
    // let token_b_to_deposit = 0;

    // match e.try_invoke_contract::<u32, soroban_sdk::Error>(
    //     &get_sink(e),
    //     &Symbol::new(e, "withdraw"),
    //     Vec::from_array(e, [user.clone().into_val(e)]),
    // ) {
    //     Ok(Ok(deposit_amount)) => {
    //         // Update Reserve B
    //         put_reserve_b(&e, &(reserve_b + (deposit_amount as u128)));

    //         (token_a_to_mint, deposit_amount);
    //     }
    //     Ok(Err(_)) | Err(_) => {
    //         panic_with_error!(e, );
    //     }
    // }

    // // Update Reserve B
    // put_reserve_b(&e, &(reserve_b + (token_b_to_deposit as u128)));

    // (token_a_to_burn, token_b_to_deposit)
}

// legacy code

// // Computes the delta needed to re-peg reserve A (synthetic base token) to match the target peg price.
// //
// // Uses current reserves and oracle prices to calculate the ideal reserve A value,
// // then subtracts the actual reserve A to determine how much must be minted or burned.
// //
// // # Arguments
// // * `e` - Soroban environment reference.
// // * `base_oracle_price` - Oracle price of the base asset.
// // * `quote_oracle_price` - Oracle price of the quote asset.
// //
// // # Returns
// // * `i128` — The difference: `target_reserve_a - actual_reserve_a`.
// // Positive means mint, negative means burn.
// pub fn get_delta_a(
//     e: &Env,
//     reserve_a: u128,
//     reserve_b: u128,
//     base_oracle_price: u128,
//     quote_oracle_price: u128
// ) -> i128 {
//     let peg_price = peg_price(e, base_oracle_price, quote_oracle_price);

//     // Calculate target reserve with precision-aware smoothing
//     let target_reserve_a = calculate_target_reserve_with_smoothing(
//         e,
//         reserve_a,
//         reserve_b,
//         peg_price
//     );

//     // Safe conversion using our SafeConversion utilities
//     let target_reserve_a_i128 = target_reserve_a.safe_to_i128(e);
//     let reserve_a_i128 = reserve_a.safe_to_i128(e);

//     let delta_a_raw = target_reserve_a_i128.checked_sub(reserve_a_i128).unwrap_or_else(|| {
//         panic_with_error!(e, PoolError::ArithmeticOverflow);
//     });

//     // Apply per-ledger delta cap to prevent excessive rebalancing
//     let max_delta_per_ledger = reserve_a.safe_to_i128(e) / 20; // Max 5% change per operation
//     let delta_a = if delta_a_raw.abs() > max_delta_per_ledger {
//         if delta_a_raw > 0 { max_delta_per_ledger } else { -max_delta_per_ledger }
//     } else {
//         delta_a_raw
//     };

//     delta_a
// }

// // Calculates target reserve A with epsilon-based smoothing to prevent precision attacks.
// //
// // For very small relative changes in price (< 0.01%), treats delta_a as 0 to prevent
// // discontinuous jumps that could be exploited by precision attacks.
// //
// // # Arguments
// // * `e` - Soroban environment reference.
// // * `current_reserve_a` - Current reserve A amount.
// // * `reserve_b` - Current reserve B amount.
// // * `peg_price` - Current peg price.
// //
// // # Returns
// // * `u128` — The smoothed target reserve A amount.
// fn calculate_target_reserve_with_smoothing(
//     e: &Env,
//     current_reserve_a: u128,
//     reserve_b: u128,
//     peg_price: u128
// ) -> u128 {
//     // Use round-to-nearest to prevent accumulation bias
//     let raw_target_reserve_a = reserve_b.safe_fixed_div_round(e, peg_price, PRICE_PRECISION);

//     // Calculate relative change threshold (0.01% = 100 basis points)
//     let epsilon_threshold = current_reserve_a.safe_div(e, 10_000); // 0.01%

//     // If the change is smaller than epsilon, don't rebalance to prevent micro-adjustments
//     let delta_abs = if raw_target_reserve_a > current_reserve_a {
//         raw_target_reserve_a - current_reserve_a
//     } else {
//         current_reserve_a - raw_target_reserve_a
//     };

//     if delta_abs <= epsilon_threshold {
//         // Change is too small, maintain current reserve to prevent precision attacks
//         current_reserve_a
//     } else {
//         // Change is significant enough to warrant rebalancing
//         raw_target_reserve_a
//     }
// }
