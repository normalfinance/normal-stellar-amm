use soroban_sdk::panic_with_error;

use crate::errors::math_errors::MathError;

/// A macro that validates a condition, logs a message, and panics with a specific error if the condition is false
#[macro_export]
macro_rules! validate {
    ($env:expr, $condition:expr, $error:expr, $message:expr) => {
        if !$condition {
            // Log the validation failure message
            #[cfg(debug_assertions)]
            $env.log($message);
            // Panic with the specified error
            #[cfg(debug_assertions)]
            panic_with_error!($env, $error)
        }
    };
    // Version with format string and single data parameter
    ($env:expr, $condition:expr, $error:expr, $message:expr, $data:expr) => {
        {
        if !$condition {{
            #[cfg(debug_assertions)]
            $env.log(&format!($message, $data));
            #[cfg(debug_assertions)]
            panic_with_error!($env, $error)
        }}
        }
    };
    // Version with format string and multiple data parameters
    ($env:expr, $condition:expr, $error:expr, $message:expr, $($data:expr),+ $(,)?) => {
        {
        if !$condition {{
            #[cfg(debug_assertions)]
            $env.log(&format!($message, $($data),+));
            #[cfg(debug_assertions)]
            panic_with_error!($env, $error)
        }}
        }
    };
    // Variant without logging for cases where logging isn't needed
    ($env:expr, $condition:expr, $error:expr) => {
        if !$condition {
            #[cfg(debug_assertions)]
            panic_with_error!($env, $error)
        }
    };
}

#[macro_export]
macro_rules! safe_increment {
    ($env:expr, $struct:expr, $value:expr) => {
        {
        $struct = $struct.checked_add($value).unwrap_or_else(|| {
            #[cfg(debug_assertions)]
            panic_with_error!($env, MathError::MathError);
            $struct
        });
        }
    };
}

#[macro_export]
macro_rules! safe_decrement {
    ($env:expr, $struct:expr, $value:expr) => {
        {
        $struct = $struct.checked_sub($value).unwrap_or_else(|| {
            #[cfg(debug_assertions)]
            panic_with_error!($env, MathError::MathError);
            $struct
        });
        }
    };
}
