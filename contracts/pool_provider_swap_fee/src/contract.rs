use crate::errors::Error;
use crate::events::{ Events, ProviderFeeEvents };
use crate::helpers;
use crate::interface::ProviderSwapFeeInterface;
use crate::storage::{
    get_buffer,
    get_fee_destination,
    get_max_swap_fee_fraction,
    get_operator,
    get_router,
    set_buffer,
    set_fee_destination,
    set_max_swap_fee_fraction,
    set_operator,
    set_router,
};
use soroban_sdk::auth::{ ContractContext, InvokerContractAuthEntry, SubContractInvocation };
use soroban_sdk::token::Client as SorobanTokenClient;
use soroban_sdk::{
    contract,
    contractimpl,
    panic_with_error,
    vec,
    Address,
    BytesN,
    Env,
    IntoVal,
    Symbol,
    Vec,
};
use utils::constant::{ FEE_DENOMINATOR, PRICE_PRECISION };
use utils::token::transfer_token;

#[contract]
pub struct ProviderSwapFeeCollector;

#[contractimpl]
impl ProviderSwapFeeCollector {
    // __constructor
    // Initializes the ProviderSwapFeeCollector contract.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //   - router: The address of the swap router contract.
    //   - operator: The address authorized to claim funds.
    //   - fee_destination: The address where fees are sent.
    //   - buffer: The address of the insurance fund contract.
    //   - max_swap_fee_fraction: The maximum fee in basis points (bps).
    //   - buffer_fraction: Portion of revenue for insurance in basis points (bps).
    pub fn __constructor(
        e: Env,
        router: Address,
        operator: Address,
        fee_destination: Address,
        buffer: Address,
        max_swap_fee_fraction: u32,
        buffer_fraction: u32
    ) {
        set_router(&e, &router);
        set_operator(&e, &operator);
        set_fee_destination(&e, &fee_destination);
        set_buffer(&e, &buffer);
        set_max_swap_fee_fraction(&e, &max_swap_fee_fraction);
        set_buffer_fraction(&e, &buffer_fraction);
    }

    pub fn update_buffer_fraction(e: Env, operator: Address, buffer_fraction: u32) {
        operator.require_auth();
        if operator != get_operator(&e) {
            panic_with_error!(&e, Error::Unauthorized);
        }

        // if buffer_fraction < MIN_BUFFER_FRACTION {
        //     panic_with_error!(&e, Error::FeeFractionTooHigh);
        // }

        set_buffer_fraction(&e, &buffer_fraction);
    }

    // get_max_swap_fee_fraction
    // Returns the maximum swap fee in basis points.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //
    // Returns:
    //   - A u32 value representing the maximum fee in basis points.
    pub fn get_max_swap_fee_fraction(e: Env) -> u32 {
        get_max_swap_fee_fraction(&e)
    }

    // get_buffer_fraction
    // Returns the maximum swap fee in basis points.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //
    // Returns:
    //   - A u32 value representing the maximum fee in basis points.
    pub fn get_buffer_fraction(e: Env) -> u32 {
        get_buffer_fraction(&e)
    }

    // get_router
    // Returns the address of the router contract used for swaps.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //
    // Returns:
    //   - An Address representing the swap router.
    pub fn get_router(e: Env) -> Address {
        get_router(&e)
    }

    // get_fee_destination
    // Returns the address where fees are sent.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //
    // Returns:
    //   - An Address representing the fee destination.
    pub fn get_fee_destination(e: Env) -> Address {
        get_fee_destination(&e)
    }

    // get_buffer
    // Returns the address where fees are sent.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //
    // Returns:
    //   - An Address representing the fee destination.
    pub fn get_buffer(e: Env) -> Address {
        get_buffer(&e)
    }

    // claim_fees
    // Claims all fees held by the contract and transfers them to the specified address.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //   - operator: The address calling for fee claiming (must match the stored operator).
    //   - token: The token contract address for which fees are claimed.
    //
    // Returns:
    //   - A u128 value representing the claimed token amount.
    pub fn claim_fees(e: Env, operator: Address, token: Address) -> u128 {
        operator.require_auth();
        if operator != get_operator(&e) {
            panic_with_error!(&e, Error::Unauthorized);
        }
        let amount = token_client.balance(&e.current_contract_address());
        transfer_token(
            &e,
            &token,
            &e.current_contract_address(),
            &get_fee_destination(&e),
            &amount
        );
        Events::new(&e).claim_fee(token.clone(), amount as u128, token, amount as u128);
        amount as u128
    }

    // claim_fees_and_swap
    // Claims fees and swaps them immediately using the router.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //   - operator: The address calling for the fee claim and swap (must match the stored operator).
    //   - swaps_chain: A vector describing the swap path; each element is a tuple of (intermediate token addresses, function hash, output token address).
    //   - token: The token for which fees are claimed.
    //   - out_min: The minimum acceptable output amount from the swap.
    //   - to: The destination address for the swapped tokens.
    //
    // Returns:
    //   - A u128 value representing the output token amount received after the swap.
    pub fn claim_fees_and_swap(
        e: Env,
        operator: Address,
        swap: (Vec<Address>, BytesN<32>, Address),
        token: Address,
        out_min: u128
    ) -> u128 {
        operator.require_auth();
        if operator != get_operator(&e) {
            panic_with_error!(&e, Error::Unauthorized);
        }
        let (_, _, token_out) = match swap.last() {
            Some(v) => v,
            None => panic_with_error!(&e, Error::PathIsEmpty),
        };
        let router = get_router(&e);
        let token_client = SorobanTokenClient::new(&e, &token);
        let amount = token_client.balance(&e.current_contract_address()) as u128;
        e.authorize_as_current_contract(
            vec![
                &e,
                InvokerContractAuthEntry::Contract(SubContractInvocation {
                    context: ContractContext {
                        contract: token.clone(),
                        fn_name: Symbol::new(&e, "transfer"),
                        args: (
                            e.current_contract_address(),
                            router.clone(),
                            amount as i128,
                        ).into_val(&e),
                    },
                    sub_invocations: vec![&e],
                })
            ]
        );
        let out_amount: u128 = e.invoke_contract(
            &get_router(&e),
            &Symbol::new(&e, "swap"),
            Vec::from_array(&e, [
                e.current_contract_address().to_val(),
                swaps_chain.to_val(),
                token.clone().to_val(),
                amount.into_val(&e),
                out_min.into_val(&e),
            ])
        );
        transfer_token(
            &e,
            &token_out,
            &e.current_contract_address(),
            &get_fee_destination(&e),
            &(out_amount as i128)
        );
        Events::new(&e).claim_fee(token, amount, token_out, out_amount);
        out_amount
    }
}

#[contractimpl]
impl ProviderSwapFeeInterface for ProviderSwapFeeCollector {
    // swap
    // Executes a token swap with fee deduction.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //   - user: The user initiating the swap (must be authorized).
    //   - swap: The swap args.
    //   - token_in: The input token address.
    //   - in_amount: The amount of token_in provided by the user.
    //   - out_min: The minimum acceptable output token amount (after fee deduction).
    //   - fee_fraction: The provider fee fraction in basis points (bps).
    //
    // Returns:
    //   - A u128 value representing the net output tokens transferred to the user.
    fn swap(
        e: Env,
        user: Address,
        swap: (Vec<Address>, BytesN<32>, Address),
        token_in: Address,
        in_amount: u128,
        out_min: u128,
        fee_fraction: u32
    ) -> u128 {
        user.require_auth();

        if fee_fraction > get_max_swap_fee_fraction(&e) {
            panic_with_error!(&e, Error::FeeFractionTooHigh);
        }

        transfer_token(&e, &token_in, &user, &e.current_contract_address(), &(in_amount as i128));

        let router = get_router(&e);
        e.authorize_as_current_contract(
            vec![
                &e,
                InvokerContractAuthEntry::Contract(SubContractInvocation {
                    context: ContractContext {
                        contract: token_in.clone(),
                        fn_name: Symbol::new(&e, "transfer"),
                        args: (
                            e.current_contract_address(),
                            router.clone(),
                            in_amount as i128,
                        ).into_val(&e),
                    },
                    sub_invocations: vec![&e],
                })
            ]
        );
        let amount_out: u128 = e.invoke_contract(
            &router,
            &Symbol::new(&e, "swap"),
            Vec::from_array(&e, [
                e.current_contract_address().to_val(),
                swap.into_val(&e),
                token_in.clone().to_val(),
                in_amount.into_val(&e),
                out_min.into_val(&e),
            ])
        );
        let fee_amount = (amount_out * (fee_fraction as u128)) / (FEE_DENOMINATOR as u128);
        let amount_out_w_fee = amount_out - fee_amount;
        if amount_out_w_fee < out_min {
            panic_with_error!(&e, Error::OutMinNotSatisfied);
        }
        transfer_token(
            &e,
            &swap.2,
            &e.current_contract_address(),
            &user,
            &(amount_out_w_fee as i128)
        );
        Events::new(&e).charge_provider_fee(swap.2, fee_amount);

        // Send portion of fee to the Buffer
        let fee_amount_for_buffer = fee_amount * get_buffer_fraction(&e);
        let buffer = get_buffer(&e);

        let token_client = SorobanTokenClient::new(&e, &token);

        e.authorize_as_current_contract(
            vec![
                &e,
                InvokerContractAuthEntry::Contract(SubContractInvocation {
                    context: ContractContext {
                        contract: token.clone(),
                        fn_name: Symbol::new(&e, "transfer"),
                        args: (
                            e.current_contract_address(),
                            buffer.clone(),
                            fee_amount_for_buffer as i128,
                        ).into_val(&e),
                    },
                    sub_invocations: vec![&e],
                })
            ]
        );

        let settled_amount: u128 = e.invoke_contract(
            &get_buffer(&e),
            &Symbol::new(&e, "deposit"),
            Vec::from_array(&e, [
                e.current_contract_address().to_val(),
                &token,
                fee_amount_for_buffer.into_val(&e),
            ])
        );

        Events::new(&e).settle_revenue(token.clone(), settled_amount as u128);

        // // Update total incentives data and refresh/initialize user incentive
        // let incentives = get_incentives_manager(&e);
        // let total_shares = get_total_shares(&e);
        // let user_shares = get_user_balance_shares(&e, &user);
        // let token_a_fees_collected = 0;
        // let token_b_fees_collected = 0;
        // incentives
        //     .manager()
        //     .checkpoint_user(
        //         &user,
        //         total_shares,
        //         user_shares,
        //         token_a_fees_collected,
        //         token_b_fees_collected
        //     );

        amount_out_w_fee
    }

    // swap_strict_receive
    // Executes a swap ensuring a specific output amount by adjusting the input and fee.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //   - user: The user initiating the swap (must be authorized).
    //   - swap: A vector defining the swap path.
    //   - token_in: The input token address.
    //   - out_amount: The exact target output amount.
    //   - in_max: The maximum amount of token_in the user is willing to spend.
    //   - fee_fraction: The provider fee fraction in basis points (bps).
    //
    // Returns:
    //   - A u128 value representing the total input amount (including fees) required.
    fn swap_strict_receive(
        e: Env,
        user: Address,
        swap: (Vec<Address>, BytesN<32>, Address),
        token_in: Address,
        out_amount: u128,
        in_max: u128,
        fee_fraction: u32
    ) -> u128 {
        user.require_auth();

        if fee_fraction > get_max_swap_fee_fraction(&e) {
            panic_with_error!(&e, Error::FeeFractionTooHigh);
        }

        transfer_token(&e, &token_in, &user, &e.current_contract_address(), &(in_max as i128));
        let router = get_router(&e);
        e.authorize_as_current_contract(
            vec![
                &e,
                InvokerContractAuthEntry::Contract(SubContractInvocation {
                    context: ContractContext {
                        contract: token_in.clone(),
                        fn_name: Symbol::new(&e, "transfer"),
                        args: (
                            e.current_contract_address(),
                            router.clone(),
                            in_max as i128,
                        ).into_val(&e),
                    },
                    sub_invocations: vec![&e],
                })
            ]
        );
        let amount_in: u128 = e.invoke_contract(
            &router,
            &Symbol::new(&e, "swap_strict_receive"),
            Vec::from_array(&e, [
                e.current_contract_address().to_val(),
                swap.into_val(&e),
                token_in.clone().to_val(),
                out_amount.into_val(&e),
                in_max.into_val(&e),
            ])
        );
        transfer_token(&e, &swap.2, &e.current_contract_address(), &user, &(out_amount as i128));
        let fee_amount = (amount_in * (fee_fraction as u128)) / (FEE_DENOMINATOR as u128);
        let amount_in_with_fee = amount_in + fee_amount;
        if amount_in_with_fee > in_max {
            panic_with_error!(&e, Error::InMaxNotSatisfied);
        }
        let surplus = in_max - amount_in_with_fee;
        if surplus > 0 {
            transfer_token(&e, &token_in, &e.current_contract_address(), &user, &(surplus as i128));
        }
        Events::new(&e).charge_provider_fee(token_in, fee_amount);

        // Send portion of fee to the Buffer
        let fee_amount_for_buffer = fee_amount * get_buffer_fraction(&e);
        let buffer = get_buffer(&e);

        let token_client = SorobanTokenClient::new(&e, &token);

        e.authorize_as_current_contract(
            vec![
                &e,
                InvokerContractAuthEntry::Contract(SubContractInvocation {
                    context: ContractContext {
                        contract: token.clone(),
                        fn_name: Symbol::new(&e, "transfer"),
                        args: (
                            e.current_contract_address(),
                            buffer.clone(),
                            fee_amount_for_buffer as i128,
                        ).into_val(&e),
                    },
                    sub_invocations: vec![&e],
                })
            ]
        );

        let settled_amount: u128 = e.invoke_contract(
            &get_buffer(&e),
            &Symbol::new(&e, "deposit"),
            Vec::from_array(&e, [
                e.current_contract_address().to_val(),
                &token,
                fee_amount_for_buffer.into_val(&e),
            ])
        );

        Events::new(&e).settle_revenue(token.clone(), settled_amount as u128);

        amount_in_with_fee
    }
}
