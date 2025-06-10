#![cfg(test)]

use utils::{ constant::QUOTE_PRECISION, helpers::{ log10, log10_iter } };

use crate::{ stake::{ calculate_if_shares_lost, calculate_rebase_info, Stake }, testutils::Setup };

#[test]
pub fn basic_stake_if_test() {
    let setup = Setup::default();

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 10000, 10000);
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 20_000, 10000);
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 60_078, 10000);
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 60_078, 9999);
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 60_078, 6008);
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 60_078, 6007);
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 60_078, 6006);
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 60_078, 606);
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 60_078, 600);
    assert_eq!(rebase_div, 10);
    assert_eq!(expo_diff, 1);

    let (expo_diff, rebase_div) = calculate_rebase_info(
        &setup.env,
        60_078 * QUOTE_PRECISION,
        600 * QUOTE_PRECISION + 19234
    );
    assert_eq!(rebase_div, 10);
    assert_eq!(expo_diff, 1);

    let (expo_diff, rebase_div) = calculate_rebase_info(
        &setup.env,
        60_078 * QUOTE_PRECISION,
        601 * QUOTE_PRECISION + 19234
    );
    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);

    // $800M goes to 1e-6 of dollar
    let (expo_diff, rebase_div) = calculate_rebase_info(
        &setup.env,
        800_000_078 * QUOTE_PRECISION,
        1_u128
    );

    assert_eq!(rebase_div, 10000000000000);
    assert_eq!(expo_diff, 13);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 99_999, 100);
    assert_eq!(log10(100), 2);
    assert_eq!(log10_iter(100), 2);
    assert_eq!(99_999 / 10 / 100, 99);
    assert_eq!(rebase_div, 10);
    assert_eq!(expo_diff, 1);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 100_000, 100);
    assert_eq!(log10(100), 2);
    assert_eq!(100_000 / 10 / 100, 100);
    assert_eq!(rebase_div, 100);
    assert_eq!(expo_diff, 2);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 100_001, 100);
    assert_eq!(log10(100), 2);
    assert_eq!(100_001 / 10 / 100, 100);
    assert_eq!(rebase_div, 100);
    assert_eq!(expo_diff, 2);

    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 1_242_418_900_000, 1);

    assert_eq!(rebase_div, 100000000000);
    assert_eq!(expo_diff, 11);

    // todo?: does not rebase the other direction (perhaps unnecessary)
    let (expo_diff, rebase_div) = calculate_rebase_info(&setup.env, 12412, 83295723895729080);

    assert_eq!(rebase_div, 1);
    assert_eq!(expo_diff, 0);
}

#[test]
pub fn if_shares_lost_test() {
    let setup = Setup::default();

    let _amount = QUOTE_PRECISION as u64; // $1

    let unstaking_period = 0;
    let mut total_shares = 1000 * QUOTE_PRECISION;

    let mut if_stake = Stake::new(0);
    if_stake.update_if_shares(&setup.env, 100 * QUOTE_PRECISION);
    if_stake.last_withdraw_request_shares = 100 * QUOTE_PRECISION;
    if_stake.last_withdraw_request_value = 100 * QUOTE_PRECISION - 1;

    let if_balance = 1000 * QUOTE_PRECISION;

    // unchanged balance
    let lost_shares = calculate_if_shares_lost(&setup.env, &if_stake, if_balance);
    assert_eq!(lost_shares, 2);

    let if_balance = if_balance + 100 * QUOTE_PRECISION;
    total_shares += 100 * QUOTE_PRECISION;
    let lost_shares = calculate_if_shares_lost(&setup.env, &if_stake, if_balance);
    assert_eq!(lost_shares, 2); // giving up $5 of gains

    let if_balance = if_balance - 100 * QUOTE_PRECISION;
    total_shares -= 100 * QUOTE_PRECISION;
    let lost_shares = calculate_if_shares_lost(&setup.env, &if_stake, if_balance);
    assert_eq!(lost_shares, 2); // giving up $5 of gains

    // take back gain
    let if_balance = 1100 * QUOTE_PRECISION;
    let lost_shares = calculate_if_shares_lost(&setup.env, &if_stake, if_balance);
    assert_eq!(lost_shares, 10_000_001); // giving up $10 of gains

    // doesnt matter if theres a loss
    if_stake.last_withdraw_request_value = 200 * QUOTE_PRECISION;
    let lost_shares = calculate_if_shares_lost(&setup.env, &if_stake, if_balance);
    assert_eq!(lost_shares, 0);
    if_stake.last_withdraw_request_value = 100 * QUOTE_PRECISION - 1;

    // take back gain and total_if_shares alter w/o user alter
    let if_balance = 2100 * QUOTE_PRECISION;
    total_shares *= 2;
    let lost_shares = calculate_if_shares_lost(&setup.env, &if_stake, if_balance);
    assert_eq!(lost_shares, 5_000_001); // giving up $5 of gains

    let if_balance = 2100 * QUOTE_PRECISION * 10;

    let expected_gain_if_no_loss = (if_balance * 100) / 2000;
    assert_eq!(expected_gain_if_no_loss, 1_050_000_000);
    let lost_shares = calculate_if_shares_lost(&setup.env, &if_stake, if_balance);
    assert_eq!(lost_shares, 90_909_092); // giving up $5 of gains
    assert_eq!(
        (9090908 * if_balance) / (total_shares - lost_shares) <
            if_stake.last_withdraw_request_value,
        true
    );
}
