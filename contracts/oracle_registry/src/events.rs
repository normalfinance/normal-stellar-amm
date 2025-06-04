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

pub(crate) trait OracleRegistryEvents {
    fn update_oracle(
        &self,
        user: Address
        // TODO:
    );
}

impl OracleRegistryEvents for Events {
    fn update_oracle(&self, user: Address) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "update_oracle"), user), ());
    }
}
