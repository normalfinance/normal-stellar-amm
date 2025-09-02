// #![cfg(test)]
// extern crate std;

// use crate::testutils::Setup;
// use soroban_sdk::testutils::Events;
// use soroban_sdk::{vec, IntoVal, Symbol};
// use utils::constant::PRICE_PRECISION;

// /* Swap tests are located in /integration_tests since `swap()` can only
// truly be done setting up all other contracts */

// /* Tests Needed:
// - [ ] Init admin cannot call twice
// - [ ] Setters works
// - [ ] Getters work
//  */

// #[test]
// fn test_claim_fee() {
//     let setup = Setup::default();
//     let e = setup.env;
//     let fee_amount = 100 * PRICE_PRECISION;

//     // Mint tokens to the Fee Collector to simulate collected fees
//     setup
//         .token_b_admin_client
//         .mint(&setup.fee_collector.address, &(fee_amount as i128));

//     // [x] Ensure `claim_fee()` on empty token balance does nothing
//     assert_eq!(
//         setup
//             .fee_collector
//             .claim_fees(&setup.admin, &setup.token_a.address),
//         0
//     );
//     assert_eq!(setup.token_a.balance(&setup.fee_destination), 0);

//     // [x] Ensure `claim_fee()` on existing token balance transfers the whole amount to the `fee_destination`
//     assert_eq!(
//         setup
//             .fee_collector
//             .claim_fees(&setup.admin, &setup.token_b.address),
//         fee_amount
//     );
//     assert_eq!(
//         setup.token_b.balance(&setup.fee_destination),
//         fee_amount as i128
//     );

//     // [x] Ensure the `withdraw_fee` event is emitted
//     assert_eq!(
//         vec![&e, e.events().all().last().unwrap()],
//         vec![
//             &e,
//             (
//                 setup.fee_collector.address.clone(),
//                 (Symbol::new(&e, "withdraw_fee"),).into_val(&e),
//                 (setup.token_b.address, fee_amount).into_val(&e),
//             )
//         ]
//     );
// }
