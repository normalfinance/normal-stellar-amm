#![no_std]

use soroban_sdk::{Address, Env, Symbol};

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

pub trait PoolEvents {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    fn deposit_liquidity(
        &self,
        token: Address,
        user: Address,
        amount: u128,
        share_amount: u128,
        delta_a: i128,
    );

    fn withdraw_liquidity(
        &self,
        token: Address,
        user: Address,
        share_amount: u128,
        amount: u128,
        delta_a: i128,
    );

    fn swap(
        &self,
        user: Address,
        token_in: Address,
        token_out: Address,
        in_amount: u128,
        out_amount: u128,
        delta_a_prior: i128,
        delta_a_post: i128,
    );

    fn rebalance(
        &self,
        reserve_a: u128,
        reserve_b: u128,
        new_reserve_a: u128,
        new_reserve_b: u128,
        delta_a: i128,
    );

    fn capped_mint(&self, base_oracle_price: u128, quote_oracle_price: u128, delta_a: i128);

    //    _______     __       ____  ____   ________  _______  ________
    //   |   __ "\   /""\     ("  _||_ " | /"       )/"     "||"      "\
    //   (. |__) :) /    \    |   (  ) : |(:   \___/(: ______)(.  ___  :)
    //   |:  ____/ /' /\  \   (:  |  | . ) \___  \   \/    |  |: \   ) ||
    //   (|  /    //  __'  \   \\ \__/ //   __/  \\  // ___)_ (| (___\ ||
    //  /|__/ \  /   /  \\  \  /\\ __ //\  /" \   :)(:      "||:       :)
    // (_______)(___/    \___)(__________)(_______/  \_______)(________/

    fn kill_deposit(&self);

    fn unkill_deposit(&self);

    fn kill_withdraw(&self);

    fn unkill_withdraw(&self);

    fn kill_swap(&self);

    fn unkill_swap(&self);

    fn kill_claim(&self);

    fn unkill_claim(&self);

    fn permanently_locked_liquidity(&self, amount: u128);
}

impl PoolEvents for Events {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    fn deposit_liquidity(
        &self,
        token: Address,
        user: Address,
        amount: u128,
        share_amount: u128,
        delta_a: i128,
    ) {
        let e = self.env();
        e.events().publish(
            (Symbol::new(e, "deposit_liquidity"), token, user),
            (amount, share_amount, delta_a),
        );
    }

    fn withdraw_liquidity(
        &self,
        token: Address,
        user: Address,
        share_amount: u128,
        amount: u128,
        delta_a: i128,
    ) {
        let e = self.env();
        e.events().publish(
            (Symbol::new(e, "withdraw_liquidity"), token, user),
            (share_amount, amount, delta_a),
        );
    }

    fn swap(
        &self,
        user: Address,
        token_in: Address,
        token_out: Address,
        in_amount: u128,
        out_amount: u128,
        delta_a_prior: i128,
        delta_a_post: i128,
    ) {
        let e = self.env();
        e.events().publish(
            (Symbol::new(e, "swap"), token_in, token_out, user),
            (in_amount, out_amount, delta_a_prior, delta_a_post),
        );
    }

    fn rebalance(
        &self,
        reserve_a: u128,
        reserve_b: u128,
        new_reserve_a: u128,
        new_reserve_b: u128,
        delta_a: i128,
    ) {
        let e = self.env();
        e.events().publish(
            (Symbol::new(e, "rebalance"),),
            (reserve_a, reserve_b, new_reserve_a, new_reserve_b, delta_a),
        );
    }

    fn capped_mint(&self, base_oracle_price: u128, quote_oracle_price: u128, delta_a: i128) {
        let e = self.env();
        e.events().publish(
            (Symbol::new(e, "capped_mint"),),
            (base_oracle_price, quote_oracle_price, delta_a),
        );
    }

    //    _______     __       ____  ____   ________  _______  ________
    //   |   __ "\   /""\     ("  _||_ " | /"       )/"     "||"      "\
    //   (. |__) :) /    \    |   (  ) : |(:   \___/(: ______)(.  ___  :)
    //   |:  ____/ /' /\  \   (:  |  | . ) \___  \   \/    |  |: \   ) ||
    //   (|  /    //  __'  \   \\ \__/ //   __/  \\  // ___)_ (| (___\ ||
    //  /|__/ \  /   /  \\  \  /\\ __ //\  /" \   :)(:      "||:       :)
    // (_______)(___/    \___)(__________)(_______/  \_______)(________/

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

    fn permanently_locked_liquidity(&self, amount: u128) {
        let e = self.env();
        e.events()
            .publish((Symbol::new(e, "permanently_locked_liquidity"),), amount);
    }
}
