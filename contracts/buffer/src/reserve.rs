use soroban_sdk::{contracttype, Env};
use utils::math::safe_math::SafeMath;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reserve {
    pub balance: u128,        // the current token balance of the reserve.
    pub max_balance: u128,    // the max token balance the reserve can have.
    pub total_inflow: u128,   // the total amount of deposits made into this reserve.
    pub total_outflow: u128,  // the total amount of payouts made from this reserve.
    pub total_withdraw: u128, //  the total amount of withdraws made from this reserve.
    pub last_payout: u128,    // the token amount of the last payout.
    pub last_payout_ts: u64,  // the timestamp of the last payout.
    pub last_update_ts: u64,  // the timestamp of the last reserve update.
}

impl Reserve {
    pub fn new(now: u64) -> Self {
        Reserve {
            balance: 0,
            max_balance: 0,
            total_inflow: 0,
            total_outflow: 0,
            total_withdraw: 0,
            last_payout: 0,
            last_payout_ts: 0,
            last_update_ts: now,
        }
    }

    pub fn update_max_balance(self, max_balance: u128, now: u64) -> Self {
        Reserve {
            max_balance,
            last_update_ts: now,
            ..self
        }
    }

    pub fn deposit(self, e: &Env, amount: u128, now: u64) -> Self {
        Reserve {
            balance: self.balance.safe_add(e, amount),
            total_inflow: self.total_inflow.safe_add(e, amount),
            last_update_ts: now,
            ..self
        }
    }

    pub fn payout(self, e: &Env, amount: u128, now: u64) -> Self {
        Reserve {
            balance: self.balance.safe_sub(e, amount),
            total_outflow: self.total_outflow.safe_add(e, amount),
            last_payout: amount,
            last_payout_ts: now,
            last_update_ts: now,
            ..self
        }
    }

    pub fn withdraw(self, e: &Env, amount: u128, now: u64) -> Self {
        Reserve {
            balance: self.balance.safe_sub(e, amount),
            total_outflow: self.total_outflow.safe_add(e, amount),
            total_withdraw: self.total_withdraw.safe_add(e, amount),
            last_update_ts: now,
            ..self
        }
    }

    pub fn update_balance(self, amount: u128, now: u64) -> Self {
        Reserve {
            balance: amount,
            last_update_ts: now,
            ..self
        }
    }
}
