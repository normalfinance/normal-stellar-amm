#![no_std]

use soroban_sdk::{ Address, Env, Symbol };

#[derive(Clone)]
pub struct Events(Env);

impl Events {
    #[inline(always)]
    pub fn env(&self) -> &Env {
        &self.0
    }

    #[inline(always)]
    pub fn new(env: &Env) -> Events {
        Events(env.clone())
    }
}

// This trait is used to emit events related to liquidity pool operations.
// It provides methods for emitting events when liquidity is deposited into the pool,
//  when liquidity is withdrawn from the pool, and when a trade occurs in the pool.
// Events structured to ease integration with third party tools.
pub trait PoolEvents {
    fn deposit_liquidity(&self, token: Address, amount: u128, share_amount: u128);

    fn withdraw_liquidity(&self, token: Address, amount: u128, share_amount: u128);

    fn trade(
        &self,
        user: Address,
        token_in: Address,
        token_out: Address,
        in_amount: u128,
        out_amount: u128,
        fee_amount: u128
    );

    fn rebalance(&self, delta_a: i128, new_reserve_a: u128);

    fn kill_deposit(&self);

    fn unkill_deposit(&self);

    fn kill_withdraw(&self);

    fn unkill_withdraw(&self);

    fn kill_swap(&self);

    fn unkill_swap(&self);

    fn kill_claim(&self);

    fn unkill_claim(&self);
}

// This trait is used to emit events related to liquidity pool operations.
// It provides methods for emitting events when liquidity is deposited into the pool,
//  when liquidity is withdrawn from the pool, and when a trade occurs in the pool.
impl PoolEvents for Events {
    fn deposit_liquidity(&self, token: Address, amount: u128, share_amount: u128) {
        // topics
        // [
        //   "deposit_liquidity": Symbol, // event identifier
        //   token: Address,   // contract addresses identifying asset deposited to the pool
        // ]
        //
        // body
        // [
        //   share_amount: i128, // amount of pool tokens received from the pool
        //   amount: i128,      // amount of tokens deposited to the pool for assetA
        // ]
        let e = self.env();
        e.events().publish(
            (Symbol::new(e, "deposit_liquidity"), token),
            (share_amount as i128, amount as i128)
        );
    }

    fn withdraw_liquidity(&self, token: Address, amount: u128, share_amount: u128) {
        // topics
        // [
        //   "withdraw_liquidity": Symbol, // event identifier
        //   asset: Address,   // contract addresses identifying asset withdrawn from the pool
        // ]
        //
        // body
        // [
        //   share_amount: i128, // amount of pool tokens sent to the pool
        //   amount: i128,      // amount of tokens withdrawn from the pool for assetA
        // ]
        let e = self.env();
        e.events().publish(
            (Symbol::new(e, "withdraw_liquidity"), token),
            (share_amount as i128, amount as i128)
        );
    }

    fn trade(
        &self,
        user: Address,
        token_in: Address,
        token_out: Address,
        in_amount: u128,
        out_amount: u128,
        fee_amount: u128
    ) {
        // topics
        // [
        //   "trade": Symbol,       // event identifier
        //   sold_asset: Address,   // asset sent to the pool
        //   bought_asset: Address, // asset received from the pool
        //   trader: Address        // address of account/contract that initiated the trade
        // ]
        // body
        // [
        //   sold_amount: i128,   // amount of tokens sent to the pool
        //   bought_amount: i128, // amount of tokens received from the pool
        //   fee: i128            // fee charged by the protocol (asset sent to the pool) - optional
        // ]

        let e = self.env();
        e.events().publish(
            (Symbol::new(e, "trade"), token_in, token_out, user),
            (in_amount as i128, out_amount as i128, fee_amount as i128)
        );
    }

    fn rebalance(&self, delta_a: i128, new_reserve_a: u128) {
        // topics
        // [
        //   "rebalance": Symbol, // event identifier
        // ]
        //
        // body
        // [
        //   oracle_price: i128, // amount of pool tokens sent to the pool
        //   pool_price: i128,      // amount of tokens withdrawn from the pool for assetA
        // ]
        let e = self.env();
        e.events().publish((Symbol::new(e, "rebalance"),), (delta_a, new_reserve_a as i128));
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

    fn kill_swap(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "kill_swap"),), ())
    }

    fn unkill_swap(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "unkill_swap"),), ())
    }

    fn kill_claim(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "kill_claim"),), ())
    }

    fn unkill_claim(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "unkill_claim"),), ())
    }
}
