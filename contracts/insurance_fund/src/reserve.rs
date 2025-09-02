use soroban_sdk::{Address, Env};
use normal_rust_types::types::InsuranceFundReserve;

use crate::storage::put_reserve;

pub trait InsuranceFundReserveExt {
    fn new(token: Address, now: u64) -> Self;
    fn save(&mut self, e: &Env);
    fn deposit(&mut self, amount: u128, now: u64);
    fn withdraw(&mut self, amount: u128, now: u64);
    fn claim(&mut self, amount: u128, now: u64);
    fn update_balance(&mut self, amount: u128, now: u64);
    fn add_total_shares(&mut self, amount: u128, now: u64);
    fn remove_total_shares(&mut self, amount: u128, now: u64);
}

impl InsuranceFundReserveExt for InsuranceFundReserve {
    fn new(token: Address, now: u64) -> Self {
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

    fn save(&mut self, e: &Env) {
        put_reserve(e, &self.token, &self);
    }

    fn deposit(&mut self, amount: u128, now: u64) {
        self.balance = self.balance.saturating_add(amount);
        self.total_deposits = self.total_deposits.saturating_add(amount);
        self.last_update_ts = now;
    }

    fn withdraw(&mut self, amount: u128, now: u64) {
        self.balance = self.balance.saturating_sub(amount);
        self.total_withdrawals = self.total_withdrawals.saturating_add(amount);
        self.last_update_ts = now;
    }

    fn claim(&mut self, amount: u128, now: u64) {
        self.balance = self.balance.saturating_sub(amount);
        self.total_claims = self.total_claims.saturating_add(amount);
        self.last_claim = amount;
        self.last_claim_ts = now;
        self.last_update_ts = now;
    }

    fn update_balance(&mut self, amount: u128, now: u64) {
        self.balance = amount;
        self.last_update_ts = now;
    }

    fn add_total_shares(&mut self, amount: u128, now: u64) {
        self.total_shares = self.total_shares.saturating_add(amount);
        self.last_update_ts = now;
    }

    fn remove_total_shares(&mut self, amount: u128, now: u64) {
        self.total_shares = self.total_shares.saturating_sub(amount);
        self.last_update_ts = now;
    }
}
