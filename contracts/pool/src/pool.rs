use core::cmp::max;

use crate::errors::PoolError;
use crate::errors::PoolValidationError;
use crate::events::Events as LiquidityPoolEvents;
use crate::events::PoolEvents;
use crate::storage::get_last_oracle_valid;
use crate::storage::get_last_trade_ts;
use crate::storage::get_last_update_ts;
use crate::storage::get_oracle_registry;
use crate::storage::get_volume_24h;
use crate::storage::set_last_trade_ts;
use crate::storage::set_volume_24h;
use crate::storage::{ get_reserve_a, get_reserve_b, put_reserve_a };
use pool_tokens::{ burn_synthetic_tokens, get_total_synthetic_tokens, mint_synthetic_tokens };
use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::contracttype;
use soroban_sdk::Address;
use soroban_sdk::IntoVal;
use soroban_sdk::Symbol;
use soroban_sdk::Vec;
use soroban_sdk::{ panic_with_error, Env };

use utils::constant::PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO_I128;
use utils::constant::TWENTY_FOUR_HOUR;
use utils::constant::{ FEE_MULTIPLIER, PRICE_PRECISION };
use utils::math::safe_math::SafeMath;
use utils::math::stats::calculate_rolling_sum;
use utils::storage::OraclePriceData;
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
    pub base_asset_id: Symbol,
    // Oracle address for the quote asset (TokenB) - usually XLM or USDC
    pub quote_asset_id: Symbol,
    pub tier: PoolTier,
    pub status: PoolStatus,
    pub fee_fraction: u32, // 1 = 0.01%
    // The pool's claim on the insurance fund
    pub insurance_claim: InsuranceClaim,
    // The max liquidity imbalance before price premiums are added and/or the buffer/if is used
    // liquidity imbalance is the difference between quote token and base token value. When it's less than 0,
    // the pool does not have enough liquidity to fill all orders and will apply a price premium to new swaps.
    // precision = QUOTE_PRECISION
    pub liquidity_max_imbalance: u128,

    // The time the market is set to expire. Only set if market is in reduce only mode
    pub expiry_ts: u64,
}

impl Pool {
    pub fn is_in_settlement(&self, now: u64) -> bool {
        let in_settlement = matches!(self.status, PoolStatus::Settlement | PoolStatus::Delisted);
        let expired = self.expiry_ts != 0 && now >= self.expiry_ts;
        in_settlement || expired
    }

    pub fn is_reduce_only(&self) -> bool {
        self.status == PoolStatus::ReduceOnly
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
    pub fn get_net_liquidity_imbalance(
        &self,
        e: &Env,
        base_oracle_price: u128,
        quote_oracle_price: u128
    ) -> i128 {
        // "base_oracle_price <= 0"
        validate!(e, base_oracle_price > 0, PoolError::InvalidOracle);
        // "quote_oracle_price <= 0"
        validate!(e, quote_oracle_price > 0, PoolError::InvalidOracle);

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

    // pub fn is_price_divergence_ok_for_settle_pnl(&self, oracle_price: i64) -> bool {
    //     let oracle_divergence = oracle_price
    //         .safe_sub(self.amm.historical_oracle_data.last_oracle_price_twap_5min)?
    //         .safe_mul(PERCENTAGE_PRECISION_I64)?
    //         .safe_div(
    //             self.amm.historical_oracle_data.last_oracle_price_twap_5min.min(oracle_price)
    //         )?
    //         .unsigned_abs();

    //     let oracle_divergence_limit = match self.contract_tier {
    //         ContractTier::A => PERCENTAGE_PRECISION_U64 / 200, // 50 bps
    //         ContractTier::B => PERCENTAGE_PRECISION_U64 / 200, // 50 bps
    //         ContractTier::C => PERCENTAGE_PRECISION_U64 / 100, // 100 bps
    //         ContractTier::Speculative => PERCENTAGE_PRECISION_U64 / 40, // 250 bps
    //         ContractTier::HighlySpeculative => PERCENTAGE_PRECISION_U64 / 40, // 250 bps
    //         ContractTier::Isolated => PERCENTAGE_PRECISION_U64 / 40, // 250 bps
    //     };

    //     if oracle_divergence >= oracle_divergence_limit {
    //         msg!(
    //             "market_index={} price divergence too large to safely settle pnl: {} >= {}",
    //             self.market_index,
    //             oracle_divergence,
    //             oracle_divergence_limit
    //         );
    //         return Ok(false);
    //     }

    //     let min_price = oracle_price.min(
    //         self.amm.historical_oracle_data.last_oracle_price_twap_5min
    //     );

    //     let std_limit = (
    //         match self.contract_tier {
    //             ContractTier::A => min_price / 50, // 200 bps
    //             ContractTier::B => min_price / 50, // 200 bps
    //             ContractTier::C => min_price / 20, // 500 bps
    //             ContractTier::Speculative => min_price / 10, // 1000 bps
    //             ContractTier::HighlySpeculative => min_price / 10, // 1000 bps
    //             ContractTier::Isolated => min_price / 10, // 1000 bps
    //         }
    //     ).unsigned_abs();

    //     if self.amm.oracle_std.max(self.amm.mark_std) >= std_limit {
    //         msg!(
    //             "market_index={} std too large to safely settle pnl: {} >= {}",
    //             self.market_index,
    //             self.amm.oracle_std.max(self.amm.mark_std),
    //             std_limit
    //         );
    //         return Ok(false);
    //     }

    //     Ok(true)
    // }

    pub fn get_oracle_price(&self, e: Env, asset_id: Symbol, now: u64) -> OraclePriceData {
        let oracle_price_data: OraclePriceData = e.invoke_contract(
            &get_oracle_registry(&e),
            &Symbol::new(&e, "get_price"),
            Vec::from_array(&e, [
                e.current_contract_address().to_val(),
                asset_id.to_val(),
                now.into_val(&e),
            ])
        );
        oracle_price_data
    }

    pub fn peg_price(
        &self,
        e: &Env,
        base_oracle_price: u128,
        quote_oracle_price: u128,
        now: u64
    ) -> u128 {
        if base_oracle_price == 0 || quote_oracle_price == 0 {
            return 0;
        }

        quote_oracle_price.fixed_div_floor(e, &base_oracle_price, &PRICE_PRECISION)
    }

    pub fn update_volume_24h(&self, e: &Env, quote_asset_amount: u128, now: u64) {
        let since_last = max(1_u64, now.safe_sub(e, get_last_trade_ts(e)));

        let volume_24h = get_volume_24h(e);

        set_volume_24h(
            e,
            &calculate_rolling_sum(e, volume_24h, quote_asset_amount, since_last, TWENTY_FOUR_HOUR)
        );

        set_last_trade_ts(e, &now);
    }

    pub fn is_recent_oracle_valid(&self, e: &Env, current_ts: u64) -> bool {
        get_last_oracle_valid(e) && current_ts == get_last_update_ts(e)
    }

    pub fn get_delta_a(
        &self,
        e: &Env,
        base_oracle_price: u128,
        quote_oracle_price: u128,
        now: u64
    ) -> i128 {
        let (reserve_a, reserve_b) = (get_reserve_a(e), get_reserve_b(e));

        let peg_price = self.peg_price(e, base_oracle_price, quote_oracle_price, now);
        let target_reserve_a = reserve_b.fixed_div_floor(e, &peg_price, &PRICE_PRECISION);
        let delta_a = (target_reserve_a as i128).checked_sub(reserve_a as i128).unwrap();

        delta_a
    }

    // Mints or burns token_a to re-peg the pool's price to it's oracle price.
    //
    // # Arguments
    //
    // * `now` - The current timestamp.
    pub fn rebalance(&self, e: &Env, base_oracle_price: u128, quote_oracle_price: u128, now: u64) {
        let reserve_a = get_reserve_a(&e);

        // Find the ideal reserve_a amount such that the pool's price is equal to the oracle price
        let delta_a = self.get_delta_a(&e, base_oracle_price, quote_oracle_price, now);

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
            panic_with_error!(e, PoolValidationError::InsufficientBalance);
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
pub struct InsuranceClaim {
    // The amount of revenue last settled
    // Positive if funds left the pool,
    // negative if funds were pulled into the pool
    // precision: QUOTE_PRECISION
    pub rev_withdraw_since_last_settle: i128,
    // The max amount of insurance that the pool can use to resolve liquidity deficits
    // precision: QUOTE_PRECISION
    pub quote_max_insurance: u128,
    // The amount of insurance that has been used to resolve liquidity deficits
    // precision: QUOTE_PRECISION
    pub quote_settled_insurance: u128,
    // The last time revenue was settled in/out of the pool
    pub last_revenue_withdraw_ts: u64,
}
