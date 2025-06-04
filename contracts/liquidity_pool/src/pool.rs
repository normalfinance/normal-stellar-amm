use crate::errors::LiquidityPoolError;
use crate::oracle;
use crate::events::Events as PoolEvents;
use crate::events::LiquidityPoolEvents;
use crate::storage::get_historical_oracle_data;
use crate::storage::{ get_reserve_a, get_reserve_b, put_reserve_a };
use crate::{ errors::LiquidityPoolValidationError };
use sep_40_oracle::Asset;
use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::contracttype;
use soroban_sdk::Address;
use soroban_sdk::{ panic_with_error, Env };

use token_synthetic::{ burn_synthetic, mint_synthetic };
use utils::constant::{ FEE_MULTIPLIER, PRICE_PRECISION };
use utils::oracle::oracle_validity;
use utils::oracle::OracleGuardRails;
use utils::oracle::OracleValidity;
use utils::storage::PoolStatus;
use utils::storage::PoolTier;

#[contracttype]
#[derive(Clone)]
pub struct Pool {
    pub asset: Address,

    pub token_a: Address,
    pub token_b: Address,

    pub tier: PoolTier,
    pub status: PoolStatus,

    pub fee_fraction: u32, // 1 = 0.01%

    // Oracle address for the base (synthetic) asset (i.e. nBTC)
    pub base_oracle: AssetId,
    // Oracle address for the quote asset (TokenB) - usually XLM or USDC
    pub quote_oracle: AssetId,

    // The pool's claim on the insurance fund
    pub insurance_claim: InsuranceClaim,

    pub expiry_ts: u64,
    pub expiry_price: u128,
}

impl Pool {
    pub fn calculate_net_reserve_imbalance(&self, oracle_price: i64) -> i128 {
        validate!(oracle_price > 0, ErrorCode::InvalidOracle, "oracle_price <= 0")?;

        let net_user_base_asset_value = amm.base_asset_amount_with_amm
            .safe_add(amm.base_asset_amount_with_unsettled_lp)?
            .safe_mul(oracle_price.cast()?)?
            .safe_div(PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO.cast()?)?;

        net_user_base_asset_value.safe_add(calculate_net_user_cost_basis(amm)?)
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
    pub fn get_current_price(self, e: &Env, a_in_b: bool, in_usd: bool) -> u128 {
        let (reserve_a, reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

        let mut price = 0_u128;

        if reserve_a == 0 || reserve_b == 0 {
            return price;
        }

        if a_in_b {
            // price of 1 A in terms of B
            price = reserve_b.fixed_div_floor(e, &reserve_a, &PRICE_PRECISION);

            if in_usd {
                let quote_oracle_price_data = oracle::get_quote_oracle_price(e, &self);
                price = price.fixed_mul_floor(
                    e,
                    &(quote_oracle_price_data.price as u128),
                    &PRICE_PRECISION
                );
            }
        } else {
            // price of 1 B in terms of A
            price = reserve_a.fixed_div_floor(e, &reserve_b, &PRICE_PRECISION);

            if in_usd {
                let base_oracle_price_data = oracle::get_base_oracle_price(e, &self);
                price = price.fixed_mul_floor(
                    e,
                    &(base_oracle_price_data.price as u128),
                    &PRICE_PRECISION
                );
            }
        }
        price
    }

    pub fn get_delta_a(&self, e: &Env) -> i128 {
        let (reserve_a, reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

        let target_price = oracle::get_target_oracle_price(e, &self);
        let target_reserve_a = reserve_b.fixed_div_floor(e, &target_price, &PRICE_PRECISION);
        let delta_a = (target_reserve_a as i128).checked_sub(reserve_a as i128).unwrap();

        delta_a
    }

    pub fn get_max_confidence_interval_multiplier(&self) -> u64 {
        // assuming validity_guard_rails max confidence pct is 2%
        match self.tier {
            PoolTier::A => 1, // 2%
            PoolTier::B => 1, // 2%
            PoolTier::C => 2, // 4%
            PoolTier::Speculative => 10, // 20%
            PoolTier::HighlySpeculative => 50, // 100%
            PoolTier::Isolated => 50, // 100%
        }
    }

    pub fn get_sanitize_clamp_denominator(&self) -> Option<i64> {
        match self.tier {
            PoolTier::A => Some(10_i64), // 10%
            PoolTier::B => Some(5_i64), // 20%
            PoolTier::C => Some(2_i64), // 50%
            PoolTier::Speculative => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
            PoolTier::HighlySpeculative => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
            PoolTier::Isolated => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
        }
    }

    // Initializes the liquidity pool.
    //
    // # Arguments
    //
    // * `admin` - The address of the admin user.
    //
    // # Returns
    //
    // The type of the pool as a Symbol.
    pub fn rebalance(&self, e: &Env) {
        let reserve_a = get_reserve_a(&e);

        // Find the ideal reserve_a amount such that the pool's price is equal to the oracle price
        let delta_a = self.get_delta_a(e);

        if delta_a > 0 {
            mint_synthetic(&e, &e.current_contract_address(), delta_a);
            put_reserve_a(&e, reserve_a + (delta_a as u128));
        }
        if delta_a < 0 {
            burn_synthetic(&e, &e.current_contract_address(), delta_a.abs() as u128);
            put_reserve_a(&e, reserve_a - (delta_a.abs() as u128));
        }

        let historical_oracle_data = get_historical_oracle_data(&e);
        let oracle_price_data = oracle::get_base_oracle_price(e, &self);

        let oracle_is_valid =
            oracle_validity(
                e,
                e.current_contract_address(),
                historical_oracle_data.last_oracle_price_twap,
                &oracle_price_data,
                &self.oracle_guard_rails.validity,
                self.get_max_confidence_interval_multiplier(),
                true
            ) == OracleValidity::Valid;

        // cannot repeg if oracle is invalid
        if !oracle_is_valid {
            panic_with_error!(e, LiquidityPoolError::InvalidOracle);
        }

        let new_reserve_a = get_reserve_a(&e);

        PoolEvents::new(&e).rebalance(delta_a, new_reserve_a);
    }

    pub fn get_amount_out(
        &self,
        e: &Env,
        in_amount: u128,
        reserve_sell: u128,
        reserve_buy: u128
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
        reserve_buy: u128
    ) -> (u128, u128) {
        if out_amount == 0 {
            return (0, 0);
        }

        let dy_w_fee = out_amount.fixed_mul_ceil(
            &e,
            &FEE_MULTIPLIER,
            &(FEE_MULTIPLIER - (self.fee_fraction as u128))
        );
        // if total value including fee is more than the reserve, math can't be done properly
        if dy_w_fee >= reserve_buy {
            panic_with_error!(e, LiquidityPoolValidationError::InsufficientBalance);
        }
        // +1 just in case there were some rounding errors & convert to real units in place
        let result =
            reserve_buy.fixed_mul_floor(&e, &reserve_sell, &(reserve_buy - dy_w_fee)) -
            reserve_sell +
            1;
        (result, dy_w_fee - out_amount)
    }
}

#[contracttype]
#[derive(Default, Clone, Eq, PartialEq, Debug)]
#[repr(C)]
pub struct InsuranceClaim {
    /// The amount of revenue last settled
    /// Positive if funds left the pool,
    /// negative if funds were pulled into the pool
    /// precision: QUOTE_PRECISION
    pub rev_withdraw_since_last_settle: i64,
    /// The max amount of insurance that the pool can use to resolve liquidity deficits
    /// precision: QUOTE_PRECISION
    pub quote_max_insurance: u64,
    /// The amount of insurance that has been used to resolve liquidity deficits
    /// precision: QUOTE_PRECISION
    pub quote_settled_insurance: u64,
    /// The last time revenue was settled in/out of the pool
    pub last_revenue_withdraw_ts: i64,
}
