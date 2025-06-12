#![cfg(test)]
extern crate std;

// use crate::reserve::Reserve;
use crate::testutils::{ Setup, TestConfig };
use soroban_sdk::testutils::{ Address as _, AuthorizedFunction, AuthorizedInvocation, Events };
use soroban_sdk::{ vec, Address, Error, IntoVal, Symbol, Val, Vec };
use utils::test_utils::buffer::Reserve;
use utils::test_utils::jump;

//  ___      ___       __        __    _____  ___
// |"  \    /"  |     /""\      |" \  (\"   \|"  \
//  \   \  //   |    /    \     ||  | |.\\   \    |
//  /\\  \/.    |   /' /\  \    |:  | |: \.   \\  |
// |: \.        |  //  __'  \   |.  | |.  \    \. |
// |.  \    /:  | /   /  \\  \  /\  |\|    \    \ |
// |___|\__/|___|(___/    \___)(__\_|_)\___|\____\)

#[test]
fn test_deposit() {
    let setup = Setup::default();

    let amount_to_deposit = 100_0000000_u128;

    setup.token_a_admin_client.mint(&setup.router.address, &(amount_to_deposit as i128));

    setup.buffer.deposit(&setup.router.address, &setup.token_a.address, &amount_to_deposit);
    assert_eq!(setup.env.auths()[0], (
        setup.admin.clone(),
        AuthorizedInvocation {
            function: AuthorizedFunction::Contract((
                setup.buffer.address.clone(),
                Symbol::new(&setup.env, "deposit"),
                Vec::from_array(&setup.env, [
                    setup.admin.to_val(),
                    amount_to_deposit.into_val(&setup.env),
                ]),
            )),
            sub_invocations: std::vec![AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    setup.token_a.address.clone(),
                    Symbol::new(&setup.env, "transfer"),
                    Vec::from_array(&setup.env, [
                        setup.admin.to_val(),
                        setup.buffer.address.to_val(),
                        (amount_to_deposit as i128).into_val(&setup.env),
                    ]),
                )),
                sub_invocations: std::vec![],
            }],
        },
    ));

    // Token was transferred from user to Buffer
    assert_eq!(setup.token_a.balance(&setup.router.address), 0);
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
fn test_request_payout() {
    let setup = Setup::default();

    let amount_to_payout = 100_0000000_u128;

    let reserve_before = setup.buffer.get_reserve(&setup.token_a.address);

    setup.buffer.request_payout(&setup.admin, &setup.token_a.address, &amount_to_payout);

    // Buffer transferred token to Router
    assert_eq!(setup.token_a.balance(&setup.router.address), amount_to_payout as i128);

    // Buffer reserves updates
    let now = setup.env.ledger().timestamp();
    assert_eq!(setup.buffer.get_reserve(&setup.token_a.address), Reserve {
        balance: reserve_before.balance - amount_to_payout,
        max_balance: reserve_before.max_balance,
        total_inflow: reserve_before.total_inflow,
        total_outflow: reserve_before.total_outflow + amount_to_payout,
        total_withdraw: reserve_before.total_withdraw,
        last_payout: amount_to_payout,
        last_payout_ts: now,
        last_update_ts: now,
    });

    // Buffer last payout timestamp updated
    assert_eq!(setup.buffer.get_last_payout_timestamp(), setup.env.ledger().timestamp());
}

#[test]
fn test_withdraw_surplus() {
    let setup = Setup::default();

    let amount_to_witdraw = 10_0000000_u128;

    // TODO: must we deposit first?

    let reserve_before = setup.buffer.get_reserve(&setup.token_a.address);

    setup.buffer.withdraw_surplus(&setup.admin, &setup.token_a.address, &amount_to_witdraw);

    let expected_balance = reserve_before.balance * setup.buffer.get_min_reserve_ratio();

    // Buffer has min reserve ratio
    assert_eq!(setup.token_a.balance(&setup.buffer.address), expected_balance as i128);
    // Buffer reserve updates
    let now = setup.env.ledger().timestamp();
    assert_eq!(setup.buffer.get_reserve(&setup.token_a.address), Reserve {
        balance: reserve_before.balance - amount_to_witdraw,
        max_balance: reserve_before.max_balance,
        total_inflow: reserve_before.total_inflow,
        total_outflow: reserve_before.total_outflow + amount_to_witdraw,
        total_withdraw: reserve_before.total_withdraw + amount_to_witdraw,
        last_payout: reserve_before.last_payout,
        last_payout_ts: reserve_before.last_payout_ts,
        last_update_ts: now,
    });
    // Buffer sent token to admin
    assert_eq!(setup.token_a.balance(&setup.admin), amount_to_witdraw as i128);
}

#[test]
fn test_sync() {
    let setup = Setup::default();

    let excess_token_amount = 100_0000000_u128;

    let reserve_before = setup.buffer.get_reserve(&setup.token_a.address);

    setup.buffer.sync(&setup.admin, &setup.token_a.address);

    // Mint excess tokens to the buffer
    setup.token_a_admin_client.mint(&setup.buffer.address, &(excess_token_amount as i128));

    // Buffer reserve now matches balance
    assert_eq!(
        setup.token_a.balance(&setup.buffer.address),
        (reserve_before.balance + excess_token_amount) as i128
    );
}

#[test]
fn test_skim() {
    let setup = Setup::default();

    let result = setup.buffer.skim(&setup.admin, &setup.token_a.address);

    // Buffer reserve now matches balance
    assert_eq!(setup.token_a.balance(&setup.router.address), 0);
    // Excess tokens were sent to admin
    assert_eq!(setup.token_a.balance(&setup.admin), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #16)")]
fn test_deposit_from_not_fee_collector() {
    let setup = Setup::default();
    let user = Address::generate(&setup.env);

    setup.buffer.deposit(&user, &setup.token_a.address, &100_0000000_u128);
}

#[test]
#[should_panic(expected = "Error(Contract, #16)")]
fn test_request_payout_from_not_router() {
    let setup = Setup::default();
    let user = Address::generate(&setup.env);
    setup.buffer.request_payout(&user, &setup.token_a.address, &100_0000000_u128);
}

#[test]
#[should_panic(expected = "Error(Contract, #2904)")]
fn test_deposit_invalid_token() {
    let setup = Setup::default();
    let bogus_token = Address::generate(&setup.env);
    setup.buffer.deposit(&setup.admin, &bogus_token, &100_0000000_u128);
}

#[test]
#[should_panic(expected = "Error(Contract, #2904)")]
fn test_deposit_over_max() {
    let setup = Setup::default();
    setup.buffer.set_reserve_max_balance(&setup.admin, &setup.token_a.address, &99_0000000_u128);
    setup.buffer.deposit(&setup.admin, &setup.token_a.address, &100_0000000_u128);
}

#[test]
#[should_panic(expected = "Error(Contract, #2904)")]
fn test_request_payout_too_soon() {
    let setup = Setup::default();

    setup.buffer.request_payout(&setup.admin, &setup.token_a.address, &100_0000000_u128);

    // 10 seconds
    jump(&setup.env, 10);

    setup.buffer.request_payout(&setup.admin, &setup.token_a.address, &10_0000000_u128);
}

#[test]
#[should_panic(expected = "Error(Contract, #2904)")]
fn test_request_payout_insufficient_balance() {
    let setup = Setup::default();
    setup.buffer.request_payout(&setup.admin, &setup.token_a.address, &100000_0000000_u128);
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

    let admin = users[0].clone();

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

    assert_eq!(
        buffer.try_deposit(&user1, &setup.token_a.address, &desired_amount).unwrap_err(),
        Ok(Error::from_contract_error(205))
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

    let admin = users[0].clone();

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

    let user1 = users[1].clone();
    let desired_amount = 1_0000000;

    assert_eq!(
        buffer.try_request_payout(&user1, &setup.token_a.address, &desired_amount).unwrap_err(),
        Ok(Error::from_contract_error(209))
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

    buffer.request_payout(&user1, &setup.token_a.address, &desired_amount);
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
