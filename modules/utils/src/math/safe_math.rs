use soroban_sdk::{log, panic_with_error, Env};

use crate::errors::math_errors::MathError;

pub trait SafeMath: Sized {
    fn safe_add(self, e: &Env, rhs: Self) -> Self; // instead of Result<Self, ()> since it either returns Self or panics (no return)
    fn safe_sub(self, e: &Env, rhs: Self) -> Self;
    fn safe_mul(self, e: &Env, rhs: Self) -> Self;
    fn safe_div(self, e: &Env, rhs: Self) -> Self;
}

pub trait PrecisionMath: Sized {
    // Use ceiling when calculating fees (favor protocol)
    fn safe_fixed_mul_ceil(self, e: &Env, other: Self, precision: Self) -> Self;
    // Use floor when calculating user benefits (conservative)  
    fn safe_fixed_mul_floor(self, e: &Env, other: Self, precision: Self) -> Self;
    // Use round-to-nearest for price calculations (most accurate)
    fn safe_fixed_mul_round(self, e: &Env, other: Self, precision: Self) -> Self;
    // Division variants
    fn safe_fixed_div_ceil(self, e: &Env, other: Self, precision: Self) -> Self;
    fn safe_fixed_div_floor(self, e: &Env, other: Self, precision: Self) -> Self;
    fn safe_fixed_div_round(self, e: &Env, other: Self, precision: Self) -> Self;
}

pub trait SafeConversion {
    fn safe_to_u128(self, e: &Env) -> u128;
    fn safe_to_u64(self, e: &Env) -> u64;
    fn safe_to_u32(self, e: &Env) -> u32;
    fn safe_to_i128(self, e: &Env) -> i128;
    fn safe_to_i64(self, e: &Env) -> i64;
    fn safe_to_i32(self, e: &Env) -> i32;
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
                        log!(e, "Addition overflow at {}:{}", file!(), line!());
                        panic_with_error!(e, MathError::AdditionOverflow);
                    }
                }
            }

            #[track_caller]
            #[inline(always)]
            fn safe_sub(self, e: &Env, v: $t) -> $t {
                match self.checked_sub(v) {
                    Some(result) => result,
                    None => {
                        log!(e, "Subtraction underflow at {}:{}", file!(), line!());
                        panic_with_error!(e, MathError::SubtractionUnderflow);
                    }
                }
            }

            #[track_caller]
            #[inline(always)]
            fn safe_mul(self, e: &Env, v: $t) -> $t {
                match self.checked_mul(v) {
                    Some(result) => result,
                    None => {
                        log!(e, "Multiplication overflow at {}:{}", file!(), line!());
                        panic_with_error!(e, MathError::MultiplicationOverflow);
                    }
                }
            }

            #[track_caller]
            #[inline(always)]
            fn safe_div(self, e: &Env, v: $t) -> $t {
                match self.checked_div(v) {
                    Some(result) => result,
                    None => {
                        log!(e, "Division by zero at {}:{}", file!(), line!());
                        panic_with_error!(e, MathError::DivisionByZero);
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

// Safe conversion implementations
impl SafeConversion for i128 {
    #[track_caller]
    fn safe_to_u128(self, e: &Env) -> u128 {
        if self < 0 {
            log!(e, "Attempted to convert negative i128 to u128: {} at {}:{}", self, file!(), line!());
            panic_with_error!(e, MathError::NegativeToUnsigned);
        }
        u128::try_from(self).unwrap_or_else(|_| {
            log!(e, "i128 to u128 conversion overflow: {} at {}:{}", self, file!(), line!());
            panic_with_error!(e, MathError::ConversionOverflow);
        })
    }

    #[track_caller]
    fn safe_to_u64(self, e: &Env) -> u64 {
        if self < 0 {
            log!(e, "Attempted to convert negative i128 to u64: {} at {}:{}", self, file!(), line!());
            panic_with_error!(e, MathError::NegativeToUnsigned);
        }
        u64::try_from(self).unwrap_or_else(|_| {
            log!(e, "i128 to u64 conversion overflow: {} at {}:{}", self, file!(), line!());
            panic_with_error!(e, MathError::ConversionOverflow);
        })
    }

    #[track_caller]
    fn safe_to_u32(self, e: &Env) -> u32 {
        if self < 0 {
            log!(e, "Attempted to convert negative i128 to u32: {} at {}:{}", self, file!(), line!());
            panic_with_error!(e, MathError::NegativeToUnsigned);
        }
        u32::try_from(self).unwrap_or_else(|_| {
            log!(e, "i128 to u32 conversion overflow: {} at {}:{}", self, file!(), line!());
            panic_with_error!(e, MathError::ConversionOverflow);
        })
    }

    #[track_caller]
    fn safe_to_i128(self, _e: &Env) -> i128 {
        self
    }

    #[track_caller]
    fn safe_to_i64(self, e: &Env) -> i64 {
        i64::try_from(self).unwrap_or_else(|_| {
            log!(e, "i128 to i64 conversion overflow: {} at {}:{}", self, file!(), line!());
            panic_with_error!(e, MathError::ConversionOverflow);
        })
    }

    #[track_caller]
    fn safe_to_i32(self, e: &Env) -> i32 {
        i32::try_from(self).unwrap_or_else(|_| {
            log!(e, "i128 to i32 conversion overflow: {} at {}:{}", self, file!(), line!());
            panic_with_error!(e, MathError::ConversionOverflow);
        })
    }
}

impl SafeConversion for u128 {
    #[track_caller]
    fn safe_to_u128(self, _e: &Env) -> u128 {
        self
    }

    #[track_caller]
    fn safe_to_u64(self, e: &Env) -> u64 {
        u64::try_from(self).unwrap_or_else(|_| {
            log!(e, "u128 to u64 conversion overflow: {} at {}:{}", self, file!(), line!());
            panic_with_error!(e, MathError::ConversionOverflow);
        })
    }

    #[track_caller]
    fn safe_to_u32(self, e: &Env) -> u32 {
        u32::try_from(self).unwrap_or_else(|_| {
            log!(e, "u128 to u32 conversion overflow: {} at {}:{}", self, file!(), line!());
            panic_with_error!(e, MathError::ConversionOverflow);
        })
    }

    #[track_caller]
    fn safe_to_i128(self, e: &Env) -> i128 {
        i128::try_from(self).unwrap_or_else(|_| {
            log!(e, "u128 to i128 conversion overflow: {} at {}:{}", self, file!(), line!());
            panic_with_error!(e, MathError::ConversionOverflow);
        })
    }

    #[track_caller]
    fn safe_to_i64(self, e: &Env) -> i64 {
        i64::try_from(self).unwrap_or_else(|_| {
            log!(e, "u128 to i64 conversion overflow: {} at {}:{}", self, file!(), line!());
            panic_with_error!(e, MathError::ConversionOverflow);
        })
    }

    #[track_caller]
    fn safe_to_i32(self, e: &Env) -> i32 {
        i32::try_from(self).unwrap_or_else(|_| {
            log!(e, "u128 to i32 conversion overflow: {} at {}:{}", self, file!(), line!());
            panic_with_error!(e, MathError::ConversionOverflow);
        })
    }
}

// Precision-aware math implementations
impl PrecisionMath for u128 {
    #[track_caller]
    fn safe_fixed_mul_ceil(self, e: &Env, other: u128, precision: u128) -> u128 {
        // Calculate (self * other + precision - 1) / precision
        let product = self.checked_mul(other).unwrap_or_else(|| {
            log!(e, "Fixed-point multiplication overflow: {} * {} at {}:{}", self, other, file!(), line!());
            panic_with_error!(e, MathError::FixedPointOverflow);
        });
        
        let numerator = product.checked_add(precision - 1).unwrap_or_else(|| {
            log!(e, "Fixed-point ceiling calculation overflow at {}:{}", file!(), line!());
            panic_with_error!(e, MathError::FixedPointOverflow);
        });
        
        numerator.checked_div(precision).unwrap_or_else(|| {
            log!(e, "Fixed-point division by zero at {}:{}", file!(), line!());
            panic_with_error!(e, MathError::DivisionByZero);
        })
    }
    
    #[track_caller]
    fn safe_fixed_mul_floor(self, e: &Env, other: u128, precision: u128) -> u128 {
        // Calculate (self * other) / precision
        let product = self.checked_mul(other).unwrap_or_else(|| {
            log!(e, "Fixed-point multiplication overflow: {} * {} at {}:{}", self, other, file!(), line!());
            panic_with_error!(e, MathError::FixedPointOverflow);
        });
        
        product.checked_div(precision).unwrap_or_else(|| {
            log!(e, "Fixed-point division by zero at {}:{}", file!(), line!());
            panic_with_error!(e, MathError::DivisionByZero);
        })
    }
    
    #[track_caller]
    fn safe_fixed_mul_round(self, e: &Env, other: u128, precision: u128) -> u128 {
        // Calculate (self * other + precision / 2) / precision for round-to-nearest
        let product = self.checked_mul(other).unwrap_or_else(|| {
            log!(e, "Fixed-point multiplication overflow: {} * {} at {}:{}", self, other, file!(), line!());
            panic_with_error!(e, MathError::FixedPointOverflow);
        });
        
        let half_precision = precision / 2;
        let numerator = product.checked_add(half_precision).unwrap_or_else(|| {
            log!(e, "Fixed-point rounding calculation overflow at {}:{}", file!(), line!());
            panic_with_error!(e, MathError::FixedPointOverflow);
        });
        
        numerator.checked_div(precision).unwrap_or_else(|| {
            log!(e, "Fixed-point division by zero at {}:{}", file!(), line!());
            panic_with_error!(e, MathError::DivisionByZero);
        })
    }
    
    #[track_caller]
    fn safe_fixed_div_ceil(self, e: &Env, other: u128, precision: u128) -> u128 {
        // Calculate (self * precision + other - 1) / other
        let numerator = self.checked_mul(precision).unwrap_or_else(|| {
            log!(e, "Fixed-point division numerator overflow: {} * {} at {}:{}", self, precision, file!(), line!());
            panic_with_error!(e, MathError::FixedPointOverflow);
        });
        
        let adjusted_numerator = numerator.checked_add(other - 1).unwrap_or_else(|| {
            log!(e, "Fixed-point ceiling division overflow at {}:{}", file!(), line!());
            panic_with_error!(e, MathError::FixedPointOverflow);
        });
        
        adjusted_numerator.checked_div(other).unwrap_or_else(|| {
            log!(e, "Fixed-point division by zero at {}:{}", file!(), line!());
            panic_with_error!(e, MathError::DivisionByZero);
        })
    }
    
    #[track_caller]
    fn safe_fixed_div_floor(self, e: &Env, other: u128, precision: u128) -> u128 {
        // Calculate (self * precision) / other
        let numerator = self.checked_mul(precision).unwrap_or_else(|| {
            log!(e, "Fixed-point division numerator overflow: {} * {} at {}:{}", self, precision, file!(), line!());
            panic_with_error!(e, MathError::FixedPointOverflow);
        });
        
        numerator.checked_div(other).unwrap_or_else(|| {
            log!(e, "Fixed-point division by zero at {}:{}", file!(), line!());
            panic_with_error!(e, MathError::DivisionByZero);
        })
    }
    
    #[track_caller]
    fn safe_fixed_div_round(self, e: &Env, other: u128, precision: u128) -> u128 {
        // Calculate (self * precision + other / 2) / other for round-to-nearest
        let numerator = self.checked_mul(precision).unwrap_or_else(|| {
            log!(e, "Fixed-point division numerator overflow: {} * {} at {}:{}", self, precision, file!(), line!());
            panic_with_error!(e, MathError::FixedPointOverflow);
        });
        
        let half_other = other / 2;
        let adjusted_numerator = numerator.checked_add(half_other).unwrap_or_else(|| {
            log!(e, "Fixed-point rounding division overflow at {}:{}", file!(), line!());
            panic_with_error!(e, MathError::FixedPointOverflow);
        });
        
        adjusted_numerator.checked_div(other).unwrap_or_else(|| {
            log!(e, "Fixed-point division by zero at {}:{}", file!(), line!());
            panic_with_error!(e, MathError::DivisionByZero);
        })
    }
}

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
