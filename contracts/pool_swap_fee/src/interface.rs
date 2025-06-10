use soroban_sdk::{ Address, BytesN, Env, Vec };

pub trait PoolSwapFeeInterface {
    // swap
    // Executes a multi-hop token swap with fee deduction.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //   - user: The user initiating the swap (must be authorized).
    //   - swap: A swap path.
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
    ) -> u128;

    // swap_strict_receive
    // Executes a multi-hop swap ensuring a specific output amount by adjusting the input and fee.
    //
    // Arguments:
    //   - e: The Soroban environment.
    //   - user: The user initiating the swap (must be authorized).
    //   - swap: A swap path.
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
    ) -> u128;
}

pub trait AdminInterface {
    // Initialize admin user. Will panic if called twice
    fn init_admin(e: Env, account: Address);

    // Set the Router
    fn set_router(e: Env, admin: Address, router: Address);

    // Set the Buffer
    fn set_buffer(e: Env, admin: Address, buffer: Address);

    // Set the Fee Collector
    fn set_fee_destination(e: Env, admin: Address, fee_destination: Address);

    // Set the buffer fraction
    fn set_buffer_fraction(e: Env, admin: Address, buffer_fraction: u32);

    // Set the max swap fee fraction
    fn set_max_swap_fee_fraction(e: Env, admin: Address, max_swap_fee_fraction: u32);

    // Get the max swap fee fraction
    fn get_max_swap_fee_fraction(e: Env) -> u32;

    // Get the buffer fraction
    fn get_buffer_fraction(e: Env) -> u32;

    // Get the Router
    fn get_router(e: Env) -> Address;

    // Get the Buffer
    fn get_buffer(e: Env) -> Address;

    // Get the Fee Destination
    fn get_fee_destination(e: Env) -> Address;

    // Claim swap fees and send to the fee destination
    fn claim_fees(e: Env, admin: Address, token: Address) -> u128;

    // Claim swap fees, swap them to the token, and send to the fee destination
    fn claim_fees_and_swap(
        e: Env,
        admin: Address,
        swap: (Vec<Address>, BytesN<32>, Address),
        token: Address,
        out_min: u128
    ) -> u128;
}
