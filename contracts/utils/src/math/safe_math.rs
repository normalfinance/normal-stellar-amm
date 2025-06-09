use soroban_sdk::{log, panic_with_error, Env};

use crate::errors::math_errors::MathError;

pub trait SafeMath: Sized {
    fn safe_add(self, e: &Env, rhs: Self) -> Self; // instead of Result<Self, ()> since it either returns Self or panics (no return)
    fn safe_sub(self, e: &Env, rhs: Self) -> Self;
    fn safe_mul(self, e: &Env, rhs: Self) -> Self;
    fn safe_div(self, e: &Env, rhs: Self) -> Self;
}

macro_rules! checked_impl {
    ($t:ty) => {
        impl SafeMath for $t {
            #[track_caller]
            #[inline(always)]
            fn safe_add(self, e: &Env, v: $t) -> $t {
                match self.checked_add(v) {
                    Some(result) => result,
                    None => {
                        log!(e, "Math error thrown at {}:{}", file!(), line!());
                        panic_with_error!(e, MathError::MathError);
                    }
                }
            }

            #[track_caller]
            #[inline(always)]
            fn safe_sub(self, e: &Env, v: $t) -> $t {
                match self.checked_sub(v) {
                    Some(result) => result,
                    None => {
                        log!(e, "Math error thrown at {}:{}", file!(), line!());
                        panic_with_error!(e, MathError::MathError);
                    }
                }
            }

            #[track_caller]
            #[inline(always)]
            fn safe_mul(self, e: &Env, v: $t) -> $t {
                match self.checked_mul(v) {
                    Some(result) => result,
                    None => {
                        log!(e, "Math error thrown at {}:{}", file!(), line!());
                        panic_with_error!(e, MathError::MathError);
                    }
                }
            }

            #[track_caller]
            #[inline(always)]
            fn safe_div(self, e: &Env, v: $t) -> $t {
                match self.checked_div(v) {
                    Some(result) => result,
                    None => {
                        log!(e, "Math error thrown at {}:{}", file!(), line!());
                        panic_with_error!(e, MathError::MathError);
                    }
                }
            }
        }
    };
}

checked_impl!(u128);
checked_impl!(u64);
checked_impl!(u32);
checked_impl!(i128);
checked_impl!(i64);
checked_impl!(i32);
#[cfg(test)]
mod test {
    use crate::math::safe_math::{MathError, SafeMath};
    use soroban_sdk::Env;

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn safe_add() {
        let e = Env::default();
        assert_eq!((1_u128).safe_add(&e, 1), 2);
        // assert_eq!((1_u128).safe_add(u128::MAX, &env), Err(MathError::MathError));
        // TODO:
        assert_eq!((1_u128).safe_add(&e, u128::MAX), 0);
    }

    #[test]
    fn safe_sub() {
        let e = Env::default();
        assert_eq!((1_u128).safe_sub(&e, 1), 0);
        // assert_eq!((0_u128).safe_sub(1), Err(MathError::MathError));
    }

    #[test]
    fn safe_mul() {
        let e = Env::default();
        assert_eq!((8_u128).safe_mul(&e, 80), 640);
        assert_eq!((1_u128).safe_mul(&e, 1), 1);
        // assert_eq!((2_u128).safe_mul(u128::MAX), Err(MathError::MathError));
    }

    #[test]
    fn safe_div() {
        let e = Env::default();
        assert_eq!((155_u128).safe_div(&e, 8), 19);
        assert_eq!((159_u128).safe_div(&e, 8), 19);
        assert_eq!((160_u128).safe_div(&e, 8), 20);

        assert_eq!((1_u128).safe_div(&e, 1), 1);
        assert_eq!((1_u128).safe_div(&e, 100), 0);
        // assert_eq!((1_u128).safe_div(0), Err(MathError::MathError));
    }
}
