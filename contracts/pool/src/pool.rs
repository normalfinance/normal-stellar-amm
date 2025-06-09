use crate::errors::PoolError;
use crate::errors::PoolValidationError;
use crate::events::Events as LiquidityPoolEvents;
use crate::events::PoolEvents;
use crate::oracle;
use crate::storage::{get_reserve_a, get_reserve_b, put_reserve_a};
use pool_tokens::{burn_synthetic_tokens, get_total_synthetic_tokens, mint_synthetic_tokens};
use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::contracttype;
use soroban_sdk::Address;
use soroban_sdk::{panic_with_error, Env};

use utils::constant::PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128;
use utils::constant::{FEE_MULTIPLIER, PRICE_PRECISION};
use utils::math::safe_math::SafeMath;
use utils::storage::AssetId;
use utils::storage::PoolStatus;
use utils::storage::PoolTier;
use utils::token::get_token_balance;
use utils::validate;

#[contracttype]
#[derive(Clone)]
pub struct Pool {
    pub asset: Address,
    pub token_b: Address,
    // Oracle address for the base (synthetic) asset (i.e. nBTC)
    pub base_asset_id: AssetId,
    // Oracle address for the quote asset (TokenB) - usually XLM or USDC
    pub quote_asset_id: AssetId,
    pub tier: PoolTier,
    pub status: PoolStatus,
    pub fee_fraction: u32, // 1 = 0.01%
    // The pool's claim on the insurance fund
    pub insurance_claim: InsuranceClaim,
    // The max liquidity imbalance before price premiums are added and/or the buffer/if is used
    // liquidity imbalance is the difference between quote token and base token value. When it's less than 0,
    // the pool does not have enough liquidity to fill all orders and will apply a price premium to new swaps.
    // precision = QUOTE_PRECISION
    pub liquidity_max_imbalance: u64,
}

impl Pool {
    // Gets the current pool liquidity imbalance.
    //
    // # Arguments
    //
    // * base_oracle_price - Price of the base token.
    // * quote_oracle_price - Price of the quote token.
    //
    // # Returns
    //
    // The liquidity imbalance of the pool as an i128.
    pub fn calculate_net_liquidity_imbalance(
        &self,
        e: &Env,
        base_oracle_price: u128,
        quote_oracle_price: u128,
    ) -> i128 {
        validate!(
            e,
            base_oracle_price > 0,
            PoolError::InvalidOracle,
            "base_oracle_price <= 0"
        );
        validate!(
            e,
            quote_oracle_price > 0,
            PoolError::InvalidOracle,
            "quote_oracle_price <= 0"
        );

        let base_token_supply = get_total_synthetic_tokens(&e);
        let reserve_b = get_reserve_b(e);

        let net_base_asset_value = (base_token_supply as i128)
            .safe_mul(e, base_oracle_price as i128)
            .safe_div(e, PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128);

        let net_quote_asset_value = (reserve_b as i128)
            .safe_mul(e, quote_oracle_price as i128)
            .safe_div(e, PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128);

        net_quote_asset_value.safe_sub(e, net_base_asset_value)
    }

    // Gets the current pool price.
    //
    // # Arguments
    //
    // * a_in_b - Should the price be denominated in Token A or B.
    // * in_usd - Should that price be in USD.
    //
    // # Returns
    //
    // The price of the pool as a u128.
    pub fn get_current_price(self, e: &Env, a_in_b: bool, in_usd: bool, now: u64) -> u128 {
        let (reserve_a, reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

        let mut price = 0_u128;

        if reserve_a == 0 || reserve_b == 0 {
            return price;
        }

        if a_in_b {
            // price of 1 A in terms of B
            price = reserve_b.fixed_div_floor(e, &reserve_a, &PRICE_PRECISION);

            if in_usd {
                let quote_oracle_price_data =
                    oracle::get_oracle_price(e, &self.quote_asset_id, false, now);
                price = price.fixed_mul_floor(
                    e,
                    &(quote_oracle_price_data.price as u128),
                    &PRICE_PRECISION,
                );
            }
        } else {
            // price of 1 B in terms of A
            price = reserve_a.fixed_div_floor(e, &reserve_b, &PRICE_PRECISION);

            if in_usd {
                let base_oracle_price_data =
                    oracle::get_oracle_price(e, &self.base_asset_id, false, now);
                price = price.fixed_mul_floor(
                    e,
                    &(base_oracle_price_data.price as u128),
                    &PRICE_PRECISION,
                );
            }
        }
        price
    }

    pub fn get_delta_a(&self, e: &Env, now: u64) -> i128 {
        let (reserve_a, reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

        let target_price = oracle::get_target_oracle_price(e, &self, now);
        let target_reserve_a = reserve_b.fixed_div_floor(e, &target_price, &PRICE_PRECISION);
        let delta_a = (target_reserve_a as i128)
            .checked_sub(reserve_a as i128)
            .unwrap();

        delta_a
    }

    pub fn get_max_confidence_interval_multiplier(&self) -> u64 {
        // assuming validity_guard_rails max confidence pct is 2%
        match self.tier {
            PoolTier::A => 1,                  // 2%
            PoolTier::B => 1,                  // 2%
            PoolTier::C => 2,                  // 4%
            PoolTier::Speculative => 10,       // 20%
            PoolTier::HighlySpeculative => 50, // 100%
            PoolTier::Isolated => 50,          // 100%
        }
    }

    pub fn get_sanitize_clamp_denominator(&self) -> Option<i64> {
        match self.tier {
            PoolTier::A => Some(10_i64),         // 10%
            PoolTier::B => Some(5_i64),          // 20%
            PoolTier::C => Some(2_i64),          // 50%
            PoolTier::Speculative => None,       // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
            PoolTier::HighlySpeculative => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
            PoolTier::Isolated => None,          // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
        }
    }

    // Mints or burns token_a to re-peg the pool's price to it's oracle price.
    //
    // # Arguments
    //
    // * `now` - The current timestamp.
    pub fn rebalance(&self, e: &Env, now: u64) {
        let reserve_a = get_reserve_a(&e);

        // Find the ideal reserve_a amount such that the pool's price is equal to the oracle price
        let delta_a = self.get_delta_a(e, now);

        if delta_a > 0 {
            mint_synthetic_tokens(&e, &e.current_contract_address(), delta_a);
            put_reserve_a(&e, reserve_a + (delta_a as u128));
        }
        if delta_a < 0 {
            burn_synthetic_tokens(&e, &e.current_contract_address(), delta_a.abs() as u128);
            put_reserve_a(&e, reserve_a - (delta_a.abs() as u128));
        }

        LiquidityPoolEvents::new(&e).rebalance(delta_a, now);
    }

    pub fn get_amount_out(
        &self,
        e: &Env,
        in_amount: u128,
        reserve_sell: u128,
        reserve_buy: u128,
    ) -> (u128, u128) {
        if in_amount == 0 {
            return (0, 0);
        }

        // in * reserve_buy / (reserve_sell + in) - fee
        let result = in_amount.fixed_mul_floor(&e, &reserve_buy, &(reserve_sell + in_amount));
        let fee = result.fixed_mul_ceil(&e, &(self.fee_fraction as u128), &FEE_MULTIPLIER);
        (result - fee, fee)
    }

    pub fn get_amount_out_strict_receive(
        &self,
        e: &Env,
        out_amount: u128,
        reserve_sell: u128,
        reserve_buy: u128,
    ) -> (u128, u128) {
        if out_amount == 0 {
            return (0, 0);
        }

        let dy_w_fee = out_amount.fixed_mul_ceil(
            &e,
            &FEE_MULTIPLIER,
            &(FEE_MULTIPLIER - (self.fee_fraction as u128)),
        );
        // if total value including fee is more than the reserve, math can't be done properly
        if dy_w_fee >= reserve_buy {
            panic_with_error!(e, PoolValidationError::InsufficientBalance);
        }
        // +1 just in case there were some rounding errors & convert to real units in place
        let result = reserve_buy.fixed_mul_floor(&e, &reserve_sell, &(reserve_buy - dy_w_fee))
            - reserve_sell
            + 1;
        (result, dy_w_fee - out_amount)
    }
}

#[contracttype]
#[derive(Default, Clone, Eq, PartialEq, Debug)]
pub struct InsuranceClaim {
    // The amount of revenue last settled
    // Positive if funds left the pool,
    // negative if funds were pulled into the pool
    // precision: QUOTE_PRECISION
    pub rev_withdraw_since_last_settle: i64,
    // The max amount of insurance that the pool can use to resolve liquidity deficits
    // precision: QUOTE_PRECISION
    pub quote_max_insurance: u64,
    // The amount of insurance that has been used to resolve liquidity deficits
    // precision: QUOTE_PRECISION
    pub quote_settled_insurance: u64,
    // The last time revenue was settled in/out of the pool
    pub last_revenue_withdraw_ts: u64,
}
