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

    pub fn set_incentives_config(&self, expired_at: u64, tps: u128) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "set_incentives_config"),), (expired_at, tps))
    }

    pub fn claim(
        &self,
        user: Address,
        reward_token: Address,
        amount: u128,
        token_b: Address,
        fees_owed: u128
    ) {
        // topics
        // [
        //   "claim_rewclaim_incentivesard": Symbol,    // event identifier
        //   reward_token: Address,     // Address of token claimed
        //   claimant: Address          // address of account/contract that initiated the claim
        // ]
        // body
        // [
        //   amount: i128,              // amount of reward tokens claimed
        // ]

        let e = self.env();
        e.events().publish(
            (Symbol::new(e, "claim_incentives"), reward_token, token_b, user),
            (amount as i128, fees_owed as i128)
        );
    }
}
