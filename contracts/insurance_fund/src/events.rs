use soroban_sdk::{ Address, Env, Symbol };

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

pub(crate) trait InsuranceFundEvents {
    fn if_stake_record(
        &self,
        ts: u64,
        user: Address,
        action: StakeAction,
        amount: u128,
        insurance_vault_amount_before: u128,
        if_shares_before: u128,
        total_if_shares_before: u128,
        if_shares_after: u128,
        total_if_shares_after: u128
    );

    fn collect_premium(&self, sender: Address, amount: u128);

    fn kill_deposit(&self);

    fn unkill_deposit(&self);

    fn kill_request_withdraw(&self);

    fn unkill_request_withdraw(&self);

    fn kill_withdraw(&self);

    fn unkill_withdraw(&self);
}

impl InsuranceFundEvents for Events {
    fn if_stake_record(
        &self,
        ts: u64,
        user: Address,
        action: StakeAction,
        amount: u128,
        insurance_vault_amount_before: u128,
        if_shares_before: u128,
        total_if_shares_before: u128,
        if_shares_after: u128,
        total_if_shares_after: u128
    ) {
        self.env()
            .events()
            .publish(
                (Symbol::new(self.env(), "deposit"), user, action),
                (
                    ts,
                    amount,
                    insurance_vault_amount_before,
                    if_shares_before,
                    total_if_shares_before,
                    if_shares_after,
                    total_if_shares_after,
                )
            );
    }

    fn collect_premium(&self, sender: Address, amount: u128) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "collect_premium"), sender), amount);
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

    fn kill_request_withdraw(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "kill_request_withdraw"),), ())
    }

    fn unkill_request_withdraw(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "unkill_request_withdraw"),), ())
    }

    fn kill_withdraw(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "kill_withdraw"),), ())
    }

    fn unkill_withdraw(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "unkill_withdraw"),), ())
    }
}
