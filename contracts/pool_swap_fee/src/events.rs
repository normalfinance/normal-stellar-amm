use soroban_sdk::{Address, Env, Symbol};
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

pub(crate) trait ProviderFeeEvents {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    fn swap(
        &self,
        asset: Symbol,
        pool: Address,
        user: Address,
        direction: SwapDirection,
        in_amount: u128,
        out_amount: u128,
        fee_amount: u128,
        lp_fee: u128,
        buffer_fee: u128,
        if_premium: u128,
        revenue_fee: u128,
    );

    fn claim_fees(&self, token: Address, sender: Address, amount: u128);
}

impl ProviderFeeEvents for Events {
    //  ___      ___       __        __    _____  ___
    // |"  \    /"  |     /""\      |" \  (\"   \|"  \
    //  \   \  //   |    /    \     ||  | |.\\   \    |
    //  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
    // |: \.        |  //  __'  \   |.  | |.  \    \. |
    // |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
    // |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

    fn swap(
        &self,
        asset: Symbol,
        pool: Address,
        user: Address,
        direction: SwapDirection,
        in_amount: u128,
        out_amount: u128,
        fee_amount: u128,
        lp_fee: u128,
        buffer_fee: u128,
        if_premium: u128,
        revenue_fee: u128,
    ) {
        self.env().events().publish(
            (Symbol::new(self.env(), "swap"), asset, pool, user),
            (
                direction,
                in_amount,
                out_amount,
                fee_amount,
                lp_fee,
                buffer_fee,
                if_premium,
                revenue_fee,
            ),
        );
    }

    fn claim_fees(&self, token: Address, sender: Address, amount: u128) {
        self.env().events().publish(
            (Symbol::new(self.env(), "claim_fees"),),
            (token, sender, amount),
        );
    }
}
