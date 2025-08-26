use crate::errors::storage_errors::StorageError;
use soroban_sdk::panic_with_error;

#[macro_export]
macro_rules! generate_instance_storage_setter {
    ($attr_name:ident, $key:expr, $data_type:ty) => {
        paste! {
            pub fn [<set_ $attr_name>](e: &Env, $attr_name: &$data_type) {
                bump_instance(e);
                e.storage()
                    .instance()
                    .set(&$key, $attr_name)
            }
        }
    };
}

#[macro_export]
macro_rules! generate_instance_storage_getter {
    ($attr_name:ident, $key:expr, $data_type:ty) => {
        paste! {
            pub fn [<get_ $attr_name>](e: &Env) -> $data_type {
                bump_instance(e);
                let value_result = e.storage().instance().get(&$key);
                match value_result {
                    Some(value) => value,
                    None => {
                        panic_with_error!(e, StorageError::ValueNotInitialized)
                    }
                }
            }
        }
    };
}

#[macro_export]
macro_rules! generate_instance_storage_getter_with_default {
    ($attr_name:ident, $key:expr, $data_type:ty, $default:expr) => {
        paste! {
            pub fn [<get_ $attr_name>](e: &Env) -> $data_type {
                bump_instance(e);
                e.storage().instance().get(&$key).unwrap_or($default)
            }
        }
    };
}

#[macro_export]
macro_rules! generate_instance_storage_getter_and_setter {
    ($attr_name:ident, $key:expr, $data_type:ty) => {
        generate_instance_storage_getter!($attr_name, $key, $data_type);
        generate_instance_storage_setter!($attr_name, $key, $data_type);
    };
}

#[macro_export]
macro_rules! generate_instance_storage_getter_and_setter_with_default {
    ($attr_name:ident, $key:expr, $data_type:ty, $default:expr) => {
        generate_instance_storage_getter_with_default!($attr_name, $key, $data_type, $default);
        generate_instance_storage_setter!($attr_name, $key, $data_type);
    };
}

// A macro that validates a condition and panics with a specific error if the condition is false
#[macro_export]
macro_rules! validate {
    ($env:expr, $condition:expr, $error:expr) => {
        if !$condition {
            #[cfg(debug_assertions)]
            panic_with_error!($env, $error) // Panic with the specified error
        }
    };
}
