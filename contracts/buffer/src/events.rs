use soroban_sdk::{ Address, Env, Symbol };

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

    fn resolve_liquidity_deficit(
        &self,
        pool: Address,
        token: Address,
        user: Address,
        amount: u128,
        paid: u128
    );

    fn withdraw_surplus(&self, token: Address, user: Address, amount: u128);

    fn skim(&self, token: Address, user: Address, amount: i128);

    //    _______     __       ____  ____   ________  _______  ________
    //   |   __ "\   /""\     ("  _||_ " | /"       )/"     "||"      "\
    //   (. |__) :) /    \    |   (  ) : |(:   \___/(: ______)(.  ___  :)
    //   |:  ____/ /' /\  \   (:  |  | . ) \___  \   \/    |  |: \   ) ||
    //   (|  /    //  __'  \   \\ \__/ //   __/  \\  // ___)_ (| (___\ ||
    //  /|__/ \  /   /  \\  \  /\\ __ //\  /" \   :)(:      "||:       :)
    // (_______)(___/    \___)(__________)(_______/  \_______)(________/

    fn kill_deposit(&self);

    fn unkill_deposit(&self);

    fn kill_resolve_liquidity_deficit(&self);

    fn unkill_resolve_liquidity_deficit(&self);
}

impl BufferEvents for Events {
    fn deposit(&self, token: Address, user: Address, amount: u128) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "deposit"), token), (user, amount));
    }

    fn resolve_liquidity_deficit(
        &self,
        pool: Address,
        token: Address,
        user: Address,
        amount: u128,
        paid: u128
    ) {
        self.env()
            .events()
            .publish(
                (Symbol::new(self.env(), "resolve_liquidity_deficit"), pool, token),
                (user, amount, paid)
            );
    }

    fn withdraw_surplus(&self, token: Address, user: Address, amount: u128) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "withdraw_surplus"), token), (user, amount));
    }

    fn skim(&self, token: Address, user: Address, amount: i128) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "skim"), token), (user, amount));
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

    fn kill_resolve_liquidity_deficit(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "kill_resolve_liquidity_deficit"),), ())
    }

    fn unkill_resolve_liquidity_deficit(&self) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "unkill_resolve_liquidity_deficit"),), ())
    }
}
