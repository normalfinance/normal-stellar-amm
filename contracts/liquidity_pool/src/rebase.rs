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
