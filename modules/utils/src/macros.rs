use soroban_sdk::panic_with_error;

use crate::errors::math_errors::MathError;

//   __    _____  ___    ________  ___________   __      _____  ___    ______    _______
//  |" \  (\"   \|"  \  /"       )("     _   ") /""\    (\"   \|"  \  /" _  "\  /"     "|
//  ||  | |.\\   \    |(:   \___/  )__/  \\__/ /    \   |.\\   \    |(: ( \___)(: ______)
//  |:  | |: \.   \\  | \___  \       \\_ /   /' /\  \  |: \.   \\  | \/ \      \/    |
//  |.  | |.  \    \. |  __/  \\      |.  |  //  __'  \ |.  \    \. | //  \ _   // ___)_
//  /\  |\|    \    \ | /" \   :)     \:  | /   /  \\  \|    \    \ |(:   _) \ (:      "|
// (__\_|_)\___|\____\)(_______/       \__|(___/    \___)\___|\____\) \_______) \_______)

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

//    _______    _______   _______    ________  __      ________  ___________  _______  _____  ___  ___________
//   |   __ "\  /"     "| /"      \  /"       )|" \    /"       )("     _   ")/"     "|(\"   \|"  \("     _   ")
//   (. |__) :)(: ______)|:        |(:   \___/ ||  |  (:   \___/  )__/  \\__/(: ______)|.\\   \    |)__/  \\__/
//   |:  ____/  \/    |  |_____/   ) \___  \   |:  |   \___  \       \\_ /    \/    |  |: \.   \\  |   \\_ /
//   (|  /      // ___)_  //      /   __/  \\  |.  |    __/  \\      |.  |    // ___)_ |.  \    \. |   |.  |
//  /|__/ \    (:      "||:  __   \  /" \   :) /\  |\  /" \   :)     \:  |   (:      "||    \    \ |   \:  |
// (_______)    \_______)|__|  \___)(_______/ (__\_|_)(_______/       \__|    \_______) \___|\____\)    \__|

#[macro_export]
macro_rules! generate_persistent_storage_setter {
    ($attr_name:ident, $key:expr, $data_type:ty) => {
        paste! {
            pub fn [<set_ $attr_name>](e: &Env, $attr_name: &$data_type) {
                bump_persistent(e);
                e.storage()
                    .persistent()
                    .set(&$key, $attr_name)
            }
        }
    };
}

#[macro_export]
macro_rules! generate_persistent_storage_getter {
    ($attr_name:ident, $key:expr, $data_type:ty) => {
        paste! {
            pub fn [<get_ $attr_name>](e: &Env) -> $data_type {
                bump_persistent(e);
                let value_result = e.storage().persistent().get(&$key);
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
macro_rules! generate_persistent_storage_getter_with_default {
    ($attr_name:ident, $key:expr, $data_type:ty, $default:expr) => {
        paste! {
            pub fn [<get_ $attr_name>](e: &Env) -> $data_type {
                bump_persistent(e);
                e.storage().persistent().get(&$key).unwrap_or($default)
            }
        }
    };
}

#[macro_export]
macro_rules! generate_persistent_storage_getter_and_setter {
    ($attr_name:ident, $key:expr, $data_type:ty) => {
        generate_persistent_storage_getter!($attr_name, $key, $data_type);
        generate_persistent_storage_setter!($attr_name, $key, $data_type);
    };
}

#[macro_export]
macro_rules! generate_persistent_storage_getter_and_setter_with_default {
    ($attr_name:ident, $key:expr, $data_type:ty, $default:expr) => {
        generate_persistent_storage_getter_with_default!($attr_name, $key, $data_type, $default);
        generate_persistent_storage_setter!($attr_name, $key, $data_type);
    };
}

//  ___      ___  __      ___        __     ________       __  ___________  __      ______    _____  ___
// |"  \    /"  |/""\    |"  |      |" \   |"      "\     /""\("     _   ")|" \    /    " \  (\"   \|"  \
//  \   \  //  //    \   ||  |      ||  |  (.  ___  :)   /    \)__/  \\__/ ||  |  // ____  \ |.\\   \    |
//   \\  \/. .//' /\  \  |:  |      |:  |  |: \   ) ||  /' /\  \  \\_ /    |:  | /  /    ) :)|: \.   \\  |
//    \.    ////  __'  \  \  |___   |.  |  (| (___\ || //  __'  \ |.  |    |.  |(: (____/ // |.  \    \. |
//     \\   //   /  \\  \( \_|:  \  /\  |\ |:       :)/   /  \\  \\:  |    /\  |\\        /  |    \    \ |
//      \__/(___/    \___)\_______)(__\_|_)(________/(___/    \___)\__|   (__\_|_)\"_____/    \___|\____\)

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
