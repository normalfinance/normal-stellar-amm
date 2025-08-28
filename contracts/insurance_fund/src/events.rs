use soroban_sdk::{Address, Env, Symbol};

use normal_rust_types::types::StakeAction;

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
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    fn insurance_stake_record(
        &self,
        user: Address,
        token: Address,
        action: StakeAction,
        amount: u128,
        reserve_balance_before: u128,
        stake_shares_before: u128,
        total_shares_before: u128,
        stake_shares_after: u128,
        total_shares_after: u128,
    );

    fn collect_premium(&self, sender: Address, token: Address, amount: u128);

    fn whitelist_token(&self, sender: Address, token: Address, symbol: Symbol);

    fn remove_whitelist_token(&self, sender: Address, token: Address, reserve_amount: u128);

    fn sync_optimal_insurance(
        &self,
        sender: Address,
        previous_insurance: u128,
        new_insurance: u128,
    );

    fn premium_payer_status_updated(
        &self,
        ts: u64,
        admin: Address,
        user: Address,
        old_status: bool,
        new_status: bool,
    );

    fn sync(&self, sender: Address, token: Address, amount: i128);

    fn skim(&self, sender: Address, token: Address, amount: i128);

    //    _______     __       ____  ____   ________  _______  ________
    //   |   __ "\   /""\     ("  _||_ " | /"       )/"     "||"      "\
    //   (. |__) :) /    \    |   (  ) : |(:   \___/(: ______)(.  ___  :)
    //   |:  ____/ /' /\  \   (:  |  | . ) \___  \   \/    |  |: \   ) ||
    //   (|  /    //  __'  \   \\ \__/ //   __/  \\  // ___)_ (| (___\ ||
    //  /|__/ \  /   /  \\  \  /\\ __ //\  /" \   :)(:      "||:       :)
    // (_______)(___/    \___)(__________)(_______/  \_______)(________/

    fn kill_deposit(&self);

    fn unkill_deposit(&self);

    fn kill_request_withdraw(&self);

    fn unkill_request_withdraw(&self);

    fn kill_withdraw(&self);

    fn unkill_withdraw(&self);
}

impl InsuranceFundEvents for Events {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    fn insurance_stake_record(
        &self,
        user: Address,
        token: Address,
        action: StakeAction,
        amount: u128,
        reserve_balance_before: u128,
        stake_shares_before: u128,
        total_shares_before: u128,
        stake_shares_after: u128,
        total_shares_after: u128,
    ) {
        self.env().events().publish(
            (
                Symbol::new(self.env(), "insurance_stake_record"),
                user,
                token,
                action,
            ),
            (
                amount,
                reserve_balance_before,
                stake_shares_before,
                total_shares_before,
                stake_shares_after,
                total_shares_after,
            ),
        );
    }

    fn collect_premium(&self, sender: Address, token: Address, amount: u128) {
        self.env().events().publish(
            (Symbol::new(self.env(), "collect_premium"), sender),
            (token, amount),
        );
    }

    fn whitelist_token(&self, sender: Address, token: Address, symbol: Symbol) {
        self.env().events().publish(
            (Symbol::new(self.env(), "whitelist_token"), sender),
            (token, symbol),
        );
    }

    fn remove_whitelist_token(&self, sender: Address, token: Address, reserve_amount: u128) {
        self.env().events().publish(
            (Symbol::new(self.env(), "remove_whitelist_token"), sender),
            (token, reserve_amount),
        );
    }

    fn sync_optimal_insurance(
        &self,
        sender: Address,
        previous_insurance: u128,
        new_insurance: u128,
    ) {
        self.env().events().publish(
            (Symbol::new(self.env(), "sync_optimal_insurance"), sender),
            (previous_insurance, new_insurance),
        );
    }

    fn premium_payer_status_updated(
        &self,
        ts: u64,
        admin: Address,
        user: Address,
        old_status: bool,
        new_status: bool,
    ) {
        self.env().events().publish(
            (
                Symbol::new(self.env(), "premium_payer_status_updated"),
                ts,
                admin,
                user,
                old_status,
                new_status,
            ),
            (),
        );
    }

    fn sync(&self, user: Address, token: Address, amount: i128) {
        self.env()
            .events()
            .publish((Symbol::new(self.env(), "sync"), token), (user, amount));
    }

    fn skim(&self, user: Address, token: Address, amount: i128) {
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
