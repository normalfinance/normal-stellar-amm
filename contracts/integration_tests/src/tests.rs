#![cfg(test)]
extern crate std;

use crate::contracts::buffer::Reserve;
use crate::contracts::pool::InsuranceClaim;
use crate::testutils::{ Setup };
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::TokenClient;
use soroban_sdk::{ vec, Address, IntoVal, Symbol, Vec };
use utils::constant::ONE_MINUTE;
use utils::test_utils::jump;

/**
 * Swap Test Scenarios
 * 
 * 🟢 1. Price Movement Scenarios (Oracle-driven)
        Test how the pool reacts to changes in the oracle price, since this affects the amount of synthetic asset minted/burned and swap behavior.

        Normal Conditions
        Gradual price increase

        Gradual price decrease

        Flat price for extended periods

        Volatile/Edge Cases
        Sudden large upward spike (e.g. +50% in a minute)

        Sudden crash (e.g. -80%)

        Flash crash and recovery (drop + full recovery in a short span)

        Oscillating/choppy prices (high-frequency changes within a narrow range)

        🟠 2. Swap Activity Scenarios
        Swap behavior impacts LP value, insurance fund utilization, and fee accumulation.

        Regular Activity
        Steady buy/sell volume matching market demand

        Alternating buys and sells near NAV

        Imbalanced Pressure
        Consistent one-sided buys (e.g. tracking token demand outpaces base)

        Consistent one-sided sells (tracking token being dumped)

        Stress Activity
        Massive arbitrage attempts from NAV drift

        Back-to-back large swaps (e.g. whale trades)

        High-frequency micro-swaps (test fee accounting and rounding behavior)

        🔵 3. Liquidity Scenarios
        Liquidity conditions affect slippage, solvency, and pool health.

        Normal Liquidity
        Full initial pool deposit

        Moderate swap fees relative to volume

        Low Liquidity
        Minimal LP capital

        Few/no swaps for a long time, then sudden large swap

        Liquidity Churn
        LPs frequently enter and exit

        Partial withdrawal after each major price movement

        🔴 4. Synthetic Price Divergence Scenarios
        Since the token is synthetic and trades on AMMs like Uniswap, test how deviations from NAV are corrected.

        Token price > NAV (arbitrage opportunity to mint and sell)

        Token price < NAV (arbitrage to buy and redeem/burn)

        NAV shifts sharply while market price lags

        Broken peg not corrected (e.g. during oracle outage)

        🟣 5. Oracle Anomalies
        Your pool depends on oracles — simulate misbehavior.

        Delayed updates (oracle price stale)

        Erroneous price (1,000x spike due to bad data)

        Oracle downtime (no updates for X blocks)

        Oracle switching (change data sources mid-operation)
 */

/**
 * Integration Tests needed
 *
 * [ ] Liquidity Imbalance Event - Buffer
 *  - [ ] Not enough coverage
 *  - [x] Enough coverage
 * [ ] Liquidity Imbalance Event - IF
 *  - [ ] No coverage
 *  - [ ] Too little coverage
 *  - [x] Enough coverge
 *  - [ ] Too much coverage
 */

//    _______    ______      ______    ___
//   |   __ "\  /    " \    /    " \  |"  |
//   (. |__) :)// ____  \  // ____  \ ||  |
//   |:  ____//  /    ) :)/  /    ) :)|:  |
//   (|  /   (: (____/ //(: (____/ //  \  |___
//  /|__/ \   \        /  \        /  ( \_|:  \
// (_______)   \"_____/    \"_____/    \_______)

#[test]
fn full_simulation() {
    let setup = Setup::default();

    // Config
    let epochs = 1000;
    let epoch_length = ONE_MINUTE as u64;
    let btc_prices = Vec::from_array(&setup.env, [50_000]);
    let xlm_prices = Vec::from_array(&setup.env, [50]);

    // Trackers
    let buffer_balance = setup.token2.balance(id);

    // Simulation
    for i in 1..epochs as usize {
        let now = setup.env.ledger().timestamp();

        // Update oracle prices
        let btc_price = btc_prices.get(i).unwrap();
        let xlm_price = xlm_prices.get(i).unwrap();
        setup.oracle_client.set_price(&Vec::from_array(&setup.env, [btc_price, xlm_price]), &now);

        // ...

        // Execute swaps
         setup.fee_collector.swap();

        // Move time forward
        jump(&setup.env, &epoch_length);
    }

    // Assertions

    // [ ] Pool price

    // [ ] Buffer reserve and balance

    // [ ] Insurance Fund premiums paid

    // [ ] Fee Collector revenue
    assert_eq!()
}

#[test]
fn test_swap() {
    let setup = Setup::default();
    let users = setup.users;

    // Test values
    let in_amount = 10_0000000;
    let out_min = 2_8952731; // FIXME:

    // Collect pre-swap values
    let buffer_reserve = setup.buffer.get_reserve(&setup.token2.address);
    let if_total_shares = setup.insurance_fund.get_total_shares();
    let if_balance = setup.token2.balance(&setup.insurance_fund.address);
    let fee_collector_balance = setup.token2.balance(&setup.fee_collector.address);
    let incentives_info = setup.router.get_rewards_info(user, tokens, pool_index);
    let pool_reserves = setup.router.get_reserves(tokens, &setup.pool_index);
    let pool_info = setup.router.get_info(tokens, pool_index);
    let btc_price = setup.oracle_registry.get_price(
        &setup.admin,
        &setup.btc_asset_id,
        &false,
        &None
    );
    let xlm_price = setup.oracle_registry.get_price(
        &setup.admin,
        &setup.xlm_asset_id,
        &false,
        &None
    );
    let target_price = btc_price.price / xlm_price.price;

    // Swap
    let amount_out = setup.router.swap(
        &users[1],
        &tokens,
        &setup.token2.address,
        &setup.token1.address,
        &setup.pool_index,
        &in_amount,
        &out_min
    );

    /* Pool */

    // [x] Ensure pool price is still pegged to the oracle price
    let updated_reserves = setup.router.get_reserves(&setup.tokens, &setup.pool_index);
    let pool_price = updated_reserves.get(1).unwrap() / updated_reserves.get(0).unwrap();
    assert_eq!(pool_price, target_price);

    /* Fees */
    let fee_amount = in_amount * pool_info.pool.fee_fraction;
    let lp_fee = fee_amount * setup.buffer.get_lp_revenue_fraction();
    let buffer_fee = (fee_amount - lp_fee) * setup.buffer.get_buffer_fraction();
    let if_fee = 0; // TODO:
    let protocol_fee = fee_amount - lp_fee - buffer_fee - if_fee;

    /* Buffer */

    // [x] Ensure the Buffer received a deposit() of token2
    assert_eq!(setup.token2.balance(&setup.buffer.address), buffer_balance + buffer_fee);
    assert_eq!(setup.buffer.get_reserve(&setup.token2), Reserve {
        balance: buffer_reserve + buffer_fee,
        last_update_ts: now,
        total_inflow: buffer_reserve.total_inflow + buffer_fee,
        ..buffer_reserve
    });

    // [x] Ensure Buffer `resolve_deficit` event is emitted
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            setup.buffer.address.clone(),
            (
                Symbol::new(&setup.env, "resolve_deficit"),
                setup.fee_collector.address.clone(),
            ).into_val(&e),
            buffer_fee.into_val(&e),
        )]
    );

    /* Insurance Fund */

    // [x] Ensure the IF received a pay_premium() of token2
    assert_eq!(setup.token2.balance(&setup.insurance_fund.address), if_balance + if_fee);

    // [x] Ensure IF total shares remains unchanged
    assert_eq!(setup.insurance_fund.get_total_shares(), if_total_shares);

    // [x] Ensure IF `collect_premium` event is emitted
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            setup.insurance_fund.address.clone(),
            (
                Symbol::new(&setup.env, "collect_premium"),
                setup.fee_collector.address.clone(),
            ).into_val(&e),
            if_fee.into_val(&e),
        )]
    );

    /* Incentives */

    // [x] Ensure `fee_growth_b` incentive data is increased
    let fee_growth_b = lp_fee / pool_tokens::get_total_lp_tokens(&setup.env);
    let new_fee_growth_b = incentives_info.get("fee_b").unwrap() + fee_growth_b;
    assert_eq!(
        setup.router.get_rewards_info(user, tokens, pool_index).get("fee_b").unwrap(),
        new_fee_growth_b
    );

    /* Pool Swap Fee */

    // [x] Ensure any remaining fee is kept in the contract as revenue
    assert_eq!(
        setup.token2.balance(&setup.fee_collector.address),
        fee_collector_balance + protocol_fee
    );

    // [x] Ensure Fee Collector `charge_provider_fee` event is emitted
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            setup.fee_collector.address.clone(),
            (Symbol::new(&setup.env, "charge_provider_fee"), setup.token1.address.clone()).into_val(
                &e
            ),
            protocol_fee.into_val(&e),
        )]
    );
}

//  _______   ____  ____   _______   _______   _______   _______
// |   _  "\ ("  _||_ " | /"     "| /"     "| /"     "| /"      \
// (. |_)  :)|   (  ) : |(: ______)(: ______)(: ______)|:        |
// |:     \/ (:  |  | . ) \/    |   \/    |   \/    |  |_____/   )
// (|  _  \\  \\ \__/ //  // ___)   // ___)   // ___)_  //      /
// |: |_)  :) /\\ __ //\ (:  (     (:  (     (:      "||:  __   \
// (_______/ (__________) \__/      \__/      \_______)|__|  \___)

#[test]
fn test_buffer_resolve_liquidity_deficit_event() {
    let setup = Setup::default();

    let claim_amount = 100_0000000_u128;

    // Setup Buffer with more then enough reserves
    setup.buffer.deposit(&setup.admin, &setup.token2.address, &(claim_amount * 2));

    // request_payout
    setup.buffer.resolve_liquidity_deficit(
        &setup.admin,
        &setup.token2.address,
        &claim_amount,
        &setup.pool_address
    );
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            setup.buffer.address.clone(),
            (
                Symbol::new(&setup.env, "resolve_liquidity_deficit"),
                setup.token2.address.clone(),
                setup.admin.clone(),
            ).into_val(&setup.env),
            claim_amount.into_val(&setup.env),
        )]
    );
}

#[test]
fn test_resolve_deficit_with_buffer_with_enough_funds() {
    let setup = Setup::default();

    let claim_amount = 100_0000000_u128;

    // Setup Buffer with more then enough reserves
    setup.buffer.deposit(&setup.admin, &setup.token2.address, &(claim_amount * 2));

    // Collect pre-claim values
    let buffer_reserve = setup.buffer.get_reserve(&setup.token2.address);
    let buffer_balance = setup.token2.balance(&setup.buffer.address);
    let pool_token1_balance = setup.token1.balance(&setup.pool_address);
    let pool_token2_balance = setup.token2.balance(&setup.pool_address);
    let pool_reserves = setup.router.get_reserves(&setup.tokens, &setup.pool_index);
    let pool_info = setup.router.get_info(tokens, pool_index);

    let btc_price = setup.oracle_registry.get_price(
        &setup.admin,
        &setup.btc_asset_id,
        &false,
        &None
    );
    let xlm_price = setup.oracle_registry.get_price(
        &setup.admin,
        &setup.xlm_asset_id,
        &false,
        &None
    );
    let target_price = btc_price.price / xlm_price.price;

    // File a claim
    let paid = setup.buffer.resolve_liquidity_deficit(
        &setup.admin,
        &setup.token2.address,
        &claim_amount,
        &setup.pool_address
    );

    // [x] Ensure Buffer reserve is updated
    let now = setup.env.ledger().timestamp();
    assert_eq!(setup.buffer.get_reserve(&setup.token2.address), Reserve {
        balance: buffer_reserve - paid,
        last_payout: paid,
        last_payout_ts: now,
        last_update_ts: now,
        total_outflow: buffer_reserve.total_outflow + paid,
        ..buffer_reserve
    });

    // [x] Ensure Buffer token2 balance is decreased
    assert_eq!(setup.token2.balance(&setup.buffer.address), buffer_balance - paid);

    // [ ] Ensure Claim event is emitted
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            setup.buffer.address.clone(),
            (
                Symbol::new(&setup.env, "resolve_deficit"),
                user1.clone(),
                StakeAction::Deposit,
            ).into_val(&e),
            amount_to_deposit.into_val(&e),
        )]
    );

    // TODO: compute expected min/burn token1 amount
    let token1_delta = 0;

    // [x] Ensure Pool reserves are updated
    let reserves = setup.router.get_reserves(&setup.tokens, &setup.pool_index);
    assert_eq!(reserves.get(0).unwrap(), pool_reserves.get(0).unwrap() + token1_delta);
    assert_eq!(reserves.get(1).unwrap(), pool_reserves.get(1).unwrap() + paid);

    // [x] Ensure Pool price peg is maintained
    let pool_price = reserves.get(1).unwrap() / reserves.get(0).unwrap();
    assert_eq!(pool_price, target_price);

    // [x] Ensure Pool token balances match
    assert_eq!(setup.token1.balance(&setup.pool_address), pool_token1_balance + token1_delta);
    assert_eq!(setup.token2.balance(&setup.pool_address), pool_token2_balance + paid);

    // [ ] Ensure Pool insurance claim is updated
    assert_eq!(
        setup.router.get_info(&setup.tokens, &setup.pool_index).insurance_claim,
        InsuranceClaim {
            last_revenue_withdraw_ts: now,
            quote_max_insurance: pool_info.insurance_claim.quote_max_insurance,
            quote_settled_insurance: pool_info.insurance_claim.quote_settled_insurance + paid,
            rev_withdraw_since_last_settle: 0,
        }
    );

    // [ ] Ensure Pool rebalance event is emitted
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            setup.pool_address.clone(),
            (Symbol::new(&setup.env, "rebalance"), user1.clone()).into_val(&e),
            amount_to_deposit.into_val(&e),
        )]
    );
}

//   __    _____  ___    ________  ____  ____   _______        __      _____  ___    ______    _______
//  |" \  (\"   \|"  \  /"       )("  _||_ " | /"      \      /""\    (\"   \|"  \  /" _  "\  /"     "|
//  ||  | |.\\   \    |(:   \___/ |   (  ) : ||:        |    /    \   |.\\   \    |(: ( \___)(: ______)
//  |:  | |: \.   \\  | \___  \   (:  |  | . )|_____/   )   /' /\  \  |: \.   \\  | \/ \      \/    |
//  |.  | |.  \    \. |  __/  \\   \\ \__/ //  //      /   //  __'  \ |.  \    \. | //  \ _   // ___)_
//  /\  |\|    \    \ | /" \   :)  /\\ __ //\ |:  __   \  /   /  \\  \|    \    \ |(:   _) \ (:      "|
// (__\_|_)\___|\____\)(_______/  (__________)|__|  \___)(___/    \___)\___|\____\) \_______) \_______)

#[test]
fn test_resolve_deficit_with_insurance_fund_with_enough_funds() {
    let setup = Setup::default();
    let users = setup.users;

    let stake_amount = 100_0000000_u128;
    let claim_amount = 100_0000000_u128;

    // Setup IF with more then enough stakes
    for user in users {
        setup.insurance_fund.deposit(&user, &stake_amount);
    }

    // Collect pre-claim values
    let if_total_shares = setup.insurance_fund.get_total_shares();
    let if_balance = setup.token2.balance(&setup.insurance_fund.address);
    let if_rate = setup.insurance_fund.get_rate();
    let if_optimal_utilization = setup.insurance_fund.get_optimal_utilization();
    let pool_token1_balance = setup.token1.balance(&setup.pool_address);
    let pool_token2_balance = setup.token2.balance(&setup.pool_address);
    let pool_reserves = setup.router.get_reserves(&setup.tokens, &setup.pool_index);
    let pool_info = setup.router.get_info(tokens, pool_index);

    let btc_price = setup.oracle_registry.get_price(
        &setup.admin,
        &setup.btc_asset_id,
        &false,
        &None
    );
    let xlm_price = setup.oracle_registry.get_price(
        &setup.admin,
        &setup.xlm_asset_id,
        &false,
        &None
    );
    let target_price = btc_price.price / xlm_price.price;

    // File a claim
    let paid = setup.insurance_fund.resolve_liquidity_deficit(&setup.admin, &setup.pool_address);

    // [x] Ensure IF total shares is unchanged
    assert_eq!(setup.insurance_fund.get_total_shares(), if_total_shares);

    // TODO: should we check these for explicit equality instead of not equals?
    // [x] Ensure IF rate is updated for new balance
    assert_ne!(setup.insurance_fund.get_rate(), if_rate);

    // [x] Ensure IF optimal utilization is updated
    assert_ne!(setup.insurance_fund.get_optimal_utilization(), if_optimal_utilization);

    // [x] Ensure IF token2 balance is decreased
    assert_eq!(setup.token2.balance(&setup.insurance_fund.address), if_balance - paid);

    // [ ] Ensure Claim event is emitted
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            setup.insurance_fund.address.clone(),
            (
                Symbol::new(&setup.env, "resolve_deficit"),
                user1.clone(),
                StakeAction::Deposit,
            ).into_val(&e),
            amount_to_deposit.into_val(&e),
        )]
    );

    // TODO: compute expected min/burn token1 amount
    let token1_delta = 0;

    // [x] Ensure Pool reserves are updated
    let reserves = setup.router.get_reserves(&setup.tokens, &setup.pool_index);
    assert_eq!(reserves.get(0).unwrap(), pool_reserves.get(0).unwrap() + token1_delta);
    assert_eq!(reserves.get(1).unwrap(), pool_reserves.get(1).unwrap() + paid);

    // [x] Ensure Pool price peg is maintained
    let pool_price = reserves.get(1).unwrap() / reserves.get(0).unwrap();
    assert_eq!(pool_price, target_price);

    // [x] Ensure Pool token balances match
    assert_eq!(setup.token1.balance(&setup.pool_address), pool_token1_balance + token1_delta);
    assert_eq!(setup.token2.balance(&setup.pool_address), pool_token2_balance + paid);

    // [ ] Ensure Pool insurance claim is updated
    assert_eq!(
        setup.router.get_info(&setup.tokens, &setup.pool_index).insurance_claim,
        InsuranceClaim {
            last_revenue_withdraw_ts: now,
            quote_max_insurance: pool_info.insurance_claim.quote_max_insurance,
            quote_settled_insurance: pool_info.insurance_claim.quote_settled_insurance + paid,
            rev_withdraw_since_last_settle: 0,
        }
    );

    // [ ] Ensure Pool rebalance event is emitted
    assert_eq!(
        vec![&setup.env, setup.env.events().all().last().unwrap()],
        vec![&setup.env, (
            setup.pool_address.clone(),
            (Symbol::new(&setup.env, "rebalance"), user1.clone()).into_val(&e),
            amount_to_deposit.into_val(&e),
        )]
    );
}

#[test]
fn test_integration() {
    let setup = Setup::default();

    xlm_admin.mint(&setup.admin, &344_000_0000000);
    pool.deposit(&setup.admin, &100_000_0000000);

    // swap through many pools at once
    let user = Address::generate(&setup.env);
    xlm_admin.mint(&user, &10_0000000);

    let (pool_index, _pool_address) = setup.router.get_pools(&tokens).iter().last().unwrap();

    assert_eq!(
        setup.router.swap(
            &user,
            &tokens,
            &nbtc.address,
            &xlm.address,
            &pool_index,
            &10_0000000,
            &2_8952731
        ),
        2_8952731
    );

    // now swap with additional provider fee
    xlm_admin.mint(&user, &10_0000000);
    assert_eq!(
        swap_fee.swap(
            &user,
            &(
                vec![&setup.env, xlm.address.clone(), nbtc.address.clone()],
                pool_hash.clone(),
                nbtc.address.clone(),
            ),
            &xlm.address,
            &10_0000000,
            &2_8864196,
            &30
        ),
        2_8864196
    );
}

//    _______    ______      ______    ___
//   |   __ "\  /    " \    /    " \  |"  |
//   (. |__) :)// ____  \  // ____  \ ||  |
//   |:  ____//  /    ) :)/  /    ) :)|:  |
//   (|  /   (: (____/ //(: (____/ //  \  |___
//  /|__/ \   \        /  \        /  ( \_|:  \
// (_______)   \"_____/    \"_____/    \_______)
