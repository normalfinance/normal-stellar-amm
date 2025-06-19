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

pub(crate) trait ProviderFeeEvents {
    fn charge_provider_fee(&self, token: Address, amount: u128);

    fn claim_fee(&self, token: Address, amount: u128);

    fn buffer_deposit(&self, token: Address, amount: u128);

    fn insurance_premium(&self, token: Address, amount: u128);
}

impl ProviderFeeEvents for Events {
    fn charge_provider_fee(&self, token: Address, amount: u128) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "charge_provider_fee"),), (token, amount));
    }

    fn claim_fee(&self, token: Address, amount: u128) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "withdraw_fee"),), (token, amount));
    }

    fn buffer_deposit(&self, token: Address, amount: u128) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "buffer_deposit"),), (token, amount));
    }

    fn insurance_premium(&self, token: Address, amount: u128) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "insurance_premium"),), (token, amount));
    }
}
