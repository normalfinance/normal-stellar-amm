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

pub(crate) trait BufferFundEvents {
    fn settle(
        &self,
        tokens: Vec<Address>,
        user: Address,
        pool_address: Address,
        reward_token: Address,
        reward_amount: u128
    );

    fn claim(
        &self,
        tokens: Vec<Address>,
        user: Address,
        pool_address: Address,
        reward_token: Address,
        reward_amount: u128
    );
}

impl BufferFundEvents for Events {
    fn settle(&self, token: Address, user: Address, pool_address: Address, amount: u128) {
        self.env()
            .events()
            .publish(
                (Symbol::new(self.env(), "claim"), token, user),
                (pool_address, token, amount)
            );
    }

    fn claim(
        &self,
        tokens: Vec<Address>,
        user: Address,
        pool_address: Address,
        reward_token: Address,
        reward_amount: u128
    ) {
        self.env()
            .events()
            .publish(
                (Symbol::new(self.env(), "claim"), tokens, user),
                (pool_address, reward_token, reward_amount)
            );
    }
}
