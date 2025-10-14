// use soroban_fixed_point_math::SorobanFixedPoint;
// use soroban_sdk::{contracttype, Address, Env};
// use utils::{
//     bump::bump_persistent,
//     constant::{PRICE_PRECISION, PRICE_PRECISION_I64},
//     math::safe_math::{SafeConversion, SafeMath},
// };

// use crate::storage::{get_bonus_reserve_b, get_max_bonus_fraction};

// #[derive(Clone)]
// #[contracttype]
// pub struct Bonus {
//     pub user: Address,
//     pub amount: u128,     // in token units (scaled integer)
//     pub updated_at: u64,  // ledger timestamp or sequence
//     pub valid_after: u64, // timestamp + vesting_delay
//     pub pre_dev: i128,    // deviation before trade (scaled)
//     pub post_dev: i128,   // deviation after trade (scaled)
// }

// impl Bonus {
//     pub fn new(user: Address) -> Self {
//         Bonus {
//             user,
//             amount: 0,
//             updated_at: 0,
//             valid_after: 0,
//             pre_dev: 0,
//             post_dev: 0,
//         }
//     }

//     pub fn save(&mut self, e: &Env) {
//         save_bonus(e, &self.user, &self);
//     }

//     pub fn claim(&mut self, current_time: u64) {
//         self.amount = 0;
//         self.updated_at = current_time;
//     }

//     pub fn record(&mut self, amount: u128, vesting_delay: u64, current_time: u64) {
//         self.amount = 0;
//         self.valid_after = current_time + vesting_delay;
//         self.updated_at = current_time;
//     }
// }

// pub fn calculate_bonus_rate(e: &Env, pool_price: u128, peg_price: u128) -> u32 {
//     if pool_price == 0 || peg_price == 0 {
//         return 0;
//     }

//     // let deviation = (pool_price / peg_price) - 1.0;
//     let ratio = pool_price.safe_div(e, peg_price);
//     let deviation = (ratio as i128).safe_sub(e, 1);
//     let abs_dev = deviation.abs();

//     let max_bonus = get_max_bonus_fraction(e);
//     let k = 25.0;

//     // let rate = (max_bonus).safe_mul(e, (1.safe_sub(e, (-k.))))
//     (max_bonus) * (1.0 - (-k * abs_dev).exp())
// }

// pub fn calculate_bonus_amount(
//     e: &Env,
//     bonus_rate: u32,
//     trade_amount: u128,
// ) -> u128 {
//     // let base_bonus_rate = crate::bonus::calculate_bonus_rate(&e, pool_price, peg_price);
//     0
// }

// pub fn record_bonus(
//     e: &Env,
//     user: &Address,
//     pool_price: u128,
//     peg_price: u128,
//     trade_amount: u128,
//     current_time: u64,
// ) {
//     let vesting_delay = 60; // TODO: move to storage.rs

//     let bonus_rate = calculate_bonus_rate(e, pool_price, peg_price);

//     let bonus_reserve = get_bonus_reserve_b(e);
//     let bonus_amount = calculate_bonus_amount(e, bonus_rate, trade_amount);

//     if bonus_amount > 0 {
//         let mut bonus = get_bonus(e, user);
//         bonus.record(bonus_amount, vesting_delay, current_time);
//         bonus.save(e);
//     }
// }

// // Storage

// pub fn get_bonus(e: &Env, user: &Address) -> Bonus {
//     let key = user;
//     let stake_info = match e.storage().persistent().get::<_, Bonus>(&key) {
//         Some(stake) => {
//             bump_persistent(e, &key);
//             stake
//         }
//         None => Bonus::new(user.clone()),
//     };

//     stake_info
// }

// pub fn save_bonus(e: &Env, user: &Address, bonus_info: &Bonus) {
//     let key = user;
//     e.storage().persistent().set(&key, bonus_info);
//     bump_persistent(e, &key);
// }
