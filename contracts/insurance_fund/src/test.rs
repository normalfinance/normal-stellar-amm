#![cfg(test)]
extern crate std;

use crate::testutils::{Setup, TestConfig};
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Events};
use soroban_sdk::{vec, Address, Error, IntoVal, Symbol, Val, Vec};
use utils::test_utils::jump;

// from drift

// #[test]
// fn basic_stake_if_test() {
//     let setup = Setup::default();

//     assert_eq!((0_i32).signum(), 0);
//     assert_eq!((1_i32).signum(), 1);
//     assert_eq!(-(1_i32).signum(), -1);

//     assert_eq!((0_i128).signum(), 0);
//     assert_eq!((1_i128).signum(), 1);

//     let mut if_balance = 0;

//     let mut if_stake = Stake::new(0);

//     let amount = QUOTE_PRECISION as u64; // $1
//     let mut insurance_fund = InsuranceFund {
//         unstaking_period: 0,
//         ..InsuranceFund::default()
//     };

//     setup.contract.deposit(user, amount);
//     add_insurance_fund_stake(
//         amount,
//         if_balance,
//         &mut if_stake,
//         &mut user_stats,
//         &mut spot_market,
//         0
//     ).unwrap();

//     assert_eq!(if_stake.unchecked_if_shares(), amount as u128);
//     if_balance += amount;

//     // must request first
//     assert!(
//         remove_insurance_fund_stake(
//             if_balance,
//             &mut if_stake,
//             &mut user_stats,
//             &mut spot_market,
//             0
//         ).is_err()
//     );

//     assert_eq!(if_stake.unchecked_if_shares(), amount as u128);
//     assert_eq!(spot_market.insurance_fund.total_shares, amount as u128);
//     assert_eq!(spot_market.insurance_fund.shares_base, 0);

//     request_remove_insurance_fund_stake(
//         if_stake.unchecked_if_shares(),
//         if_balance,
//         &mut if_stake,
//         &mut user_stats,
//         &mut spot_market,
//         0
//     ).unwrap();
//     assert_eq!(if_stake.last_withdraw_request_shares, if_stake.unchecked_if_shares());
//     assert_eq!(if_stake.last_withdraw_request_value, if_balance - 1); //rounding in favor
//     assert_eq!(if_stake.unchecked_if_shares(), amount as u128);
//     assert_eq!(spot_market.insurance_fund.total_shares, amount as u128);
//     assert_eq!(spot_market.insurance_fund.shares_base, 0);

//     let amount_returned = remove_insurance_fund_stake(
//         if_balance,
//         &mut if_stake,
//         &mut user_stats,
//         &mut spot_market,
//         0
//     ).unwrap();
//     assert_eq!(amount_returned, amount - 1);
//     if_balance -= amount_returned;

//     assert_eq!(if_stake.unchecked_if_shares(), 0);
//     assert_eq!(if_stake.cost_basis, 1);
//     assert_eq!(if_stake.last_withdraw_request_shares, 0);
//     assert_eq!(if_stake.last_withdraw_request_value, 0);
//     assert_eq!(spot_market.insurance_fund.total_shares, 0);
//     assert_eq!(spot_market.insurance_fund.shares_base, 0);
//     assert_eq!(if_balance, 1);

//     add_insurance_fund_stake(
//         1234,
//         if_balance,
//         &mut if_stake,
//         &mut user_stats,
//         &mut spot_market,
//         0
//     ).unwrap();
//     assert_eq!(if_stake.cost_basis, 1234);
//     assert_eq!(spot_market.insurance_fund.user_shares, 1234);
//     assert_eq!(spot_market.insurance_fund.total_shares, 1235); // protocol claims the 1 balance
//     assert_eq!(spot_market.insurance_fund.shares_base, 0);
// }

// custom

#[test]
fn test_deposit() {
    let setup = Setup::default();

    let user = setup.users[0];
    let amount_to_deposit = 100_0000000_u128;

    setup
        .token_a_admin_client
        .mint(&user, &(amount_to_deposit as i128));

    setup.contract.deposit(&user, &amount_to_deposit);

    // Token was transferred from user to Insurance Fund
    assert_eq!(setup.token_a.balance(&user), 0);
    assert_eq!(
        setup.token_a.balance(&setup.contract.address),
        amount_to_deposit as i128
    );

    // Insurance Fund issued shares
    assert_eq!(setup.contract.get_, 0);
}

#[test]
fn test_request_withdraw() {
    let setup = Setup::default();
}

#[test]
fn test_cancel_request_withdraw() {
    let setup = Setup::default();
}

#[test]
fn test_withdraw() {
    let setup = Setup::default();
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_deposit_during_withdraw() {
    let setup = Setup::default();
    let user = Address::generate(&setup.env);

    setup
        .contract
        .deposit(&user, &setup.token_a.address, &100_0000000_u128);
}

#[test]
#[should_panic(expected = "Error(Contract, #16)")]
fn test_request_withdraw_during_unstaking_period() {
    let setup = Setup::default();
    let user = Address::generate(&setup.env);

    setup
        .contract
        .deposit(&user, &setup.token_a.address, &100_0000000_u128);
}

// paused ops

#[test]
fn test_deposit_killed() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            mint_to_user: i128::MAX,
            ..TestConfig::default()
        }),
    );
    let e = setup.env;
    let insurance_fund = setup.contract;
    let users = setup.users;

    assert_eq!(insurance_fund.get_is_killed_deposit(), false);
    assert_eq!(insurance_fund.get_is_killed_request_payout(), false);

    let admin = users[0].clone();

    insurance_fund.kill_deposit(&admin);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&e, "kill_deposit"),).into_val(&e),
                Val::VOID.into_val(&e),
            )
        ]
    );
    assert_eq!(insurance_fund.get_is_killed_deposit(), true);
    assert_eq!(insurance_fund.get_is_killed_request_payout(), false);

    let user1 = users[1].clone();
    let desired_amount = 1_0000000;

    assert_eq!(
        insurance_fund
            .try_deposit(&user1, &desired_amount)
            .unwrap_err(),
        Ok(Error::from_contract_error(205))
    );

    insurance_fund.unkill_deposit(&admin);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&e, "unkill_deposit"),).into_val(&e),
                Val::VOID.into_val(&e),
            )
        ]
    );
    assert_eq!(insurance_fund.get_is_killed_deposit(), false);
    assert_eq!(insurance_fund.get_is_killed_request_payout(), false);

    insurance_fund.deposit(&user1, &desired_amount);
}

#[test]
fn test_request_payout_killed() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            mint_to_user: i128::MAX,
            ..TestConfig::default()
        }),
    );
    let e = setup.env;
    let insurance_fund = setup.contract;
    let users = setup.users;

    assert_eq!(insurance_fund.get_is_killed_deposit(), false);
    assert_eq!(insurance_fund.get_is_killed_request_payout(), false);

    let admin = users[0].clone();

    insurance_fund.kill_request_payout(&admin);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&e, "kill_request_payout"),).into_val(&e),
                Val::VOID.into_val(&e),
            )
        ]
    );
    assert_eq!(insurance_fund.get_is_killed_deposit(), false);
    assert_eq!(insurance_fund.get_is_killed_request_payout(), true);

    let user1 = users[1].clone();
    let desired_amount = 1_0000000;

    assert_eq!(
        insurance_fund
            .try_request_payout(&user1, &desired_amount)
            .unwrap_err(),
        Ok(Error::from_contract_error(209))
    );

    insurance_fund.unkill_request_payout(&admin);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&e, "unkill_request_payout"),).into_val(&e),
                Val::VOID.into_val(&e),
            )
        ]
    );
    assert_eq!(insurance_fund.get_is_killed_deposit(), false);
    assert_eq!(insurance_fund.get_is_killed_request_payout(), false);

    insurance_fund.deposit(&user1, &desired_amount);
}

#[test]
fn test_kill_deposit_event() {
    let setup = Setup::default();
    let insurance_fund = setup.contract;

    insurance_fund.kill_deposit(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&setup.env, "kill_deposit"),).into_val(&setup.env),
                ().into_val(&setup.env),
            )
        ]
    );
}

#[test]
fn test_kill_request_payout_event() {
    let setup = Setup::default();
    let insurance_fund = setup.contract;

    insurance_fund.kill_request_payout(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&setup.env, "kill_request_payout"),).into_val(&setup.env),
                ().into_val(&setup.env),
            )
        ]
    );
}

#[test]
fn test_unkill_deposit_event() {
    let setup = Setup::default();
    let insurance_fund = setup.contract;

    insurance_fund.unkill_deposit(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&setup.env, "unkill_deposit"),).into_val(&setup.env),
                ().into_val(&setup.env),
            )
        ]
    );
}

#[test]
fn test_unkill_request_payout_event() {
    let setup = Setup::default();
    let insurance_fund = setup.contract;

    insurance_fund.unkill_request_payout(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&setup.env, "unkill_request_payout"),).into_val(&setup.env),
                ().into_val(&setup.env),
            )
        ]
    );
}
