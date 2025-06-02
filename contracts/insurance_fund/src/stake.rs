use crate::errors::{ InsuranceFundError };
use soroban_fixed_point_math::SorobanFixedPoint;
use crate::storage::{
    get_insurance_vault_amount,
    get_shares_base,
    get_total_shares,
    put_shares_base,
    put_total_shares,
    put_user_shares,
};
use soroban_sdk::{ contracttype, log, panic_with_error, Address, Env };
use utils::bump::{ bump_instance, bump_persistent, bump_temporary };
use utils::helpers::log10_iter;
use utils::math::safe_math::SafeMath;
use utils::{ safe_decrement, safe_increment, validate };
use utils::errors::math_errors::MathError;

//   ________  ___________   __       __   ___  _______
//  /"       )("     _   ") /""\     |/"| /  ")/"     "|
// (:   \___/  )__/  \\__/ /    \    (: |/   /(: ______)
//  \___  \       \\_ /   /' /\  \   |    __/  \/    |
//   __/  \\      |.  |  //  __'  \  (// _  \  // ___)_
//  /" \   :)     \:  | /   /  \\  \ |: | \  \(:      "|
// (_______/       \__|(___/    \___)(__|  \__)\_______)

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
    pub cost_basis: i64,
}

impl Stake {
    pub fn new(now: u64) -> Self {
        Stake {
            // authority,
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

        validate!(
            e,
            self.if_base == shares_base,
            InsuranceFundError::InvalidIFRebase,
            "if stake bases mismatch. user base: {} market base {}",
            self.if_base,
            insurance_fund.shares_base
        );
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

    if insurance_vault_amount != 0 && insurance_vault_amount < total_shares {
        let (expo_diff, rebase_divisor) = calculate_rebase_info(
            e,
            total_shares,
            insurance_vault_amount
        );

        put_total_shares(e, total_shares / rebase_divisor); // safe_div
        put_user_shares(e, user_shares / rebase_divisor); // safe_div\
        put_shares_base(e, shares_base + expo_diff); // safe_add

        log!(e, "rebasing insurance fund: expo_diff={}", expo_diff);
    }

    if insurance_vault_amount != 0 && total_shares == 0 {
        put_total_shares(e, insurance_vault_amount as u128);
    }
}

pub fn apply_rebase_to_stake(e: &Env, stake: &mut Stake) {
    let shares_base = get_shares_base(e);

    if shares_base != stake.if_base {
        validate!(
            e,
            shares_base > stake.if_base,
            InsuranceFundError::InvalidIFRebase,
            "Rebase expo out of bounds"
        );

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
        validate!(
            e,
            total_if_shares == 0,
            InsuranceFundError::InvalidIFSharesDetected,
            "assumes total_if_shares == 0"
        );

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
    validate!(
        e,
        n_shares <= total_if_shares,
        InsuranceFundError::InvalidIFSharesDetected,
        "n_shares({}) > total_if_shares({})",
        n_shares,
        total_if_shares
    );

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

        // "Issue calculating delta if_shares after canceling request {} < {}", new_n_shares, n_shares
        if new_n_shares <= n_shares {
            panic_with_error!(e, InsuranceFundError::InvalidIFSharesDetected);
        }

        n_shares.safe_sub(e, new_n_shares)
    } else {
        0
    };

    if_shares_lost
}

pub fn settle_revenue_to_insurance_fund(
    e: &Env,
    insurance_vault_amount: u64,
    now: i64,
    check_invariants: bool
) -> u64 {
    // update_spot_market_cumulative_interest(spot_market, None, now)?;

    validate!(
        e,
        user_factor <= total_factor,
        InsuranceFundError::RevenueSettingsCannotSettleToIF,
        "invalid if_factor settings on spot market"
    );

    let depositors_claim = validate_spot_market_vault_amount(
        spot_market,
        spot_market_vault_amount
    )?;

    // let mut token_amount = get_token_amount(
    //     spot_market.revenue_pool.scaled_balance,
    //     spot_market,
    //     &SpotBalanceType::Deposit,
    // )?;
    // let mut token_amount = get_insurance_vault_amount(e);

    if depositors_claim < token_amount.cast()? {
        // only allow half of withdraw available when utilization is high
        token_amount = depositors_claim.max(0).cast::<u128>()?.safe_div(2)?;
    }

    if user_shares > 0 {
        // only allow MAX_APR_PER_REVENUE_SETTLE_TO_INSURANCE_FUND_VAULT or 1/10th of revenue pool to be settled
        let capped_apr_amount = insurance_vault_amount

            .safe_mul(MAX_APR_PER_REVENUE_SETTLE_TO_INSURANCE_FUND_VAULT.cast::<u128>()?)?
            .safe_div(PERCENTAGE_PRECISION)?
            .safe_div(ONE_YEAR.safe_div(revenue_settle_period.cast()?)?.max(1))?;
        let capped_token_pct_amount = token_amount.safe_div(10)?;
        token_amount = capped_token_pct_amount.min(capped_apr_amount);
    }

    let insurance_fund_token_amount = token_amount.fixed_mul_floor(
        e,
        &SHARE_OF_REVENUE_ALLOCATED_TO_INSURANCE_FUND_VAULT_NUMERATOR,
        &SHARE_OF_REVENUE_ALLOCATED_TO_INSURANCE_FUND_VAULT_DENOMINATOR
    );
    // let insurance_fund_token_amount = get_proportion_u128(
    //     token_amount,
    //     SHARE_OF_REVENUE_ALLOCATED_TO_INSURANCE_FUND_VAULT_NUMERATOR,
    //     SHARE_OF_REVENUE_ALLOCATED_TO_INSURANCE_FUND_VAULT_DENOMINATOR,
    // )?
    // .cast::<u64>()?;

    if check_invariants {
        validate!(
            &e,
            insurance_fund_token_amount != 0,
            InsuranceFundError::NoRevenueToSettleToIF,
            "no amount to settle to insurance fund"
        );
    }

    last_revenue_settle_ts = now;

    let protocol_if_factor = total_factor.safe_sub(user_factor);

    // give protocol its cut
    if protocol_if_factor > 0 {
        let n_shares = vault_amount_to_if_shares(
            insurance_fund_token_amount
                .safe_mul(protocol_if_factor.cast()?)?
                .safe_div(spot_market.insurance_fund.total_factor.cast()?)?,
            spot_market.insurance_fund.total_shares,
            insurance_vault_amount
        )?;

        put_total_shares(e, total_shares + n_shares);
    }

    let total_if_shares_before = total_shares;

    update_revenue_pool_balances(
        insurance_fund_token_amount.cast::<u128>()?,
        &SpotBalanceType::Borrow,
        spot_market
    )?;

    // emit!(InsuranceFundRecord {
    //     ts: now,
    //     spot_market_index: spot_market.market_index,
    //     perp_market_index: 0, // todo: make option?
    //     amount: insurance_fund_token_amount.cast()?,

    //     user_if_factor: spot_market.insurance_fund.user_factor,
    //     total_if_factor: spot_market.insurance_fund.total_factor,
    //     vault_amount_before: spot_market_vault_amount,
    //     insurance_vault_amount_before: insurance_vault_amount,
    //     total_if_shares_before,
    //     total_if_shares_after: spot_market.insurance_fund.total_shares,
    // });

    insurance_fund_token_amount.cast()
}
