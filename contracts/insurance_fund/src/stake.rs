use crate::errors::InsuranceFundError;
use crate::storage::{ get_shares_base, get_total_shares, set_shares_base, set_total_shares };
use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::{ contracttype, log, panic_with_error, Address, Env };
use utils::bump::{ bump_instance, bump_persistent, bump_temporary };
use utils::errors::math_errors::MathError;
use utils::helpers::log10_iter;
use utils::math::safe_math::SafeMath;
use utils::{ safe_decrement, safe_increment, validate };

#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StakeAction {
    Deposit,
    WithdrawRequest,
    WithdrawCancelRequest,
    Withdraw,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stake {
    pub if_shares: u128,
    pub last_withdraw_request_shares: u128, // get zero as 0 when not in escrow
    pub if_base: u128, // exponent for if_shares decimal places (for rebase)
    pub last_valid_ts: u64,
    pub last_withdraw_request_value: u128,
    pub last_withdraw_request_ts: u64,
    pub cost_basis: u128,
}

impl Stake {
    pub fn new(now: u64) -> Self {
        Stake {
            last_withdraw_request_shares: 0,
            last_withdraw_request_value: 0,
            last_withdraw_request_ts: 0,
            cost_basis: 0,
            if_base: 0,
            last_valid_ts: now,
            if_shares: 0,
        }
    }

    fn validate_base(&self, e: &Env) {
        let shares_base = get_shares_base(e);

        //  "if stake bases mismatch. user base: {} market base {}",
        validate!(e, self.if_base == shares_base, InsuranceFundError::InvalidIFRebase);
    }

    pub fn checked_if_shares(&self, e: &Env) -> u128 {
        self.validate_base(e);
        self.if_shares
    }

    pub fn unchecked_if_shares(&self) -> u128 {
        self.if_shares
    }

    pub fn increase_if_shares(&mut self, e: &Env, delta: u128) {
        self.validate_base(e);
        safe_increment!(e, self.if_shares, delta);
    }

    pub fn decrease_if_shares(&mut self, e: &Env, delta: u128) {
        self.validate_base(e);
        safe_decrement!(e, self.if_shares, delta);
    }

    pub fn update_if_shares(&mut self, e: &Env, new_shares: u128) {
        self.validate_base(e);
        self.if_shares = new_shares;
    }
}

pub fn get_stake(e: &Env, key: &Address) -> Stake {
    let stake_info = match e.storage().persistent().get::<_, Stake>(key) {
        Some(stake) => {
            bump_persistent(e, &key);
            stake
        }
        None => Stake::new(e.ledger().timestamp()),
    };

    stake_info
}

pub fn save_stake(e: &Env, key: &Address, stake_info: &Stake) {
    e.storage().persistent().set(key, stake_info);
    bump_persistent(e, &key);
}

pub fn apply_rebase_to_insurance_fund(e: &Env, insurance_vault_amount: u128) {
    let total_shares = get_total_shares(e);
    let shares_base = get_shares_base(e);

    if insurance_vault_amount != 0 && insurance_vault_amount < total_shares {
        let (expo_diff, rebase_divisor) = calculate_rebase_info(
            e,
            total_shares,
            insurance_vault_amount
        );

        set_total_shares(e, &total_shares.safe_div(e, rebase_divisor));
        set_shares_base(e, &shares_base.safe_add(e, expo_diff as u128));

        log!(e, "rebasing insurance fund: expo_diff={}", expo_diff);
    }

    if insurance_vault_amount != 0 && total_shares == 0 {
        set_total_shares(e, &(insurance_vault_amount as u128));
    }
}

pub fn apply_rebase_to_stake(e: &Env, stake: &mut Stake) {
    let shares_base = get_shares_base(e);

    if shares_base != stake.if_base {
        //  "Rebase expo out of bounds"
        validate!(e, shares_base > stake.if_base, InsuranceFundError::InvalidIFRebase);

        let expo_diff = (shares_base - stake.if_base) as u32;

        let rebase_divisor = (10_u128).pow(expo_diff);

        log!(e, "rebasing insurance fund stake: base: {} -> {} ", stake.if_base, shares_base);

        stake.if_base = shares_base;

        let old_if_shares = stake.unchecked_if_shares();
        let new_if_shares = old_if_shares.safe_div(e, rebase_divisor);

        log!(e, "rebasing insurance fund stake: shares -> {} ", new_if_shares);

        stake.update_if_shares(e, new_if_shares);

        stake.last_withdraw_request_shares = stake.last_withdraw_request_shares.safe_div(
            e,
            rebase_divisor
        );
    }
}

pub fn vault_amount_to_if_shares(
    e: &Env,
    amount: u128,
    total_if_shares: u128,
    insurance_vault_amount: u128
) -> u128 {
    // relative to the entire pool + total amount minted
    let n_shares = if insurance_vault_amount > 0 {
        // assumes total_if_shares != 0 (in most cases) for nice result for user
        amount.fixed_mul_floor(e, &total_if_shares, &insurance_vault_amount)
        // get_proportion_u128(e, amount, total_if_shares, insurance_vault_amount)
    } else {
        // must be case that total_if_shares == 0 for nice result for user
        // "assumes total_if_shares == 0"
        validate!(e, total_if_shares == 0, InsuranceFundError::InvalidIFSharesDetected);

        amount
    };

    n_shares
}

pub fn if_shares_to_vault_amount(
    e: &Env,
    n_shares: u128,
    total_if_shares: u128,
    insurance_vault_amount: u128
) -> u128 {
    //  "n_shares({}) > total_if_shares({})",
    validate!(e, n_shares <= total_if_shares, InsuranceFundError::InvalidIFSharesDetected);

    let amount = if total_if_shares > 0 {
        insurance_vault_amount.fixed_mul_floor(e, &n_shares, &total_if_shares)
        // get_proportion_u128(e, insurance_vault_amount, n_shares, total_if_shares)
    } else {
        0
    };

    amount
}

pub fn calculate_rebase_info(
    e: &Env,
    total_if_shares: u128,
    insurance_vault_amount: u128
) -> (u32, u128) {
    let rebase_divisor_full = total_if_shares.safe_div(e, 10).safe_div(e, insurance_vault_amount);

    let expo_diff = log10_iter(rebase_divisor_full) as u32;
    let rebase_divisor = (10_u128).pow(expo_diff);

    (expo_diff, rebase_divisor)
}

pub fn calculate_if_shares_lost(e: &Env, stake: &Stake, insurance_vault_amount: u128) -> u128 {
    let total_shares = get_total_shares(e);

    let n_shares = stake.last_withdraw_request_shares;

    let amount = if_shares_to_vault_amount(e, n_shares, total_shares, insurance_vault_amount);

    let if_shares_lost = if amount > stake.last_withdraw_request_value {
        let new_n_shares = vault_amount_to_if_shares(
            e,
            stake.last_withdraw_request_value,
            total_shares.safe_sub(e, n_shares),
            insurance_vault_amount.safe_sub(e, stake.last_withdraw_request_value)
        );

        //  "Issue calculating delta if_shares after canceling request {} < {}",
        validate!(e, new_n_shares <= n_shares, InsuranceFundError::InvalidIFSharesDetected);

        n_shares.safe_sub(e, new_n_shares)
    } else {
        0
    };

    if_shares_lost
}
