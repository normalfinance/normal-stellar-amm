use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsuranceFundReserve {
    pub token: Address,
    pub balance: u128, // the current token balance of the reserve.
    // shares
    pub total_shares: u128, // the total amount of issued shares.
    pub shares_base: u128,  // exponent for lp shares (for rebasing).
    // metrics
    pub total_deposits: u128, // the total amount of deposits made into this reserve.
    pub total_withdrawals: u128, // the total amount of payouts made from this reserve.
    pub total_claims: u128,
    pub last_claim: u128,    // the token amount of the last claim.
    pub last_claim_ts: u64,  // the timestamp of the last claim.
    pub last_update_ts: u64, // the timestamp of the last reserve update.
}

impl InsuranceFundReserve {
    pub fn new(token: Address, now: u64) -> Self {
        InsuranceFundReserve {
            token,
            balance: 0,
            total_deposits: 0,
            total_withdrawals: 0,
            total_claims: 0,
            total_shares: 0,
            shares_base: 0,
            last_claim: 0,
            last_claim_ts: 0,
            last_update_ts: now,
        }
    }

    pub fn deposit(self, amount: u128, now: u64) -> Self {
        InsuranceFundReserve {
            balance: self.balance.saturating_add(amount),
            total_deposits: self.total_deposits.saturating_add(amount),
            last_update_ts: now,
            ..self
        }
    }

    pub fn withdraw(self, amount: u128, now: u64) -> Self {
        InsuranceFundReserve {
            balance: self.balance.saturating_sub(amount),
            total_withdrawals: self.total_withdrawals.saturating_add(amount),
            last_update_ts: now,
            ..self
        }
    }

    pub fn claim(self, amount: u128, now: u64) -> Self {
        InsuranceFundReserve {
            balance: self.balance.saturating_sub(amount),
            total_claims: self.total_claims.saturating_add(amount),
            last_claim: amount,
            last_claim_ts: now,
            last_update_ts: now,
            ..self
        }
    }

    pub fn update_balance(self, amount: u128, now: u64) -> Self {
        InsuranceFundReserve {
            balance: amount,
            last_update_ts: now,
            ..self
        }
    }
}
