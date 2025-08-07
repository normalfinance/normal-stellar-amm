use soroban_sdk::{Address, Env, Symbol, Val, Vec};
use utils::state::pool::SwapDirection;

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

pub(crate) trait PoolRouterEvents {
    fn deposit_liquidity(
        &self,
        asset: Symbol,
        pool: Address,
        user: Address,
        amount: u128,
        share_amount: u128,
    );

    fn swap(
        &self,
        asset: Symbol,
        pool: Address,
        user: Address,
        direction: SwapDirection,
        in_amount: u128,
        out_amount: u128,
    );

    fn withdraw_liquidity(
        &self,
        asset: Symbol,
        pool: Address,
        user: Address,
        share_amount: u128,
        amount: u128,
    );

    fn add_pool(&self, asset: Symbol, token_b: Address, pool: Address, init_args: Vec<Val>);

    fn config_rewards(&self, asset: Symbol, pool: Address, pool_tps: u128, expired_at: u64);

    fn claim(
        &self,
        asset: Symbol,
        pool: Address,
        user: Address,
        reward_token: Address,
        reward_amount: u128,
    );
}

impl PoolRouterEvents for Events {
    fn deposit_liquidity(
        &self,
        asset: Symbol,
        pool: Address,
        user: Address,
        amount: u128,
        share_amount: u128,
    ) {
        self.env().events().publish(
            (
                Symbol::new(self.env(), "deposit_liquidity"),
                asset,
                pool,
                user,
            ),
            (amount, share_amount),
        );
    }

    fn swap(
        &self,
        asset: Symbol,
        pool: Address,
        user: Address,
        direction: SwapDirection,
        in_amount: u128,
        out_amount: u128,
    ) {
        self.env().events().publish(
            (Symbol::new(self.env(), "swap"), asset, pool, user),
            (direction, in_amount, out_amount),
        );
    }

    fn withdraw_liquidity(
        &self,
        asset: Symbol,
        pool: Address,
        user: Address,
        share_amount: u128,
        amount: u128,
    ) {
        self.env().events().publish(
            (
                Symbol::new(self.env(), "withdraw_liquidity"),
                asset,
                pool,
                user,
            ),
            (share_amount, amount),
        );
    }

    fn add_pool(&self, asset: Symbol, token_b: Address, pool: Address, init_args: Vec<Val>) {
        self.env().events().publish(
            (Symbol::new(self.env(), "add_pool"), asset),
            (pool, token_b, init_args),
        );
    }

    fn config_rewards(&self, asset: Symbol, pool: Address, pool_tps: u128, expired_at: u64) {
        self.env().events().publish(
            (Symbol::new(self.env(), "config_rewards"), asset, pool),
            (pool_tps, expired_at),
        );
    }

    fn claim(
        &self,
        asset: Symbol,
        pool: Address,
        user: Address,
        reward_token: Address,
        reward_amount: u128,
    ) {
        self.env().events().publish(
            (Symbol::new(self.env(), "claim"), asset, pool, user),
            (reward_token, reward_amount),
        );
    }
}
