use soroban_sdk::Env;

use crate::constant::DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR;

use super::safe_math::SafeMath;

pub fn sanitize_new_price(
    e: &Env,
    new_price: i128,
    last_price_twap: i128,
    sanitize_clamp_denominator: Option<i64>
) -> i128 {
    // when/if twap is 0, dont try to normalize new_price
    if last_price_twap == 0 {
        return new_price;
    }

    let new_price_spread = new_price.safe_sub(e, last_price_twap);

    // cap new oracle update to 100/MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR% delta from twap
    let sanitize_clamp_denominator = if
        let Some(sanitize_clamp_denominator) = sanitize_clamp_denominator
    {
        sanitize_clamp_denominator
    } else {
        DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
    };

    if sanitize_clamp_denominator == 0 {
        // no need to use price band check
        return new_price;
    }

    let price_twap_price_band = last_price_twap.safe_div(e, sanitize_clamp_denominator as i128);

    let capped_update_price = if
        new_price_spread.unsigned_abs() > price_twap_price_band.unsigned_abs()
    {
        if new_price > last_price_twap {
            last_price_twap.safe_add(e, price_twap_price_band)
        } else {
            last_price_twap.safe_sub(e, price_twap_price_band)
        }
    } else {
        new_price
    };

    capped_update_price
}
