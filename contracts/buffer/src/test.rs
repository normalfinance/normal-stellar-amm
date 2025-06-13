#![cfg(test)]
extern crate std;

use crate::reserve::Reserve;
use crate::testutils::{ Setup, TestConfig };
use soroban_sdk::testutils::{ Address as _, AuthorizedFunction, AuthorizedInvocation, Events };
use soroban_sdk::{ vec, Address, Error, IntoVal, Symbol, Val, Vec };
use utils::constant::{FIVE_MINUTE, ONE_HOUR, TWENTY_FOUR_HOUR};
use utils::test_utils::jump;

#[test]
fn test_deposit() {
    let setup = Setup::default();
    let users = setup.users;

    let amount_to_deposit = 100_0000000_u128;

    // Mint tokens to user
    setup.token_a_admin_client.mint(&users[1], &(amount_to_deposit as i128));

    // Deposit
    setup.buffer.deposit(&users[1], &setup.token_a.address, &amount_to_deposit);

    assert_eq!(setup.env.auths()[0], (
        users[1].clone(),
        AuthorizedInvocation {
            function: AuthorizedFunction::Contract((
                setup.buffer.address.clone(),
                Symbol::new(&setup.env, "deposit"),
                Vec::from_array(&setup.env, [
                    users[1].to_val(),
                    setup.token_a.address.to_val(),
                    amount_to_deposit.into_val(&setup.env),
                ]),
            )),
            sub_invocations: std::vec![AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    setup.token_a.address.clone(),
                    Symbol::new(&setup.env, "transfer"),
                    Vec::from_array(&setup.env, [
                        users[1].to_val(),
                        setup.buffer.address.to_val(),
                        (amount_to_deposit as i128).into_val(&setup.env),
                    ]),
                )),
                sub_invocations: std::vec![],
            }],
        },
    ));

    // Token was transferred from user to Buffer
    assert_eq!(setup.token_a.balance(&users[1]), 0);
    assert_eq!(setup.token_a.balance(&setup.buffer.address), amount_to_deposit as i128);

    // Buffer reserve updates
    let now = setup.env.ledger().timestamp();
    assert_eq!(setup.buffer.get_reserve(&setup.token_a.address), Reserve {
        balance: amount_to_deposit,
        max_balance: 0,
        total_inflow: amount_to_deposit,
        total_outflow: 0,
        total_withdraw: 0,
        last_payout: 0,
        last_payout_ts: 0,
        last_update_ts: now,
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #801)")]
fn test_deposit_invalid_token() {
    let setup = Setup::default();
    let bogus_token = Address::generate(&setup.env);

    setup.token_a_admin_client.mint(&setup.admin, &(100_0000000_u128 as i128));

    setup.buffer.deposit(&setup.admin, &bogus_token, &100_0000000_u128);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_deposit_over_max() {
    let setup = Setup::default();
    let amount_to_deposit = 100_0000000_u128;

    setup.buffer.set_reserve_max_balance(&setup.admin, &setup.token_a.address, &99_0000000_u128);

    setup.token_a_admin_client.mint(&setup.admin, &(amount_to_deposit as i128));

    setup.buffer.deposit(&setup.admin, &setup.token_a.address, &amount_to_deposit);
}

#[test]
fn test_request_payout() {
    let setup = Setup::default();
    let users = setup.users;

    // Deposit
    let amount_to_deposit = 100_0000000_u128;
    setup.token_a_admin_client.mint(&setup.admin, &(amount_to_deposit as i128));
    setup.buffer.deposit(&setup.admin, &setup.token_a.address, &amount_to_deposit);
    assert_eq!(setup.token_a.balance(&users[1]), 0);
    assert_eq!(setup.token_a.balance(&setup.buffer.address), amount_to_deposit as i128);

    // Request payout
    let reserve_before = setup.buffer.get_reserve(&setup.token_a.address);
    let desired_payout = 50_0000000_u128;
    setup.buffer.request_payout(&setup.router, &setup.token_a.address, &desired_payout);

    // Buffer transferred token to Router
    assert_eq!(setup.token_a.balance(&setup.router), desired_payout as i128);
    assert_eq!(
        setup.token_a.balance(&setup.buffer.address),
        (amount_to_deposit - desired_payout) as i128
    );

    // Buffer reserves updates
    let now = setup.env.ledger().timestamp();
    assert_eq!(setup.buffer.get_reserve(&setup.token_a.address), Reserve {
        balance: reserve_before.balance - desired_payout,
        max_balance: reserve_before.max_balance,
        total_inflow: reserve_before.total_inflow,
        total_outflow: reserve_before.total_outflow + desired_payout,
        total_withdraw: reserve_before.total_withdraw,
        last_payout: desired_payout,
        last_payout_ts: now,
        last_update_ts: now,
    });

    // Buffer last payout timestamp updated
    assert_eq!(setup.buffer.get_last_payout_timestamp(), setup.env.ledger().timestamp());
}

#[test]
#[should_panic(expected = "Error(Contract, #801)")]
fn test_request_payout_invalid_token() {
    let setup = Setup::default();
    let bogus_token = Address::generate(&setup.env);

    // Deposit
    let amount = 100_0000000_u128;
    setup.token_a_admin_client.mint(&setup.admin, &(amount as i128));
    setup.buffer.deposit(&setup.admin, &setup.token_a.address, &amount);

    setup.buffer.request_payout(&setup.router, &bogus_token, &100_0000000_u128);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_request_payout_from_not_router() {
    let setup = Setup::default();
    let users = setup.users;

    // Deposit
    let amount = 100_0000000_u128;
    setup.token_a_admin_client.mint(&setup.admin, &(amount as i128));
    setup.buffer.deposit(&setup.admin, &setup.token_a.address, &amount);

    setup.buffer.request_payout(&users[1], &setup.token_a.address, &100_0000000_u128);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_request_payout_too_soon() {
    let setup = Setup::default();
    let amount_to_deposit = 100_0000000_u128;
    let amount_to_payout = 25_0000000_u128;

    // Set min time
    let time = TWENTY_FOUR_HOUR; // 60_0000000_u64; // 60 seconds
    setup.buffer.set_min_time_between_payouts(&setup.admin, &time);

    // Deposit
    setup.token_a_admin_client.mint(&setup.admin, &(amount_to_deposit as i128));
    setup.buffer.deposit(&setup.admin, &setup.token_a.address, &amount_to_deposit);

    setup.buffer.request_payout(&setup.router, &setup.token_a.address, &amount_to_payout);
    assert_eq!(setup.token_a.balance(&setup.router), amount_to_payout as i128);

    // 10 seconds, not 60
    jump(&setup.env, ONE_HOUR);

    setup.buffer.request_payout(&setup.router, &setup.token_a.address, &amount_to_payout);
}

#[test]
fn test_multiple_request_payouts() {
    let setup = Setup::default();
    let deposit_amount = 100_0000000_u128;
    let payout_amount_1 = 50_0000000_u128;
    let payout_amount_2 = 10_0000000_u128;

    // Set min time
    let time = 60_u64;
    setup.buffer.set_min_time_between_payouts(&setup.admin, &time);

    // Deposit

    setup.token_a_admin_client.mint(&setup.admin, &(deposit_amount as i128));
    setup.buffer.deposit(&setup.admin, &setup.token_a.address, &deposit_amount);

    // 1st
    setup.buffer.request_payout(&setup.router, &setup.token_a.address, &payout_amount_1);

    jump(&setup.env, time);

    // 2nd
    setup.buffer.request_payout(&setup.router, &setup.token_a.address, &payout_amount_2);

    assert_eq!(
        setup.token_a.balance(&setup.buffer.address),
        (deposit_amount - payout_amount_1 - payout_amount_2) as i128
    );
    assert_eq!(setup.token_a.balance(&setup.router), (payout_amount_1 + payout_amount_2) as i128);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_request_payout_insufficient_balance() {
    let setup = Setup::default();

    // Deposit
    let amount = 100_0000000_u128;
    setup.token_a_admin_client.mint(&setup.admin, &(amount as i128));
    setup.buffer.deposit(&setup.admin, &setup.token_a.address, &amount);

    setup.buffer.request_payout(&setup.router, &setup.token_a.address, &100000_0000000_u128);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_request_payout_unsynced() {
    let setup = Setup::default();

    // Deposit
    let amount = 100_0000000_u128;
    setup.token_a_admin_client.mint(&setup.admin, &(amount as i128));
    setup.buffer.deposit(&setup.admin, &setup.token_a.address, &amount);

    // Mint additional tokens to the Buffer
    setup.token_a_admin_client.mint(&setup.buffer.address, &10_0000000_i128);

    // Don't call sync()

    setup.buffer.request_payout(&setup.router, &setup.token_a.address, &110_0000000_u128);
}

#[test]
fn test_withdraw_surplus() {
    let setup = Setup::default();
    let amount_to_deposit = 100_0000000_u128;
    let amount_to_withdraw = 10_0000000_u128;

    // Deposit
    setup.token_a_admin_client.mint(&setup.admin, &(amount_to_deposit as i128));
    setup.buffer.deposit(&setup.admin, &setup.token_a.address, &amount_to_deposit);
    let reserve_before = setup.buffer.get_reserve(&setup.token_a.address);

    // Withdraw
    setup.buffer.withdraw_surplus(&setup.admin, &setup.token_a.address, &amount_to_withdraw);

    // Buffer has min reserve ratio
    assert_eq!(
        setup.token_a.balance(&setup.buffer.address),
        (amount_to_deposit - amount_to_withdraw) as i128
    );

    // Buffer sent token to admin
    assert_eq!(setup.token_a.balance(&setup.admin), amount_to_withdraw as i128);

    // Buffer reserve updates
    let now = setup.env.ledger().timestamp();
    assert_eq!(setup.buffer.get_reserve(&setup.token_a.address), Reserve {
        balance: reserve_before.balance - amount_to_withdraw,
        max_balance: reserve_before.max_balance,
        total_inflow: reserve_before.total_inflow,
        total_outflow: reserve_before.total_outflow + amount_to_withdraw,
        total_withdraw: reserve_before.total_withdraw + amount_to_withdraw,
        last_payout: reserve_before.last_payout,
        last_payout_ts: reserve_before.last_payout_ts,
        last_update_ts: now,
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #102)")]
fn test_withdraw_surplus_not_admin() {
    let setup = Setup::default();
    let users = setup.users;
    setup.buffer.withdraw_surplus(&users[1], &setup.token_a.address, &100_0000000_u128);
}

#[test]
#[should_panic(expected = "Error(Contract, #801)")]
fn test_withdraw_surplus_invalid_token() {
    let setup = Setup::default();
    let bogus_token = Address::generate(&setup.env);
    setup.buffer.withdraw_surplus(&setup.admin, &bogus_token, &100_0000000_u128);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_withdraw_surplus_over_min_reserve() {
    let setup = Setup::default();
    let amount_to_deposit = 100_0000000_u128;

    let min_reserve_ratio = setup.buffer.get_min_reserve_ratio();
    let min_reserve = (min_reserve_ratio as u128) * amount_to_deposit;
    let too_much_to_withdraw = amount_to_deposit - min_reserve + 1;

    // Deposit
    setup.token_a_admin_client.mint(&setup.admin, &(amount_to_deposit as i128));
    setup.buffer.deposit(&setup.admin, &setup.token_a.address, &amount_to_deposit);

    // Withdraw
    setup.buffer.withdraw_surplus(&setup.admin, &setup.token_a.address, &too_much_to_withdraw);
}

#[test]
fn test_sync() {
    let setup = Setup::default();
    let users = setup.users;

    // Mint
    let amount_to_deposit = 100_0000000_u128;
    setup.token_a_admin_client.mint(&setup.admin, &(amount_to_deposit as i128));

    // Deposit
    setup.buffer.deposit(&setup.admin, &setup.token_a.address, &amount_to_deposit);

    let reserve_before = setup.buffer.get_reserve(&setup.token_a.address);

    // Mint excess tokens to the buffer
    let excess_token_amount = 10_0000000_u128;
    setup.token_a_admin_client.mint(&setup.buffer.address, &(excess_token_amount as i128));

    // Ensure reserve does not change
    assert_eq!(setup.buffer.get_reserve(&setup.token_a.address), reserve_before);

    // Sync (anyone can call)
    setup.buffer.sync(&users[1], &setup.token_a.address);

    // Buffer reserve now matches balance
    let now = setup.env.ledger().timestamp();
    assert_eq!(setup.buffer.get_reserve(&setup.token_a.address), Reserve {
        balance: reserve_before.balance + excess_token_amount,
        max_balance: reserve_before.max_balance,
        total_inflow: reserve_before.total_inflow,
        total_outflow: reserve_before.total_outflow,
        total_withdraw: reserve_before.total_withdraw,
        last_payout: reserve_before.last_payout,
        last_payout_ts: reserve_before.last_payout_ts,
        last_update_ts: now,
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #801)")]
fn test_sync_invalid_token() {
    let setup = Setup::default();
    let users = setup.users;
    let bogus_token = Address::generate(&setup.env);

    // Sync (anyone can call)
    setup.buffer.sync(&users[1], &bogus_token);
}

#[test]
fn test_skim() {
    let setup = Setup::default();
    let users = setup.users;

    // Mint
    let amount_to_deposit = 100_0000000_u128;
    setup.token_a_admin_client.mint(&setup.admin, &(amount_to_deposit as i128));

    // Deposit
    setup.buffer.deposit(&setup.admin, &setup.token_a.address, &amount_to_deposit);

    let reserve_before = setup.buffer.get_reserve(&setup.token_a.address);

    // Mint excess tokens to the buffer
    let excess_token_amount = 10_0000000_u128;
    setup.token_a_admin_client.mint(&setup.buffer.address, &(excess_token_amount as i128));

    // Ensure reserve does not change
    assert_eq!(setup.buffer.get_reserve(&setup.token_a.address), reserve_before);

    // Skim (anyone can call)
    setup.buffer.skim(&users[1], &setup.token_a.address);

    // Ensure reserve does not change
    assert_eq!(setup.buffer.get_reserve(&setup.token_a.address), reserve_before);

    // And token balances are updated
    assert_eq!(setup.token_a.balance(&setup.buffer.address), amount_to_deposit as i128);
    assert_eq!(setup.token_a.balance(&users[1]), excess_token_amount as i128);
}

#[test]
#[should_panic(expected = "Error(Contract, #801)")]
fn test_skim_invalid_token() {
    let setup = Setup::default();
    let users = setup.users;
    let bogus_token = Address::generate(&setup.env);

    // Sync (anyone can call)
    setup.buffer.skim(&users[1], &bogus_token);
}

#[test]
fn test_events() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        })
    );
    let e = setup.env;
    let buffer = setup.buffer;
    let token1 = setup.token_a;
    let user1 = setup.users[1].clone();
    let amount_to_deposit = 100_0000000_u128;
    let amount_to_payout = 50_0000000_u128;
    let amount_to_withdraw = 10_0000000_u128;
    let excess_amount = 25_0000000_u128;

    // mint
    setup.token_a_admin_client.mint(&user1, &(amount_to_deposit as i128));

    // deposit
    buffer.deposit(&user1, &token1.address, &amount_to_deposit);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![&e, (
            buffer.address.clone(),
            (Symbol::new(&e, "deposit"), token1.address.clone(), user1.clone()).into_val(&e),
            amount_to_deposit.into_val(&e),
        )]
    );

    // request_payout
    buffer.request_payout(&setup.router, &token1.address, &amount_to_payout);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![&e, (
            buffer.address.clone(),
            (
                Symbol::new(&e, "request_payout"),
                token1.address.clone(),
                setup.router.clone(),
            ).into_val(&e),
            amount_to_payout.into_val(&e),
        )]
    );

    // withdraw_surplus
    buffer.withdraw_surplus(&setup.admin, &token1.address, &amount_to_withdraw);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![&e, (
            buffer.address.clone(),
            (
                Symbol::new(&e, "withdraw_surplus"),
                token1.address.clone(),
                setup.admin.clone(),
            ).into_val(&e),
            amount_to_withdraw.into_val(&e),
        )]
    );

    // skim
    setup.token_a_admin_client.mint(&user1, &(excess_amount as i128));
    buffer.skim(&user1, &token1.address);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![&e, (
            buffer.address.clone(),
            (Symbol::new(&e, "skim"), token1.address.clone(), user1.clone()).into_val(&e),
            excess_amount.into_val(&e),
        )]
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
            ..TestConfig::default()
        })
    );
    let e = setup.env;
    let buffer = setup.buffer;
    let users = setup.users;

    assert_eq!(buffer.get_is_killed_deposit(), false);
    assert_eq!(buffer.get_is_killed_request_payout(), false);

    let admin = setup.admin;

    buffer.kill_deposit(&admin);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![&e, (
            buffer.address.clone(),
            (Symbol::new(&e, "kill_deposit"),).into_val(&e),
            Val::VOID.into_val(&e),
        )]
    );
    assert_eq!(buffer.get_is_killed_deposit(), true);
    assert_eq!(buffer.get_is_killed_request_payout(), false);

    let user1 = users[1].clone();
    let desired_amount = 1_0000000;

    // Mint tokens to user
    setup.token_a_admin_client.mint(&users[1], &(desired_amount as i128));

    assert_eq!(
        buffer.try_deposit(&user1, &setup.token_a.address, &desired_amount).unwrap_err(),
        Ok(Error::from_contract_error(6))
    );

    buffer.unkill_deposit(&admin);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![&e, (
            buffer.address.clone(),
            (Symbol::new(&e, "unkill_deposit"),).into_val(&e),
            Val::VOID.into_val(&e),
        )]
    );
    assert_eq!(buffer.get_is_killed_deposit(), false);
    assert_eq!(buffer.get_is_killed_request_payout(), false);

    buffer.deposit(&user1, &setup.token_a.address, &desired_amount);
}

#[test]
fn test_request_payout_killed() {
    let setup = Setup::new_with_config(
        &(TestConfig {
            ..TestConfig::default()
        })
    );
    let e = setup.env;
    let buffer = setup.buffer;
    let users = setup.users;

    assert_eq!(buffer.get_is_killed_deposit(), false);
    assert_eq!(buffer.get_is_killed_request_payout(), false);

    let admin = setup.admin;

    buffer.kill_request_payout(&admin);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![&e, (
            buffer.address.clone(),
            (Symbol::new(&e, "kill_request_payout"),).into_val(&e),
            Val::VOID.into_val(&e),
        )]
    );
    assert_eq!(buffer.get_is_killed_deposit(), false);
    assert_eq!(buffer.get_is_killed_request_payout(), true);

    let desired_amount = 1_0000000;

    assert_eq!(
        buffer
            .try_request_payout(&setup.router, &setup.token_a.address, &desired_amount)
            .unwrap_err(),
        Ok(Error::from_contract_error(7))
    );

    buffer.unkill_request_payout(&admin);
    assert_eq!(
        vec![&e, e.events().all().last().unwrap()],
        vec![&e, (
            buffer.address.clone(),
            (Symbol::new(&e, "unkill_request_payout"),).into_val(&e),
            Val::VOID.into_val(&e),
        )]
    );
    assert_eq!(buffer.get_is_killed_deposit(), false);
    assert_eq!(buffer.get_is_killed_request_payout(), false);

    // setup for payout
    setup.token_a_admin_client.mint(&users[1], &(desired_amount as i128));
    buffer.deposit(&users[1], &setup.token_a.address, &desired_amount);

    buffer.request_payout(&setup.router, &setup.token_a.address, &desired_amount);
}

#[test]
fn test_kill_deposit_event() {
    let setup = Setup::default();
    let buffer = setup.buffer;

    buffer.kill_deposit(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            buffer.address.clone(),
            (Symbol::new(&setup.env, "kill_deposit"),).into_val(&setup.env),
            ().into_val(&setup.env),
        )]
    );
}

#[test]
fn test_kill_request_payout_event() {
    let setup = Setup::default();
    let buffer = setup.buffer;

    buffer.kill_request_payout(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            buffer.address.clone(),
            (Symbol::new(&setup.env, "kill_request_payout"),).into_val(&setup.env),
            ().into_val(&setup.env),
        )]
    );
}

#[test]
fn test_unkill_deposit_event() {
    let setup = Setup::default();
    let buffer = setup.buffer;

    buffer.unkill_deposit(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            buffer.address.clone(),
            (Symbol::new(&setup.env, "unkill_deposit"),).into_val(&setup.env),
            ().into_val(&setup.env),
        )]
    );
}

#[test]
fn test_unkill_request_payout_event() {
    let setup = Setup::default();
    let buffer = setup.buffer;

    buffer.unkill_request_payout(&setup.admin);
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            buffer.address.clone(),
            (Symbol::new(&setup.env, "unkill_request_payout"),).into_val(&setup.env),
            ().into_val(&setup.env),
        )]
    );
}
