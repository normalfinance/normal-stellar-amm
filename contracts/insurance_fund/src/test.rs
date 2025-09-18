#![cfg(test)]
extern crate std;

use crate::stake::{Stake, StakeAction};
use crate::testutils::{Setup, TestConfig};
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::token::TokenClient;
use soroban_sdk::{vec, Error, IntoVal, Symbol, Val};
use utils::constant::{ONE_HOUR, PRICE_PRECISION, THIRTEEN_DAY, THIRTY_DAY};
// use utils::test_utils::insurance_fund::Stake;
use utils::test_utils::jump;

/* `resolve_liquidity_deficit()` tests are located in /integration_tests since testing this
function can only truly be done setting up all other contracts */

/* Tests Needed:
- [ ] Withdrawing all shares (a minus 1 operation is applied)
- [ ] All operations after premium has been paid
- [ ] Setters work
- [ ] Getters work

 * Deposit Tests
 * [ ] Singular deposit
 * [ ] Multiple deposits, same user
 * [ ] Multiple deposits, different users
 * [ ] Deposit over optimal coverage FAIL 20
 * [ ] Deposit while withdraw in progress FAIL 9
 *
 *  Request Withdraw Tests
 * [ ] happy path
 * [ ] already in progress FAIL 9
 * [ ] empty vault amount FAIL 12 (if already shares)
 * [ ] zero request fails FAIL 11
 * [ ] insufficent user shares FAIL 13
 * [ ] error if too low vault amount FAIL 3
 * [ ]
 */

#[test]
#[should_panic(expected = "Error(Contract, #103)")]
fn test_initialize_twice() {
    let setup = Setup::default();

    setup.insurance_fund.initialize(
        &setup.admin,
        &setup.emergency_admin,
        &setup.oracle_registry,
        &setup.pool_router,
        &setup.token_a.address,
        &THIRTEEN_DAY,
        &80_00000_u32,
        &2_00000_i32,
        &(10_00000_u32, 2_00000_u32),
    );
}

#[test]
fn test_deposit() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            mint_to_user: i128::MAX,
            ..TestConfig::default()
        }),
    );

    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;

    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    // Token was transferred from user to Insurance Fund
    assert_eq!(
        setup.token_a.balance(&users[1]),
        i128::MAX - (amount_to_deposit as i128)
    );
    assert_eq!(
        setup.token_a.balance(&setup.insurance_fund.address),
        amount_to_deposit as i128
    );

    // Insurance Fund issued shares
    assert_eq!(setup.insurance_fund.get_reserve(&setup.token_a.address).total_shares, amount_to_deposit);
    assert_eq!(
        setup.insurance_fund.get_stake(&users[1], &setup.token_a.address),
        Stake {
            cost_basis: amount_to_deposit,
            base: 0,
            shares: amount_to_deposit,
            last_withdraw_request_shares: 0,
            last_withdraw_request_ts: 0,
            last_withdraw_request_value: 0,
            user: users[1].clone(),
            token: setup.token_a.address,
        }
    );
}

#[test]
fn test_deposit_back_to_back() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        }),
    );

    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;
    let amount_to_deposit_2 = 50 * PRICE_PRECISION;

    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);
    setup
        .insurance_fund
        .deposit(&users[1], &setup.token_a.address, &amount_to_deposit_2);

    assert_eq!(
        setup.token_a.balance(&users[1]),
        TestConfig::default().mint_to_user - ((amount_to_deposit + amount_to_deposit_2) as i128)
    );
    assert_eq!(
        setup.token_a.balance(&setup.insurance_fund.address),
        (amount_to_deposit + amount_to_deposit_2) as i128
    );

    // Insurance Fund issued shares
    assert_eq!(setup.insurance_fund.get_reserve(&setup.token_a.address).total_shares, amount_to_deposit);
    assert_eq!(
        setup.insurance_fund.get_stake(&users[1], &setup.token_a.address),
        Stake {
            cost_basis: amount_to_deposit,
            base: 0,
            shares: amount_to_deposit,
            last_withdraw_request_shares: 0,
            last_withdraw_request_ts: 0,
            last_withdraw_request_value: 0,
            user: users[1].clone(),
            token: setup.token_a.address,
        }
    );
}

#[test]
fn test_deposit_from_multiple_users() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        }),
    );

    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;
    let amount_to_deposit_2 = 50 * PRICE_PRECISION;

    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);
    setup
        .insurance_fund
        .deposit(&users[2], &setup.token_a.address, &amount_to_deposit_2);

    // Token was transferred from user to Insurance Fund
    assert_eq!(
        setup.token_a.balance(&users[1]),
        TestConfig::default().mint_to_user - (amount_to_deposit as i128)
    );
    assert_eq!(
        setup.token_a.balance(&users[2]),
        TestConfig::default().mint_to_user - (amount_to_deposit_2 as i128)
    );
    assert_eq!(
        setup.token_a.balance(&setup.insurance_fund.address),
        (amount_to_deposit + amount_to_deposit_2) as i128
    );

    // Insurance Fund issued shares
    assert_eq!(
        setup.insurance_fund.get_reserve(&setup.token_a.address).total_shares,
        amount_to_deposit + amount_to_deposit_2
    );
    assert_eq!(
        setup.insurance_fund.get_stake(&users[1], &setup.token_a.address),
        Stake {
            cost_basis: amount_to_deposit,
            base: 0,
            shares: amount_to_deposit,
            last_withdraw_request_shares: 0,
            last_withdraw_request_ts: 0,
            last_withdraw_request_value: 0,
            user: users[1].clone(),
            token: setup.token_a.address,
        }
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #20)")]
fn test_deposit_over_optimal_insurance() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            mint_to_user: i128::MAX,
            ..TestConfig::default()
        }),
    );

    let users = setup.users;
    let optimal_insurance = 100_0000000_u128;
    let amount_to_deposit = 101_0000000_u128;

    // Update the optimal insurance
    setup
        .insurance_fund
        .set_optimal_insurance(&setup.admin, &optimal_insurance);

    // Attempt the deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_deposit_while_request_withdraw_in_progress() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            mint_to_user: i128::MAX,
            ..TestConfig::default()
        }),
    );

    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;

    // Make initial deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    jump(&setup.env, 10);

    // Request a withdrawal
    setup
        .insurance_fund
        .request_withdraw(&users[1], &setup.token_a.address, &amount_to_deposit);

    // 10 seconds, not enough to pass the unstaking period
    jump(&setup.env, 10);

    // Attempt another deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);
}

#[test]
fn test_request_withdraw() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        }),
    );
    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;
    let amount_to_withdraw = 50 * PRICE_PRECISION;

    // Make initial deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    let stake = setup.insurance_fund.get_stake(&users[1], &setup.token_a.address);

    // Request a withdrawal
    setup
        .insurance_fund
        .request_withdraw(&users[1], &setup.token_a.address, &amount_to_withdraw);

    // Ensure no tokens were transferred
    assert_eq!(
        setup.token_a.balance(&users[1]),
        TestConfig::default().mint_to_user - (amount_to_deposit as i128)
    );
    assert_eq!(
        setup.token_a.balance(&setup.insurance_fund.address),
        amount_to_deposit as i128
    );

    // Ensure user stake was updated
    assert_eq!(
        setup.insurance_fund.get_stake(&users[1], &setup.token_a.address),
        Stake {
            last_withdraw_request_shares: amount_to_withdraw, // n_shares
            last_withdraw_request_ts: setup.env.ledger().timestamp(),
            last_withdraw_request_value: amount_to_withdraw,
            ..stake
        }
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_request_withdraw_while_in_progress() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        }),
    );
    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;

    // Make initial deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    jump(&setup.env, 10);

    // Request a withdrawal
    setup
        .insurance_fund
        .request_withdraw(&users[1], &setup.token_a.address, &amount_to_deposit);

    // 10 seconds, not enough to pass the unstaking period
    jump(&setup.env, 10);

    // Attempt another request withdraw
    setup
        .insurance_fund
        .request_withdraw(&users[1], &setup.token_a.address, &amount_to_deposit);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn test_request_withdraw_with_empty_vault() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        }),
    );
    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;

    // Make initial deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    // Burn tokens from Insurance Fund
    TokenClient::new(&setup.env, &setup.token_a.address)
        .burn(&setup.insurance_fund.address, &(amount_to_deposit as i128));

    // Request a withdrawal
    setup
        .insurance_fund
        .request_withdraw(&users[1], &setup.token_a.address, &amount_to_deposit);
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")]
fn test_request_withdraw_with_zero_amount() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        }),
    );
    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;

    // Make initial deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    // Request a withdrawal
    setup.insurance_fund.request_withdraw(&users[1], &setup.token_a.address, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_request_withdraw_with_insufficient_shares() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        }),
    );
    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;

    // Make initial deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    // Request a withdrawal
    setup
        .insurance_fund
        .request_withdraw(&users[1], &setup.token_a.address, &(amount_to_deposit + 10_0000000_u128));
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_request_withdraw_with_insufficient_vault_amount() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        }),
    );
    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;
    let amount_to_burn = 20 * PRICE_PRECISION;
    let amount_to_withdraw = 50 * PRICE_PRECISION;

    // Make initial deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    // Burn some tokens from Insurance Fund
    TokenClient::new(&setup.env, &setup.token_a.address)
        .burn(&setup.insurance_fund.address, &(amount_to_burn as i128));
    assert_eq!(
        setup.token_a.balance(&setup.insurance_fund.address),
        (amount_to_deposit - amount_to_burn) as i128
    );

    // Request a withdrawal
    setup
        .insurance_fund
        .request_withdraw(&users[1], &setup.token_a.address, &amount_to_withdraw);
}

#[test]
fn test_cancel_request_withdraw() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        }),
    );
    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;
    let amount_to_withdraw = 50 * PRICE_PRECISION;

    // Make initial deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    // Request a withdrawal
    setup
        .insurance_fund
        .request_withdraw(&users[1], &setup.token_a.address, &amount_to_withdraw);

    let stake = setup.insurance_fund.get_stake(&users[1], &setup.token_a.address);

    // Cancel withdrawal request
    setup.insurance_fund.cancel_request_withdraw(&users[1], &setup.token_a.address);

    assert_eq!(setup.insurance_fund.get_reserve(&setup.token_a.address).total_shares, amount_to_deposit);
    assert_eq!(
        setup.insurance_fund.get_stake(&users[1], &setup.token_a.address),
        Stake {
            shares: stake.shares - 0,
            last_withdraw_request_shares: 0,
            last_withdraw_request_ts: setup.env.ledger().timestamp(),
            last_withdraw_request_value: 0,
            ..stake
        }
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_cancel_request_withdraw_no_request_in_progress() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        }),
    );
    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;

    // Make initial deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    // Cancel withdrawal request
    setup.insurance_fund.cancel_request_withdraw(&users[1], &setup.token_a.address);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_cancel_request_withdraw_invalid_rebase() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        }),
    );
    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;

    // Make initial deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    // Request a withdrawal
    setup
        .insurance_fund
        .request_withdraw(&users[1], &setup.token_a.address, &amount_to_deposit);

    // TODO: overwrite the if_base
    // setup.insurance_fund.set

    // Cancel withdrawal request
    setup.insurance_fund.cancel_request_withdraw(&users[1], &setup.token_a.address);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn test_cancel_request_withdraw_increasing_shares() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        }),
    );
    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;

    // Make initial deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    // Request a withdrawal
    setup
        .insurance_fund
        .request_withdraw(&users[1], &setup.token_a.address, &amount_to_deposit);

    // Mint tokens to the Insurance Fund to skew the users new share amount
    setup
        .token_a_admin_client
        .mint(&setup.insurance_fund.address, &50_0000000_i128);

    // Cancel withdrawal request
    setup.insurance_fund.cancel_request_withdraw(&users[1], &setup.token_a.address);
}

#[test]
fn test_withdraw() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        }),
    );
    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;
    let amount_to_withdraw = 50 * PRICE_PRECISION;

    // Make initial deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    // Request a withdrawal
    setup
        .insurance_fund
        .request_withdraw(&users[1], &setup.token_a.address, &amount_to_withdraw);

    // Get pre-withdrawal values
    let total_shares = setup.insurance_fund.get_reserve(&setup.token_a.address).total_shares;
    let stake = setup.insurance_fund.get_stake(&users[1], &setup.token_a.address);

    // Simulate unstaking period
    let unstaking_period = setup.insurance_fund.get_unstaking_period();
    jump(&setup.env, unstaking_period + ONE_HOUR);

    // Withdraw
    setup.insurance_fund.withdraw(&users[1], &setup.token_a.address);

    assert_eq!(
        setup.insurance_fund.get_reserve(&setup.token_a.address).total_shares,
        total_shares - amount_to_withdraw
    );
    assert_eq!(
        setup.insurance_fund.get_stake(&users[1], &setup.token_a.address),
        Stake {
            cost_basis: stake.cost_basis - 1,
            shares: 0,
            last_withdraw_request_shares: 0,
            last_withdraw_request_ts: setup.env.ledger().timestamp(),
            last_withdraw_request_value: 0,
            ..stake
        }
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #14)")]
fn test_withdraw_during_unstaking_period() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            mint_to_user: i128::MAX,
            ..TestConfig::default()
        }),
    );
    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;
    let amount_to_withdraw = 50 * PRICE_PRECISION;

    // Make initial deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    // Request a withdrawal
    setup
        .insurance_fund
        .request_withdraw(&users[1], &setup.token_a.address, &amount_to_withdraw);

    // Simulate unstaking period less ONE_HOUR
    let unstaking_period = setup.insurance_fund.get_unstaking_period();
    jump(&setup.env, unstaking_period - ONE_HOUR);

    // Withdraw
    setup.insurance_fund.withdraw(&users[1], &setup.token_a.address);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
// let n_shares = stake.last_withdraw_request_shares; must be postive FAIL 2
fn test_withdraw_without_prior_request() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            mint_to_user: i128::MAX,
            ..TestConfig::default()
        }),
    );
    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;

    // Make initial deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    // Simulate unstaking period
    let unstaking_period = setup.insurance_fund.get_unstaking_period();
    jump(&setup.env, unstaking_period + ONE_HOUR);

    // Withdraw
    setup.insurance_fund.withdraw(&users[1], &setup.token_a.address);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
//MUST if_shares_before >= n_shares FAIL 13
fn test_withdraw_not_decrease_shares() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            mint_to_user: i128::MAX,
            ..TestConfig::default()
        }),
    );
    let users = setup.users;
    let amount_to_deposit = 100 * PRICE_PRECISION;

    // Make initial deposit
    setup.insurance_fund.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    // Request a withdrawal
    setup
        .insurance_fund
        .request_withdraw(&users[1], &setup.token_a.address, &amount_to_deposit);

    // Simulate unstaking period
    let unstaking_period = setup.insurance_fund.get_unstaking_period();
    jump(&setup.env, unstaking_period + ONE_HOUR);

    // TODO: idk what to change here for the failure

    // Withdraw
    setup.insurance_fund.withdraw(&users[1], &setup.token_a.address);
}

#[test]
fn test_pay_premium() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        }),
    );
    let users = setup.users;
    let staker = users[1].clone();
    let payer = users[2].clone();
    let amount_to_deposit = 100 * PRICE_PRECISION;
    let amount_to_pay = 10 * PRICE_PRECISION;

    // Deposit
    setup.insurance_fund.deposit(&staker, &setup.token_a.address, &amount_to_deposit);

    // Collect pre-pay values
    let if_balance = setup.token_a.balance(&setup.insurance_fund.address);
    let total_shares = setup.insurance_fund.get_reserve(&setup.token_a.address).total_shares;

    // Pay premium
    setup.insurance_fund.pay_premium(&payer, &amount_to_pay);

    // [x] Ensure token is transferred from payer to IF
    assert_eq!(
        setup.token_a.balance(&payer),
        TestConfig::default().mint_to_user - (amount_to_pay as i128)
    );
    assert_eq!(
        setup.token_a.balance(&setup.insurance_fund.address),
        if_balance + (amount_to_pay as i128)
    );

    // [x] Ensure total_shares is unchanged - so LPs accrue interest value
    assert_eq!(setup.insurance_fund.get_reserve(&setup.token_a.address).total_shares, total_shares);  

    // [ ] Ensure staker received portion of premium when withdrawing
    setup
        .insurance_fund
        .request_withdraw(&staker, &setup.token_a.address, &amount_to_deposit);
    let unstaking_period = setup.insurance_fund.get_unstaking_period();
    jump(&setup.env, unstaking_period + ONE_HOUR);
    let staker_balance = setup.token_a.balance(&staker);
    setup.insurance_fund.withdraw(&staker, &setup.token_a.address);
    assert_eq!(
        setup.token_a.balance(&staker),
        staker_balance + (amount_to_pay as i128)
    )
}

#[test]
fn test_events() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        }),
    );
    let e = setup.env;
    let insurance_fund = setup.insurance_fund;
    let token1 = &setup.token_a;
    let user1 = setup.users[1].clone();
    let amount_to_deposit = 100 * PRICE_PRECISION;
    let amount_to_withdraw = 50_0000000_u128;
    let amount_to_pay = 25_0000000_u128;

    // mint
    setup
        .token_a_admin_client
        .mint(&user1, &(amount_to_deposit as i128));

    // deposit
    insurance_fund.deposit(&user1, &setup.token_a.address, &amount_to_deposit);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                insurance_fund.address.clone(),
                (
                    Symbol::new(&e, "if_stake_record"),
                    user1.clone(),
                    StakeAction::Deposit
                )
                    .into_val(&e),
                amount_to_deposit.into_val(&e),
            )
        ]
    );

    // request_withdraw
    insurance_fund.request_withdraw(&user1, &setup.token_a.address, &amount_to_withdraw);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                insurance_fund.address.clone(),
                (
                    Symbol::new(&e, "if_stake_record"),
                    user1.clone(),
                    StakeAction::WithdrawRequest,
                )
                    .into_val(&e),
                amount_to_withdraw.into_val(&e),
            )
        ]
    );

    // cancel_request_withdraw
    insurance_fund.cancel_request_withdraw(&user1, &setup.token_a.address);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                insurance_fund.address.clone(),
                (
                    Symbol::new(&e, "if_stake_record"),
                    user1.clone(),
                    StakeAction::WithdrawCancelRequest,
                )
                    .into_val(&e),
                amount_to_withdraw.into_val(&e),
            )
        ]
    );

    // withdraw
    insurance_fund.request_withdraw(&user1, &setup.token_a.address, &amount_to_withdraw);
    jump(&e, THIRTEEN_DAY + ONE_HOUR);
    insurance_fund.withdraw(&user1, &setup.token_a.address); 
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                insurance_fund.address.clone(),
                (
                    Symbol::new(&e, "if_stake_record"),
                    user1.clone(),
                    StakeAction::Withdraw
                )
                    .into_val(&e),
                amount_to_withdraw.into_val(&e),
            )
        ]
    );

    // pay premium
    setup
        .token_a_admin_client
        .mint(&user1, &(amount_to_pay as i128));
    insurance_fund.pay_premium(&user1, &amount_to_pay);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&e, "collect_premium"), user1.clone()).into_val(&e),
                amount_to_pay.into_val(&e),
            )
        ]
    );
}

//    _______     __       ____  ____   ________  _______  ________
//   |   __ "\   /""\     ("  _||_ " | /"       )/"     "||"      "\
//   (. |__) :) /    \    |   (  ) : |(:   \___/(: ______)(.  ___  :)
//   |:  ____/ /' /\  \   (:  |  | . ) \___  \   \/    |  |: \   ) ||
//   (|  /    //  __'  \   \\ \__/ //   __/  \\  // ___)_ (| (___\ ||
//  /|__/ \  /   /  \\  \  /\\ __ //\  /" \   :)(:      "||:       :)
// (_______)(___/    \___)(__________)(_______/  \_______)(________/

#[test]
fn test_deposit_killed() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            mint_to_user: i128::MAX,
            ..TestConfig::default()
        }),
    );
    let e = setup.env;
    let insurance_fund = setup.insurance_fund;
    let users = setup.users;

    assert_eq!(insurance_fund.get_is_killed_deposit(), false);
    assert_eq!(insurance_fund.get_is_killed_request_withdraw(), false);
    assert_eq!(insurance_fund.get_is_killed_withdraw(), false);

    let admin = setup.admin;

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
    assert_eq!(insurance_fund.get_is_killed_request_withdraw(), false);
    assert_eq!(insurance_fund.get_is_killed_withdraw(), false);

    let user1 = users[1].clone();
    let desired_amount = 1_0000000;

    assert_eq!(
        insurance_fund
            .try_deposit(&user1, &setup.token_a.address, &desired_amount)
            .unwrap_err(),
        Ok(Error::from_contract_error(30))
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
    assert_eq!(insurance_fund.get_is_killed_request_withdraw(), false);
    assert_eq!(insurance_fund.get_is_killed_withdraw(), false);

    insurance_fund.deposit(&user1, &setup.token_a.address, &desired_amount);
}

#[test]
fn test_request_withdraw_killed() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            mint_to_user: i128::MAX,
            ..TestConfig::default()
        }),
    );
    let e = setup.env;
    let insurance_fund = setup.insurance_fund;
    let users = setup.users;

    assert_eq!(insurance_fund.get_is_killed_deposit(), false);
    assert_eq!(insurance_fund.get_is_killed_request_withdraw(), false);
    assert_eq!(insurance_fund.get_is_killed_withdraw(), false);

    let admin = setup.admin;

    insurance_fund.kill_request_withdraw(&admin);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&e, "kill_request_withdraw"),).into_val(&e),
                Val::VOID.into_val(&e),
            )
        ]
    );
    assert_eq!(insurance_fund.get_is_killed_deposit(), false);
    assert_eq!(insurance_fund.get_is_killed_request_withdraw(), true);
    assert_eq!(insurance_fund.get_is_killed_withdraw(), false);

    let user1 = users[1].clone();
    let desired_amount = 1_0000000;

    assert_eq!(
        insurance_fund
            .try_request_withdraw(&user1, &setup.token_a.address, &desired_amount)
            .unwrap_err(),
        Ok(Error::from_contract_error(31))
    );

    insurance_fund.unkill_request_withdraw(&admin);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&e, "unkill_request_withdraw"),).into_val(&e),
                Val::VOID.into_val(&e),
            )
        ]
    );
    assert_eq!(insurance_fund.get_is_killed_deposit(), false);
    assert_eq!(insurance_fund.get_is_killed_request_withdraw(), false);
    assert_eq!(insurance_fund.get_is_killed_withdraw(), false);

    insurance_fund.deposit(&user1, &setup.token_a.address, &desired_amount);

    jump(&e, THIRTY_DAY);

    insurance_fund.request_withdraw(&user1, &setup.token_a.address, &desired_amount);
}

#[test]
fn test_withdraw_killed() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            mint_to_user: i128::MAX,
            ..TestConfig::default()
        }),
    );
    let e = setup.env;
    let insurance_fund = setup.insurance_fund;
    let users = setup.users;

    assert_eq!(insurance_fund.get_is_killed_deposit(), false);
    assert_eq!(insurance_fund.get_is_killed_request_withdraw(), false);
    assert_eq!(insurance_fund.get_is_killed_withdraw(), false);

    let admin = setup.admin;

    insurance_fund.kill_withdraw(&admin);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&e, "kill_withdraw"),).into_val(&e),
                Val::VOID.into_val(&e),
            )
        ]
    );
    assert_eq!(insurance_fund.get_is_killed_deposit(), false);
    assert_eq!(insurance_fund.get_is_killed_request_withdraw(), false);
    assert_eq!(insurance_fund.get_is_killed_withdraw(), true);

    let user1 = users[1].clone();
    let amount_to_deposit = 2 * PRICE_PRECISION;
    let amount_to_withdraw = 1 * PRICE_PRECISION;

    insurance_fund.deposit(&user1, &setup.token_a.address, &amount_to_deposit);

    jump(&e, THIRTY_DAY);

    insurance_fund.request_withdraw(&user1, &setup.token_a.address, &amount_to_withdraw);

    assert_eq!(
        insurance_fund.try_withdraw(&user1, &setup.token_a.address).unwrap_err(),
        Ok(Error::from_contract_error(32))
    );

    insurance_fund.unkill_withdraw(&admin);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![
            &e,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&e, "unkill_withdraw"),).into_val(&e),
                Val::VOID.into_val(&e),
            )
        ]
    );
    assert_eq!(insurance_fund.get_is_killed_deposit(), false);
    assert_eq!(insurance_fund.get_is_killed_request_withdraw(), false);
    assert_eq!(insurance_fund.get_is_killed_withdraw(), false);

    insurance_fund.withdraw(&user1, &setup.token_a.address);
}

#[test]
fn test_kill_deposit_event() {
    let setup = Setup::default();
    let insurance_fund = setup.insurance_fund;

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
fn test_kill_request_withdraw_event() {
    let setup = Setup::default();
    let insurance_fund = setup.insurance_fund;

    insurance_fund.kill_request_withdraw(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&setup.env, "kill_request_withdraw"),).into_val(&setup.env),
                ().into_val(&setup.env),
            )
        ]
    );
}

#[test]
fn test_kill_withdraw_event() {
    let setup = Setup::default();
    let insurance_fund = setup.insurance_fund;

    insurance_fund.kill_withdraw(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&setup.env, "kill_withdraw"),).into_val(&setup.env),
                ().into_val(&setup.env),
            )
        ]
    );
}

#[test]
fn test_unkill_deposit_event() {
    let setup = Setup::default();
    let insurance_fund = setup.insurance_fund;

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
fn test_unkill_request_withdraw_event() {
    let setup = Setup::default();
    let insurance_fund = setup.insurance_fund;

    insurance_fund.unkill_request_withdraw(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&setup.env, "unkill_request_withdraw"),).into_val(&setup.env),
                ().into_val(&setup.env),
            )
        ]
    );
}

#[test]
fn test_unkill_withdraw_event() {
    let setup = Setup::default();
    let insurance_fund = setup.insurance_fund;

    insurance_fund.unkill_withdraw(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![
            &setup.env,
            (
                insurance_fund.address.clone(),
                (Symbol::new(&setup.env, "unkill_withdraw"),).into_val(&setup.env),
                ().into_val(&setup.env),
            )
        ]
    );
}

// ============================================================================
// NEW COMPREHENSIVE TEST IMPLEMENTATIONS
// ============================================================================

// Tests for withdrawing all shares (minus 1 operation)
#[test]
fn test_withdraw_all_shares_minus_one() {
    let setup = Setup::default();
    let user = &setup.users[1];
    let deposit_amount = 100 * PRICE_PRECISION;
    
    // Deposit
    setup.deposit_and_expect_success(user, deposit_amount as i128);
    
    let user_shares = setup.get_user_shares(user);
    
    // Request withdraw of all shares minus 1
    let withdraw_shares = user_shares - 1;
    setup.request_withdraw_and_expect_success(user, withdraw_shares);
    
    // Advance time past unstaking period
    setup.advance_time_past_unstaking();
    
    // Execute withdrawal
    let withdrawn_amount = setup.withdraw_and_expect_success(user);
    
    // Verify user has 1 share remaining
    setup.assert_user_shares(user, 1);
    
    // Verify withdrawal amount is correct (shares - 1 worth of tokens)
    assert!(withdrawn_amount > 0);
    assert!(withdrawn_amount < deposit_amount as u128);
}

// Tests for operations after premium has been paid
#[test] 
fn test_operations_after_premium_payment() {
    let setup = Setup::default();
    let user = &setup.users[1];
    let premium_amount = 50 * PRICE_PRECISION;
    let deposit_amount = 100 * PRICE_PRECISION;
    
    // Pay premium first
    setup.pay_premium(premium_amount as i128);
    
    // Test deposit after premium
    let shares = setup.deposit_and_expect_success(user, deposit_amount as i128);
    assert!(shares > 0);
    
    // Test withdraw request after premium
    setup.request_withdraw_and_expect_success(user, shares / 2);
    
    // Test withdrawal after premium
    setup.advance_time_past_unstaking();
    let withdrawn = setup.withdraw_and_expect_success(user);
    assert!(withdrawn > 0);
}

#[test]
fn test_multiple_operations_after_premium() {
    let setup = Setup::default();
    let users = &setup.users;
    let premium_amount = 25 * PRICE_PRECISION;
    
    // Pay premium
    setup.pay_premium(premium_amount as i128);
    
    // Test multiple users can deposit
    let amounts = std::vec![100i128 * PRICE_PRECISION as i128, 150i128 * PRICE_PRECISION as i128, 75i128 * PRICE_PRECISION as i128];
    
    // Test multiple deposits different users
    let shares = setup.setup_multiple_deposits(&amounts);
    
    // Verify each user received correct shares
    for (i, amount) in amounts.iter().enumerate() {
        let user = &users[i];
        setup.assert_user_shares(user, *amount as u128);
    }
    
    // Verify total vault balance
    let total_deposited = amounts.iter().sum::<i128>() as u128;
    setup.assert_vault_balance(total_deposited);
}

// Setter tests
#[test]
fn test_admin_setters() {
    let setup = Setup::default();
    let admin = &setup.admin;
    
    // Test set_optimal_insurance
    let new_optimal = 500_000u128 * PRICE_PRECISION as u128;
    setup.insurance_fund.set_optimal_insurance(admin, &new_optimal);
    assert_eq!(setup.insurance_fund.get_optimal_insurance(), new_optimal);
    
    // Test set_unstaking_period  
    let new_period = 7 * 24 * 60 * 60u64; // 7 days
    setup.insurance_fund.set_unstaking_period(admin, &new_period);
    assert_eq!(setup.insurance_fund.get_unstaking_period(), new_period);
    
    // Test set_rate_config
    let new_optimal_util = 75_00000u32;
    let new_base_rate = 3_00000i32;
    let new_slope_a = 15_00000u32;
    let new_slope_b = 75_00000u32;
    
    setup.insurance_fund.set_rate_config(
        admin, 
        &new_optimal_util, 
        &new_base_rate, 
        &new_slope_a, 
        &new_slope_b
    );
    
    assert_eq!(setup.insurance_fund.get_optimal_utilization(), new_optimal_util);
    assert_eq!(setup.insurance_fund.get_base_rate(), new_base_rate);
    assert_eq!(setup.insurance_fund.get_rate_slopes(), (new_slope_a, new_slope_b));
}

#[test]
#[should_panic(expected = "Error(Contract, #16)")] // NotAuthorized
fn test_setters_unauthorized() {
    let setup = Setup::default();
    let unauthorized_user = &setup.users[1];
    
    // Attempt to set optimal insurance as unauthorized user
    setup.insurance_fund.set_optimal_insurance(unauthorized_user, &(1000u128 * PRICE_PRECISION as u128));
}

// Getter tests
#[test]
fn test_getters() {
    let setup = Setup::default();
    
    // Test address getters
    assert_eq!(setup.insurance_fund.get_premium_token(), setup.token_a.address);
    
    // Test config getters
    assert!(setup.insurance_fund.get_unstaking_period() > 0);
    assert!(setup.insurance_fund.get_optimal_insurance() > 0);
    assert!(setup.insurance_fund.get_optimal_utilization() > 0);
    assert!(setup.insurance_fund.get_base_rate() >= 0);
    
    let (slope_a, slope_b) = setup.insurance_fund.get_rate_slopes();
    assert!(slope_a > 0);
    assert!(slope_b > 0);
    
    // Test utilization and rate calculations
    let utilization = setup.insurance_fund.get_utilization();
    let rate = setup.insurance_fund.get_rate();
    assert!(utilization >= 0);
    assert!(rate >= 0);
}

#[test]
fn test_getters_with_activity() {
    let setup = Setup::default();
    let user = &setup.users[1];
    let deposit_amount = 100 * PRICE_PRECISION;
    
    // Test getters after deposit
    setup.deposit_and_expect_success(user, deposit_amount as i128);
    
    let reserve = setup.insurance_fund.get_reserve(&setup.token_a.address);
    assert!(reserve.total_shares > 0);
    
    let stake = setup.insurance_fund.get_stake(user, &setup.token_a.address);
    assert!(stake.unchecked_shares() > 0);
    
    // Test utilization after deposit
    let utilization = setup.insurance_fund.get_utilization();
    assert!(utilization >= 0);
}

// Additional comprehensive deposit tests
#[test]
fn test_deposit_comprehensive() {
    let setup = Setup::default();
    let user = &setup.users[1];
    
    // Test singular deposit
    let amount1 = 50 * PRICE_PRECISION;
    let shares1 = setup.deposit_and_expect_success(user, amount1 as i128);
    assert_eq!(shares1, amount1 as u128);
    
    // Test multiple deposits same user  
    let amount2 = 25 * PRICE_PRECISION;
    let shares2 = setup.deposit_and_expect_success(user, amount2 as i128);
    assert_eq!(shares2, amount2 as u128);
    
    // Verify total shares
    setup.assert_user_shares(user, (amount1 + amount2) as u128);
}

#[test]
fn test_deposit_multiple_users() {
    let setup = Setup::default();
    let amounts = std::vec![100i128 * PRICE_PRECISION as i128, 150i128 * PRICE_PRECISION as i128, 75i128 * PRICE_PRECISION as i128];
    
    // Test multiple deposits different users
    let shares = setup.setup_multiple_deposits(&amounts);
    
    // Verify each user received correct shares
    for (i, amount) in amounts.iter().enumerate() {
        let user = &setup.users[i];
        setup.assert_user_shares(user, *amount as u128);
    }
    
    // Verify total vault balance
    let total_deposited = amounts.iter().sum::<i128>() as u128;
    setup.assert_vault_balance(total_deposited);
}
