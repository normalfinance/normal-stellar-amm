use core::cmp::max;

use soroban_sdk::Env;

use super::safe_math::SafeMath;

pub fn calculate_rolling_sum(
    e: &Env,
    data1: u64,
    data2: u64,
    weight1_numer: i64,
    weight1_denom: i64
) -> u64 {
    // assumes that missing times are zeros (e.g. handle NaN as 0)
    let prev_twap_99 = (data1 as u128)
        .safe_mul(e, max(0, weight1_denom.safe_sub(e, weight1_numer) as u128))
        .safe_div(e, weight1_denom as u128);

    (prev_twap_99 as u64).safe_add(e, data2)
}

pub fn calculate_weighted_average(
    e: &Env,
    data1: i128,
    data2: i128,
    weight1: i64,
    weight2: i64
) -> i128 {
    let denominator = weight1.safe_add(e, weight2) as i128;
    let prev_twap_99 = data1.safe_mul(e, weight1 as i128);
    let latest_price_01 = data2.safe_mul(e, weight2 as i128);

    if weight1 == 0 {
        return data2;
    }

    if weight2 == 0 {
        return data1;
    }

    let bias: i128 = if weight2 > 1 {
        if latest_price_01 < prev_twap_99 {
            -1
        } else if latest_price_01 > prev_twap_99 {
            1
        } else {
            0
        }
    } else {
        0
    };

    let twap = prev_twap_99.safe_add(e, latest_price_01).safe_div(e, denominator);

    if twap == 0 && bias < 0 {
        return twap;
    }

    twap.safe_add(e, bias)
}

pub fn calculate_new_twap(
    e: &Env,
    current_price: i128,
    current_ts: i64,
    last_twap: i128,
    last_ts: i64,
    period: i64
) -> i128 {
    let since_last = max(0_i64, current_ts.safe_sub(e, last_ts));
    let from_start = max(1_i64, period.safe_sub(e, since_last));

    calculate_weighted_average(e, current_price, last_twap, since_last, from_start)
}
