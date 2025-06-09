//  let oracle_guard_rails = OracleGuardRails {
//             price_divergence: PriceDivergenceGuardRails {
//                 oracle_twap_percent_divergence: PERCENTAGE_PRECISION_U64 / 2,
//             },
//             validity: ValidityGuardRails {
//                 slots_before_stale_for_pool: 10, // ~5 seconds
//                 confidence_interval_max_size: 20_000, // 2% of price
//                 too_volatile_ratio: 5, // 5x or 80% down
//             },
//         };