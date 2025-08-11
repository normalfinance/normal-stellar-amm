// #![cfg(test)]

// use crate::testutils::Setup;
// use soroban_sdk::testutils::Address as _;
// use soroban_sdk::{symbol_short, Address, Symbol, Vec};
// use utils::test_utils::{install_dummy_wasm, jump};

// #[test]
// fn test_buffer_resolve_liquidity_deficit() {
//     let setup = Setup::default();
//     let buffer = setup.buffer;
//     let token = Address::generate(&setup.env);
//     let user = Address::generate(&setup.env);
//     let pool_address = Address::generate(&setup.env);

//     for (addr, is_ok) in [(user, false), (setup.admin, true)] {
//         assert_eq!(
//             buffer
//                 .try_resolve_liquidity_deficit(&addr, &token, &1_000_u128, &pool_address)
//                 .is_ok(),
//             is_ok
//         );
//     }
// }
