use soroban_sdk::{contracttype, Address, Env};
use utils::math::safe_math::SafeMath;

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

    pub fn deposit(&mut self, e: &Env, amount: u128, now: u64) {
        self.balance = self.balance.safe_add(e, amount);
        self.total_deposits = self.total_deposits.safe_add(e, amount);
        self.last_update_ts = now;
    }

    pub fn withdraw(&mut self, e: &Env, amount: u128, now: u64) {
        self.balance = self.balance.safe_sub(e, amount);
        self.total_withdrawals = self.total_withdrawals.safe_add(e, amount);
        self.last_update_ts = now;
    }

    pub fn claim(&mut self, e: &Env, amount: u128, now: u64) {
        self.balance = self.balance.safe_sub(e, amount);
        self.total_claims = self.total_claims.safe_add(e, amount);
        self.last_claim = amount;
        self.last_claim_ts = now;
        self.last_update_ts = now;
    }

    pub fn update_balance(&mut self, amount: u128, now: u64) {
        self.balance = amount;
        self.last_update_ts = now;
    }

    pub fn add_total_shares(&mut self, e: &Env, amount: u128, now: u64) {
        self.total_shares = self.total_shares.safe_add(e, amount);
        self.last_update_ts = now;
    }

    pub fn remove_total_shares(&mut self, e: &Env, amount: u128, now: u64) {
        self.total_shares = self.total_shares.safe_sub(e, amount);
        self.last_update_ts = now;
    }
}
