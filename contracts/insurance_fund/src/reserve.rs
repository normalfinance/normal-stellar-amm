use soroban_sdk::{contracttype, Address, Env};

use crate::storage::put_reserve;

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

    pub fn save(&mut self, e: &Env) {
        put_reserve(e, &self.token, &self);
    }

    pub fn deposit(&mut self, n_shares: u128, amount: u128, now: u64) {
        self.total_shares = self.total_shares.saturating_add(n_shares);
        self.balance = self.balance.saturating_add(amount);
        self.total_deposits = self.total_deposits.saturating_add(amount);
        self.last_update_ts = now;
    }

    pub fn withdraw(&mut self, n_shares: u128, amount: u128, now: u64) {
        self.total_shares = self.total_shares.saturating_sub(n_shares);
        self.balance = self.balance.saturating_sub(amount);
        self.total_withdrawals = self.total_withdrawals.saturating_add(amount);
        self.last_update_ts = now;
    }

    pub fn claim(&mut self, amount: u128, now: u64) {
        self.balance = self.balance.saturating_sub(amount);
        self.total_claims = self.total_claims.saturating_add(amount);
        self.last_claim = amount;
        self.last_claim_ts = now;
        self.last_update_ts = now;
    }

    pub fn sync(&mut self, balance: u128, now: u64) {
        self.balance = balance;
        self.last_update_ts = now;
    }

    pub fn skim(&mut self, skimmed: u128, now: u64) {
        self.balance = self.balance.saturating_sub(skimmed);
        self.last_update_ts = now;
    }

    pub fn remove_total_shares(&mut self, amount: u128, now: u64) {
        self.total_shares = self.total_shares.saturating_sub(amount);
        self.last_update_ts = now;
    }
}
