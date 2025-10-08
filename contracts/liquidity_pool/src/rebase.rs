use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::{ panic_with_error, Env };

pub fn rebase(e: &Env, reserve_a: &u128, reserve_b: &u128) -> (i128, i128) {
    if token_a_pool_price > token_a_oracle_price {
        over(e, reserve_a, reserve_b);
    } else {
        under(e, reserve_a, reserve_b);
    }
}

//
pub fn over(e: &Env, reserve_a: u128, reserve_b: u128) -> (u128, u128) {
    // Calculate how much Token A we need to mint
    let token_a_to_mint = 0;

    // Mint Token A
    mint_synthetic_tokens(&e, &e.current_contract_address(), token_a_to_mint);

    // Update Reserve A
    set_reserve_a(&e, &(reserve_a + (token_a_to_mint as u128)));

    // Calculate how much Token B to remove
    let token_b_to_remove = 0;

    match
        e.try_invoke_contract::<u32, soroban_sdk::Error>(
            &get_sink_address(e),
            &Symbol::new(e, "deposit"),
            Vec::from_array(e, [user.clone().into_val(e)])
        )
    {
        Ok(Ok(deposit_amount)) => {
            // Update Reserve B
            set_reserve_b(&e, &(reserve_b - (deposit_amount as u128)));

            (token_a_to_mint, deposit_amount)
        }
        Ok(Err(_)) | Err(_) => {
            panic_with_error();
        }
    }
}

//
pub fn under(e: &Env, desired_a: u128) -> (u128, u128) {
    // Calculate how much Token A we need to burh
    let token_a_to_burn = 0;

    // Burn Token A
    burn_synthetic_tokens(&e, &e.current_contract_address(), token_a_to_burn);

    // Update Reserve A
    set_reserve_a(&e, &(reserve_a - (token_a_to_burn as u128)));

    // Calculate how much Token B to deposit
    let token_b_to_deposit = 0;

    match
        e.try_invoke_contract::<u32, soroban_sdk::Error>(
            &get_sink_address(e),
            &Symbol::new(e, "withdraw"),
            Vec::from_array(e, [user.clone().into_val(e)])
        )
    {
        Ok(Ok(deposit_amount)) => {
            // Update Reserve B
            set_reserve_b(&e, &(reserve_b + (deposit_amount as u128)));

            (token_a_to_mint, deposit_amount);
        }
        Ok(Err(_)) | Err(_) => {
            panic_with_error();
        }
    }

    // Update Reserve B
    set_reserve_b(&e, &(reserve_b + (token_b_to_deposit as u128)));

    (token_a_to_burn, token_b_to_deposit)
}



// legacy acode



// Computes the delta needed to re-peg reserve A (synthetic base token) to match the target peg price.
//
// Uses current reserves and oracle prices to calculate the ideal reserve A value,
// then subtracts the actual reserve A to determine how much must be minted or burned.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `base_oracle_price` - Oracle price of the base asset.
// * `quote_oracle_price` - Oracle price of the quote asset.
//
// # Returns
// * `i128` — The difference: `target_reserve_a - actual_reserve_a`.
// Positive means mint, negative means burn.
pub fn get_delta_a(
    e: &Env,
    reserve_a: u128,
    reserve_b: u128,
    base_oracle_price: u128,
    quote_oracle_price: u128,
) -> i128 {
    let peg_price = peg_price(e, base_oracle_price, quote_oracle_price);
    
    // Calculate target reserve with precision-aware smoothing
    let target_reserve_a = calculate_target_reserve_with_smoothing(
        e, 
        reserve_a,
        reserve_b, 
        peg_price
    );
    
    // Safe conversion using our SafeConversion utilities
    let target_reserve_a_i128 = target_reserve_a.safe_to_i128(e);
    let reserve_a_i128 = reserve_a.safe_to_i128(e);
    
    let delta_a_raw = target_reserve_a_i128
        .checked_sub(reserve_a_i128)
        .unwrap_or_else(|| {
            panic_with_error!(e, PoolError::ArithmeticOverflow);
        });
    
    // Apply per-ledger delta cap to prevent excessive rebalancing
    let max_delta_per_ledger = reserve_a.safe_to_i128(e) / 20; // Max 5% change per operation
    let delta_a = if delta_a_raw.abs() > max_delta_per_ledger {
        if delta_a_raw > 0 {
            max_delta_per_ledger
        } else {
            -max_delta_per_ledger
        }
    } else {
        delta_a_raw
    };

    delta_a
}

// Calculates target reserve A with epsilon-based smoothing to prevent precision attacks.
//
// For very small relative changes in price (< 0.01%), treats delta_a as 0 to prevent
// discontinuous jumps that could be exploited by precision attacks.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `current_reserve_a` - Current reserve A amount.
// * `reserve_b` - Current reserve B amount.
// * `peg_price` - Current peg price.
//
// # Returns
// * `u128` — The smoothed target reserve A amount.
fn calculate_target_reserve_with_smoothing(
    e: &Env,
    current_reserve_a: u128,
    reserve_b: u128,
    peg_price: u128,
) -> u128 {
    // Use round-to-nearest to prevent accumulation bias
    let raw_target_reserve_a = reserve_b.safe_fixed_div_round(e, peg_price, PRICE_PRECISION);
    
    // Calculate relative change threshold (0.01% = 100 basis points)
    let epsilon_threshold = current_reserve_a.safe_div(e, 10_000); // 0.01%
    
    // If the change is smaller than epsilon, don't rebalance to prevent micro-adjustments
    let delta_abs = if raw_target_reserve_a > current_reserve_a {
        raw_target_reserve_a - current_reserve_a
    } else {
        current_reserve_a - raw_target_reserve_a
    };
    
    if delta_abs <= epsilon_threshold {
        // Change is too small, maintain current reserve to prevent precision attacks
        current_reserve_a
    } else {
        // Change is significant enough to warrant rebalancing
        raw_target_reserve_a
    }
}

// Mints or burns synthetic tokens (reserve A) to restore the peg between base and quote assets.
//
// Uses oracle prices to calculate the required change in synthetic token supply to match
// the peg. Adjusts the pool's reserve A accordingly and emits a rebalance event.
// In ReduceOnly mode, prevents minting new synthetic tokens to avoid increasing synthetic asset exposure.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `base_oracle_price` - Oracle price of the synthetic base asset.
// * `quote_oracle_price` - Oracle price of the quote asset.
// * `pool_status` - Current pool status to determine if minting should be restricted.
pub fn rebalance(e: &Env, base_oracle_price: u128, quote_oracle_price: u128, reduce_only: bool) {
    let (reserve_a, reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

    // Find the ideal reserve_a amount such that the pool's price is equal to the oracle price
    let delta_a = get_delta_a(
        &e,
        reserve_a,
        reserve_b,
        base_oracle_price,
        quote_oracle_price,
    );
    if delta_a != 0 {
        if delta_a > 0 {
            if reduce_only {
                LiquidityPoolEvents::new(&e).capped_mint(
                    base_oracle_price,
                    quote_oracle_price,
                    delta_a,
                );

                // allow minting up to 0.1 % of current supply per ledger
                // Use safe arithmetic and conversions
                let total_supply = get_total_synthetic_tokens(&e);
                let mint_cap_fraction_u32 = get_mint_cap_fraction(&e);
                // Safe conversion from u32 to u128 (always safe as u32 fits in u128)
                // Can still consider using safe conversion here for consistency
                let mint_cap_fraction = mint_cap_fraction_u32 as u128;
                let mint_cap_u128 = total_supply.safe_div(e, mint_cap_fraction);
                let mint_cap = mint_cap_u128.safe_to_i128(e);

                if delta_a > mint_cap {
                    panic_with_error!(&e, PoolError::SwapReduceOnly);
                }
            } else {
                mint_synthetic_tokens(&e, &e.current_contract_address(), delta_a);
                set_reserve_a(&e, &(reserve_a + (delta_a as u128)));
            }
        }
        if delta_a < 0 {
            burn_synthetic_tokens(&e, &e.current_contract_address(), delta_a.abs() as u128);
            set_reserve_a(&e, &(reserve_a - (delta_a.abs() as u128)));
        }

        let (new_reserve_a, new_reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

        LiquidityPoolEvents::new(&e).rebalance(
            reserve_a,
            reserve_b,
            new_reserve_a,
            new_reserve_b,
            delta_a,
        );
    }
}