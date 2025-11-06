#![no_std]

use soroban_sdk::{Address, Env, IntoVal, Symbol, Val, Vec};

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
pub trait LiquidityPoolEvents {
    fn rebase(&self);

    fn deposit_liquidity(&self, tokens: Vec<Address>, amounts: Vec<u128>, share_amount: u128);

    fn withdraw_liquidity(&self, tokens: Vec<Address>, amounts: Vec<u128>, share_amount: u128);

    fn trade(
        &self,
        user: Address,
        token_in: Address,
        token_out: Address,
        in_amount: u128,
        out_amount: u128,
        fee_amount: u128,
        tax_amount: u128,
    );

    fn update_reserves(&self, reserves: Vec<u128>);

    fn kill_deposit(&self);

    fn unkill_deposit(&self);

    fn kill_swap(&self);

    fn unkill_swap(&self);

    fn kill_claim(&self);

    fn unkill_claim(&self);

    fn kill_gauges_claim(&self);

    fn unkill_gauges_claim(&self);

    fn kill_tax(&self);

    fn unkill_tax(&self);

    fn kill_bonus(&self);

    fn unkill_bonus(&self);

    fn set_protocol_fee_fraction(&self, fraction: u32);

    fn set_fee_rebate_fraction(&self, fraction: u32);

    fn claim_protocol_fee(&self, token: Address, destination: Address, amount: u128);

    fn claim_protocol_tax(&self, token: Address, destination: Address, amount: u128);

    fn permanently_locked_liquidity(&self, amount: u128);

    // Bonus
    fn claim_bonus(&self, user: Address, token: Address, amount: u128);

    // Rate Tables
    fn set_tax_rate_table(&self, num_entries: u32);

    fn set_bonus_rate_table(&self, num_entries: u32);
}

// This trait is used to emit events related to liquidity pool operations.
// It provides methods for emitting events when liquidity is deposited into the pool,
//  when liquidity is withdrawn from the pool, and when a trade occurs in the pool.
impl LiquidityPoolEvents for Events {
    fn rebase(&self) {
        let e = self.env();
        let fn_name = Symbol::new(e, "rebase");
        let mut topics: Vec<Val> = Vec::from_array(e, [fn_name.to_val()]);
        let mut body: Vec<Val> = Vec::from_array(e, []);
        // for i in 0..tokens.len() {
        //     body.push_back((amounts.get(i).unwrap() as i128).into_val(e));
        //     topics.push_back(tokens.get(i).unwrap().into_val(e));
        // }
        e.events().publish(topics, body);
    }

    fn deposit_liquidity(&self, tokens: Vec<Address>, amounts: Vec<u128>, share_amount: u128) {
        // topics
        // [
        //   "deposit_liquidity": Symbol, // event identifier
        //   assetA: Address,   // contract addresses identifying asset deposited to the pool
        //   assetB: Address,   // contract addresses identifying asset deposited to the pool (optional)
        //   assetC: Address    // contract addresses identifying asset deposited to the pool (optional)
        // ]
        //
        // body
        // [
        //   stake_amount: i128, // amount of pool tokens received from the pool
        //   amountA: i128,      // amount of tokens deposited to the pool for assetA
        //   amountB: i128       // amount of tokens deposited to the pool for assetB (optional)
        //   amountC: i128       // amount of tokens deposited to the pool for assetC (optional)
        // ]
        let e = self.env();
        let fn_name = Symbol::new(e, "deposit_liquidity");
        let mut topics: Vec<Val> = Vec::from_array(e, [fn_name.to_val()]);
        let mut body: Vec<Val> = Vec::from_array(e, [(share_amount as i128).into_val(e)]);
        for i in 0..tokens.len() {
            body.push_back((amounts.get(i).unwrap() as i128).into_val(e));
            topics.push_back(tokens.get(i).unwrap().into_val(e));
        }
        e.events().publish(topics, body);
    }

    fn withdraw_liquidity(&self, tokens: Vec<Address>, amounts: Vec<u128>, share_amount: u128) {
        // topics
        // [
        //   "withdraw_liquidity": Symbol, // event identifier
        //   assetA: Address,   // contract addresses identifying asset withdrawn from the pool
        //   assetB: Address,   // contract addresses identifying asset withdrawn from the pool (optional)
        //   assetC: Address    // contract addresses identifying asset withdrawn from the pool (optional)
        // ]
        //
        // body
        // [
        //   stake_amount: i128, // amount of pool tokens sent to the pool
        //   amountA: i128,      // amount of tokens withdrawn from the pool for assetA
        //   amountB: i128       // amount of tokens withdrawn from the pool for assetB (optional)
        //   amountC: i128       // amount of tokens withdrawn from the pool for assetC (optional)
        // ]
        let e = self.env();
        let fn_name = Symbol::new(e, "withdraw_liquidity");
        let mut topics: Vec<Val> = Vec::from_array(e, [fn_name.to_val()]);
        let mut body: Vec<Val> = Vec::from_array(e, [(share_amount as i128).into_val(e)]);
        for i in 0..tokens.len() {
            body.push_back((amounts.get(i).unwrap() as i128).into_val(e));
            topics.push_back(tokens.get(i).unwrap().into_val(e));
        }
        e.events().publish(topics, body);
    }

    fn trade(
        &self,
        user: Address,
        token_in: Address,
        token_out: Address,
        in_amount: u128,
        out_amount: u128,
        fee_amount: u128,
        tax_amount: u128,
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
        //   tax: u128 - optional
        // ]

        let e = self.env();
        e.events().publish(
            (Symbol::new(e, "trade"), token_in, token_out, user),
            (
                in_amount as i128,
                out_amount as i128,
                fee_amount as i128,
                tax_amount as i128,
            ),
        );
    }

    fn update_reserves(&self, reserves: Vec<u128>) {
        // topics
        // [
        //   "update_reserves": Symbol, // event identifier
        // ]
        //
        // body
        // [
        //   reserve0: i128,      // updated reserve for asset0
        //   reserve1: i128,      // updated reserve for asset1
        //   reserve2: i128       // updated reserve for asset2 (optional)
        //   ...                  // additional reserves if needed
        // ]
        let e = self.env();
        let mut body: Vec<Val> = Vec::new(e);
        for reserve in reserves.iter() {
            body.push_back((reserve as i128).into_val(e));
        }
        e.events()
            .publish((Symbol::new(e, "update_reserves"),), body);
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

    fn kill_gauges_claim(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "kill_gauges_claim"),), ())
    }

    fn unkill_gauges_claim(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "unkill_gauges_claim"),), ())
    }

    fn kill_tax(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "kill_tax"),), ())
    }

    fn unkill_tax(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "unkill_tax"),), ())
    }

    fn kill_bonus(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "kill_bonus"),), ())
    }

    fn unkill_bonus(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "unkill_bonus"),), ())
    }

    fn set_protocol_fee_fraction(&self, fraction: u32) {
        // topics
        // [
        //   "set_protocol_fee": Symbol, // event identifier
        // ]
        //
        // body
        // [
        //   fraction: u32                          // new protocol fee fraction
        // ]
        let e = self.env();
        e.events()
            .publish((Symbol::new(e, "set_protocol_fee"),), (fraction,));
    }

    fn set_fee_rebate_fraction(&self, fraction: u32) {
        // topics
        // [
        //   "set_fee_rebate": Symbol, // event identifier
        // ]
        //
        // body
        // [
        //   fraction: u32                          // new protocol fee fraction
        // ]
        let e = self.env();
        e.events()
            .publish((Symbol::new(e, "set_fee_rebate"),), (fraction,));
    }

    fn claim_protocol_fee(&self, token: Address, destination: Address, amount: u128) {
        // topics
        // [
        //   "claim_protocol_fee": Symbol,  // event identifier
        //   asset: Address,                // contract address identifying asset claimed
        // ]
        //
        // body
        // [
        //   destination: Address,          // address of account/contract that received the claimed tokens
        //   amount: i128                   // amount of tokens claimed
        // ]
        let e = self.env();
        e.events().publish(
            (Symbol::new(e, "claim_protocol_fee"), token),
            (destination, amount as i128),
        );
    }

    fn claim_protocol_tax(&self, token: Address, destination: Address, amount: u128) {
        // topics
        // [
        //   "claim_protocol_tax": Symbol,  // event identifier
        //   asset: Address,                // contract address identifying asset claimed
        // ]
        //
        // body
        // [
        //   destination: Address,          // address of account/contract that received the claimed tokens
        //   amount: i128                   // amount of tokens claimed
        // ]
        let e = self.env();
        e.events().publish(
            (Symbol::new(e, "claim_protocol_tax"), token),
            (destination, amount as i128),
        );
    }

    fn permanently_locked_liquidity(&self, amount: u128) {
        let e = self.env();
        e.events()
            .publish((Symbol::new(e, "permanently_locked_liquidity"),), amount);
    }

    // Bonus

    fn claim_bonus(&self, user: Address, token: Address, amount: u128) {
        // topics
        // [
        //   "claim_bonus": Symbol,       // event identifier
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
        e.events()
            .publish((Symbol::new(e, "claim_bonus"), user, token), amount as i128);
    }

    fn set_tax_rate_table(&self, num_entries: u32) {
        // topics
        // [
        //   "set_tax_rate_table": Symbol, // event identifier
        // ]
        //
        // body
        // [
        //   num_entries: u32                    // number of entries in the table
        // ]
        let e = self.env();
        e.events()
            .publish((Symbol::new(e, "set_tax_rate_table"),), (num_entries,));
    }

    fn set_bonus_rate_table(&self, num_entries: u32) {
        // topics
        // [
        //   "set_bonus_rate_table": Symbol, // event identifier
        // ]
        //
        // body
        // [
        //   num_entries: u32                      // number of entries in the table
        // ]
        let e = self.env();
        e.events()
            .publish((Symbol::new(e, "set_bonus_rate_table"),), (num_entries,));
    }
}
