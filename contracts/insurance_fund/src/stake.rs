use crate::errors::InsuranceFundError;
use crate::reserve::InsuranceFundReserve;
use crate::storage::{get_reserve, put_reserve};
use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::{contracttype, panic_with_error, Address, Env};
use utils::bump::bump_persistent;
use utils::helpers::log10_iter;
use utils::math::safe_math::SafeMath;
use utils::validate;


impl Stake {
    pub fn new(user: Address, token: Address) -> Self {
        Stake {
            user,
            token,
            last_withdraw_request_shares: 0,
            last_withdraw_request_value: 0,
            last_withdraw_request_ts: 0,
            cost_basis: 0,
            base: 0,
            shares: 0,
        }
    }

    pub fn save(&mut self, e: &Env) {
        save_stake(e, &self.user, &self.token, &self);
    }

    fn validate_base(&self, e: &Env) {
        let reserve = get_reserve(e, &self.token);
        validate!(
            e,
            self.base == reserve.shares_base,
            InsuranceFundError::InvalidIFRebase
        );
    }

    pub fn checked_shares(&self, e: &Env) -> u128 {
        self.validate_base(e);
        self.shares
    }

    pub fn unchecked_shares(&self) -> u128 {
        self.shares
    }

    pub fn increase_shares(&mut self, e: &Env, delta: u128) {
        self.validate_base(e);
        self.shares = self.shares.saturating_add(delta);
    }

    pub fn decrease_shares(&mut self, e: &Env, delta: u128) {
        self.validate_base(e);
        self.shares = self.shares.saturating_sub(delta);
    }

    pub fn update_shares(&mut self, e: &Env, new_shares: u128) {
        self.validate_base(e);
        self.shares = new_shares;
    }
}

pub fn get_stake(e: &Env, user: &Address, token: &Address) -> Stake {
    let key = (user, token);
    let stake_info = match e.storage().persistent().get::<_, Stake>(&key) {
        Some(stake) => {
            bump_persistent(e, &key);
            stake
        }
        None => Stake::new(user.clone(), token.clone()),
    };

    stake_info
}

pub fn save_stake(e: &Env, user: &Address, token: &Address, stake_info: &Stake) {
    let key = (user, token);
    e.storage().persistent().set(&key, stake_info);
    bump_persistent(e, &key);
}

// Applies a rebase to the insurance fund share system to align the total shares
// with the current insurance vault balance.
//
// This function adjusts the total share count and share base exponent if the
// vault amount has changed, simulating a rebase event. If the insurance vault
// value is non-zero and less than the total outstanding shares, a rebase is
// applied to proportionally reduce the total shares. The rebase is computed
// using exponent and divisor logic via `calculate_rebase_info`.
//
// If the vault is non-zero and there are currently zero shares, it initializes
// the total shares to the vault amount.
//
// # Arguments
// * `e` - The Soroban environment reference.
// * `insurance_vault_amount` - The current balance of the insurance fund vault.
//
// # Behavior
// - If `insurance_vault_amount < total_shares`, applies a downward rebase.
// - If `total_shares == 0`, initializes `total_shares` to `insurance_vault_amount`.
//
// # Side Effects
// - Updates `total_shares` and `shares_base` in contract storage.
pub fn apply_rebase_to_insurance_fund(e: &Env, reserve: &mut InsuranceFundReserve) {
    if reserve.balance != 0 && reserve.balance < reserve.total_shares {
        let (expo_diff, rebase_divisor) =
            calculate_rebase_info(e, reserve.total_shares, reserve.balance);

        reserve.total_shares = reserve.total_shares.safe_div(e, rebase_divisor);
        reserve.shares_base = reserve.shares_base.safe_add(e, expo_diff as u128);
    }

    if reserve.balance != 0 && reserve.total_shares == 0 {
        reserve.total_shares = reserve.balance;
    }
}

// Applies a rebase to an individual stake's insurance fund shares to align with the global share base.
//
// This updates a staker’s `if_shares` and `last_withdraw_request_shares` based on the change
// in `shares_base`. If the base has increased (a rebase has occurred), the staker's shares
// are scaled down accordingly.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `stake` - Mutable reference to the user's stake data.
//
// # Behavior
// - Ensures the new base is greater than the previous (`shares_base > stake.if_base`).
// - Reduces shares by a factor of `10^expo_diff`.
// - Updates the stake's `if_base` and both share values.
//
// # Side Effects
// - Mutates the `stake` struct in-place.
pub fn apply_rebase_to_stake(e: &Env, stake: &mut Stake) {
    let reserve = get_reserve(e, &stake.token);

    if reserve.shares_base != stake.base {
        //  "Rebase expo out of bounds"
        validate!(
            e,
            reserve.shares_base > stake.base,
            InsuranceFundError::InvalidIFRebase
        );

        let expo_diff = (reserve.shares_base - stake.base) as u32;

        let rebase_divisor = (10_u128).pow(expo_diff);

        stake.base = reserve.shares_base;

        let old_shares = stake.unchecked_shares();
        let new_shares = old_shares.safe_div(e, rebase_divisor);

        stake.update_shares(e, new_shares);

        stake.last_withdraw_request_shares = stake
            .last_withdraw_request_shares
            .safe_div(e, rebase_divisor);
    }
}

// Converts an insurance vault amount to the equivalent number of insurance fund shares.
//
// Used when a user deposits into the insurance fund and receives shares based on the
// current proportion of total vault value and existing shares.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `amount` - Vault amount to convert.
// * `reserve` - .
//
// # Returns
// - The number of shares that correspond to the input amount.
//
// # Validation
// - If `insurance_vault_amount == 0`, then `total_if_shares` must also be zero.
// - Falls back to 1:1 minting when vault is empty.
pub fn reserve_amount_to_shares(e: &Env, amount: u128, reserve: &InsuranceFundReserve) -> u128 {
    // relative to the entire pool + total amount minted
    let n_shares = if reserve.balance > 0 {
        // assumes total_if_shares != 0 (in most cases) for nice result for user
        amount.fixed_mul_floor(e, &reserve.total_shares, &reserve.balance)
    } else {
        // must be case that total_if_shares == 0 for nice result for user
        validate!(
            e,
            reserve.total_shares == 0,
            InsuranceFundError::InvalidIFSharesDetected
        );

        amount
    };

    n_shares
}

// Converts a number of insurance fund shares into their equivalent vault value.
//
// Used when a user wants to redeem or withdraw from the insurance fund.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `n_shares` - Number of insurance fund shares to convert.
// * `reserve` -
//
// # Returns
// - The proportional vault amount corresponding to the shares.
//
// # Validation
// - Ensures `n_shares <= total_if_shares`.
// - Returns `0` if total shares are zero (vault is empty).
pub fn shares_to_reserve_amount(e: &Env, n_shares: u128, reserve: &InsuranceFundReserve) -> u128 {
    validate!(
        e,
        n_shares <= reserve.total_shares,
        InsuranceFundError::InvalidIFSharesDetected
    );

    let amount = if reserve.total_shares > 0 {
        reserve
            .balance
            .fixed_mul_floor(e, &n_shares, &reserve.total_shares)
    } else {
        0
    };

    amount
}

// Calculates the exponent difference and divisor needed to rebase insurance fund shares.
//
// This determines how much to scale down total shares so that they match the
// current vault value when over-issuance has occurred. Uses logarithmic rounding
// to produce a power-of-ten scaling factor.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `total_if_shares` - Total outstanding insurance fund shares.
// * `insurance_vault_amount` - Total assets in the insurance vault.
//
// # Returns
// - A tuple of:
//   * `expo_diff` — the exponent used to calculate the rebase divisor (as base 10).
//   * `rebase_divisor` — the divisor to apply to all shares (10^expo_diff).
pub fn calculate_rebase_info(
    e: &Env,
    total_if_shares: u128,
    insurance_vault_amount: u128,
) -> (u32, u128) {
    let rebase_divisor_full = total_if_shares
        .safe_div(e, 10)
        .safe_div(e, insurance_vault_amount);

    let expo_diff = log10_iter(rebase_divisor_full) as u32;
    let rebase_divisor = (10_u128).pow(expo_diff);

    (expo_diff, rebase_divisor)
}

// Calculates the number of insurance fund shares a staker would lose due to value drop between request and withdrawal.
//
// This is used to adjust a staker's shares if the vault value has dropped since their
// last withdrawal request, accounting for losses in available collateral.
//
// # Arguments
// * `e` - Soroban environment reference.
// * `stake` - The staker's data, including previous withdrawal request info.
// * `insurance_vault_amount` - Current total assets in the insurance vault.
//
// # Returns
// - The number of shares to be subtracted from the user's withdrawal due to losses.
//
// # Behavior
// - If vault value dropped since request, calculates what shares would now be needed to
//   match the original withdrawal amount and subtracts that from originally requested shares.
//
// # Validation
// - Ensures recalculated shares are not greater than the original request.
pub fn calculate_shares_lost(e: &Env, stake: &Stake, reserve: &InsuranceFundReserve) -> u128 {
    let n_shares = stake.last_withdraw_request_shares;

    let amount = shares_to_reserve_amount(e, n_shares, reserve);

    let if_shares_lost = if amount > stake.last_withdraw_request_value {
        let new_n_shares = reserve_amount_to_shares(
            e,
            stake.last_withdraw_request_value,
            &(InsuranceFundReserve {
                total_shares: reserve.total_shares.saturating_sub(n_shares),
                balance: reserve
                    .balance
                    .saturating_sub(stake.last_withdraw_request_value),
                ..reserve.clone()
            }),
        );

        validate!(
            e,
            new_n_shares <= n_shares,
            InsuranceFundError::InvalidIFSharesDetected
        );

        n_shares.saturating_sub(new_n_shares)
    } else {
        0
    };

    if_shares_lost
}

#[test]
pub fn basic_stake_if_test() {
    use crate::testutils::Setup;
    use utils::{constant::QUOTE_PRECISION, helpers::log10};

    let setup = Setup::default();

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 10000, 10000);
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 20_000, 10000);
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 60_078, 10000);
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 60_078, 9999);
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 60_078, 6008);
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 60_078, 6007);
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 60_078, 6006);
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 60_078, 606);
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 60_078, 600);
    assert_eq!(rebase_div, 10);
    assert_eq!(expo_diff, 1);

    let (expo_diff, rebase_div) = calculate_rebase_info(
        &setup.env,
        60_078 * QUOTE_PRECISION,
        600 * QUOTE_PRECISION + 19234,
    );
    assert_eq!(rebase_div, 10);
    assert_eq!(expo_diff, 1);

    let (expo_diff, rebase_div) = calculate_rebase_info(
        &setup.env,
        60_078 * QUOTE_PRECISION,
        601 * QUOTE_PRECISION + 19234,
    );
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    // $800M goes to 1e-6 of dollar
    let (expo_diff, rebase_div) =
        calculate_rebase_info(&setup.env, 800_000_078 * QUOTE_PRECISION, 1_u128);

    assert_eq!(rebase_div, 10000000000000);
    assert_eq!(expo_diff, 13);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 99_999, 100);
    assert_eq!(log10(100), 2);
    assert_eq!(log10_iter(100), 2);
    assert_eq!(99_999 / 10 / 100, 99);
    assert_eq!(rebase_div, 10);
    assert_eq!(expo_diff, 1);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 100_000, 100);
    assert_eq!(log10(100), 2);
    assert_eq!(100_000 / 10 / 100, 100);
    assert_eq!(rebase_div, 100);
    assert_eq!(expo_diff, 2);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 100_001, 100);
    assert_eq!(log10(100), 2);
    assert_eq!(100_001 / 10 / 100, 100);
    assert_eq!(rebase_div, 100);
    assert_eq!(expo_diff, 2);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 1_242_418_900_000, 1);

    assert_eq!(rebase_div, 100000000000);
    assert_eq!(expo_diff, 11);

    // todo?: does not rebase the other direction (perhaps unnecessary)
    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 12412, 83295723895729080);

    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);
}

#[test]
pub fn if_shares_lost_test() {
    use crate::testutils::Setup;
    use soroban_sdk::testutils::Address;
    use utils::constant::QUOTE_PRECISION;

    let setup = Setup::default();

    let _amount = QUOTE_PRECISION as u64; // $1

    let mut total_shares = 1000 * QUOTE_PRECISION;

    let token = Address::generate(&setup.env);

    let reserve = InsuranceFundReserve::new(token, setup.env.ledger().timestamp());
    let mut if_stake = Stake::new(setup.admin, token);
    if_stake.update_shares(&setup.env, 100 * QUOTE_PRECISION);
    if_stake.last_withdraw_request_shares = 100 * QUOTE_PRECISION;
    if_stake.last_withdraw_request_value = 100 * QUOTE_PRECISION - 1;

    let if_balance = 1000 * QUOTE_PRECISION;

    // unchanged balance
    let lost_shares = calculate_shares_lost(&setup.env, &if_stake, &reserve);
    assert_eq!(lost_shares, 2);

    let if_balance = if_balance + 100 * QUOTE_PRECISION;
    total_shares += 100 * QUOTE_PRECISION;
    let lost_shares = calculate_shares_lost(&setup.env, &if_stake, &reserve);
    assert_eq!(lost_shares, 2); // giving up $5 of gains

    let if_balance = if_balance - 100 * QUOTE_PRECISION;
    total_shares -= 100 * QUOTE_PRECISION;
    let lost_shares = calculate_shares_lost(&setup.env, &if_stake, &reserve);
    assert_eq!(lost_shares, 2); // giving up $5 of gains

    // take back gain
    let if_balance = 1100 * QUOTE_PRECISION;
    let lost_shares = calculate_shares_lost(&setup.env, &if_stake, &reserve);
    assert_eq!(lost_shares, 10_000_001); // giving up $10 of gains

    // doesnt matter if theres a loss
    if_stake.last_withdraw_request_value = 200 * QUOTE_PRECISION;
    let lost_shares = calculate_shares_lost(&setup.env, &if_stake, &reserve);
    assert_eq!(lost_shares, 0);
    if_stake.last_withdraw_request_value = 100 * QUOTE_PRECISION - 1;

    // take back gain and total_if_shares alter w/o user alter
    let if_balance = 2100 * QUOTE_PRECISION;
    total_shares *= 2;
    let lost_shares = calculate_shares_lost(&setup.env, &if_stake, &reserve);
    assert_eq!(lost_shares, 5_000_001); // giving up $5 of gains

    let if_balance = 2100 * QUOTE_PRECISION * 10;

    let expected_gain_if_no_loss = (if_balance * 100) / 2000;
    assert_eq!(expected_gain_if_no_loss, 1_050_000_000);
    let lost_shares = calculate_shares_lost(&setup.env, &if_stake, &reserve);
    assert_eq!(lost_shares, 90_909_092); // giving up $5 of gains
    assert_eq!(
        (9090908 * if_balance) / (total_shares - lost_shares)
            < if_stake.last_withdraw_request_value,
        true
    );
}
