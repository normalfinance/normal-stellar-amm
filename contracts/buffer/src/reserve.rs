use soroban_sdk::{ contracttype, Env };
use utils::math::safe_math::SafeMath;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reserve {
    pub balance: u128,
    pub max_balance: u128,
    pub total_inflow: u128,
    pub total_outflow: u128,
    pub total_withdraw: u128,
    pub last_payout: u128,
    pub last_payout_ts: u64,
}

impl Reserve {
    pub fn new() -> Self {
        Reserve {
            balance: 0,
            max_balance: 0,
            total_inflow: 0,
            total_outflow: 0,
            total_withdraw: 0,
            last_payout: 0,
            last_payout_ts: 0,
        }
    }

    pub fn update_max_balance(&mut self, e: &Env, max_balance: u128) -> Self {
        Reserve {
            max_balance,
            ..self
        }
    }

    pub fn deposit(&mut self, e: &Env, amount: u128) -> Self {
        Reserve {
            balance: self.balance.safe_add(e, amount),
            total_inflow: self.total_inflow.safe_add(e, amount),
            ..self
        }
    }

    pub fn payout(&mut self, e: &Env, amount: u128, now: u64) -> Self {
        Reserve {
            balance: self.balance.safe_sub(e, amount),
            total_outflow: self.total_outflow.safe_add(e, amount),
            last_payout: amount,
            last_payout_ts: now,
            ..self
        }
    }

    pub fn withdraw(&mut self, e: &Env, amount: u128) -> Self {
        Reserve {
            balance: self.balance.safe_sub(e, amount),
            total_outflow: self.total_outflow.safe_add(e, amount),
            total_withdraw: self.total_withdraw.safe_add(e, amount),
            ..self
        }
    }

    pub fn update_balance(&mut self, e: &Env, amount: u128) -> Self {
        Reserve {
            balance: amount,
            ..self
        }
    }
}
