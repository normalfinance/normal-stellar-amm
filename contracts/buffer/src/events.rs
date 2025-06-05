use soroban_sdk::{ Address, BytesN, Env, Symbol, Val, Vec };

use crate::stake::StakeAction;

#[derive(Clone)]
pub(crate) struct Events(Env);

impl Events {
    #[inline(always)]
    pub(crate) fn env(&self) -> &Env {
        &self.0
    }

    #[inline(always)]
    pub(crate) fn new(env: &Env) -> Events {
        Events(env.clone())
    }
}

pub(crate) trait BufferEvents {
    fn deposit(&self, token: Address, user: Address, amount: u128);

    fn request_payout(&self, token: Address, user: Address, amount: u128);

    fn withdraw_surplus(&self, token: Address, user: Address, amount: u128);

    fn kill_deposit(&self);

    fn unkill_deposit(&self);

    fn kill_request_payout(&self);

    fn unkill_request_payout(&self);
}

impl BufferEvents for Events {
    fn deposit(&self, token: Address, user: Address, amount: u128) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "deposit"), token, user), amount);
    }

    fn request_payout(&self, token: Address, user: Address, amount: u128) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "request_payout"), token, user), amount);
    }

    fn withdraw_surplus(&self, token: Address, user: Address, amount: u128) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "withdraw_surplus"), token, user), amount);
    }

    fn kill_deposit(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "kill_deposit"),), ())
    }

    fn unkill_deposit(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "unkill_deposit"),), ())
    }

    fn kill_request_payout(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "kill_request_payout"),), ())
    }

    fn unkill_request_payout(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "unkill_request_payout"),), ())
    }
}
