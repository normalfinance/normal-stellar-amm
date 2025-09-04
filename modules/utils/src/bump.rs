use crate::constant::{
    INSTANCE_TTL_THRESHOLD, MAX_INSTANCE_TTL, MAX_PERSISTENT_TTL, MAX_TEMPORARY_TTL,
    PERSISTENT_TTL_THRESHOLD, TEMPORARY_TTL_THRESHOLD,
};
use soroban_sdk::{Env, IntoVal, Val};

pub fn bump_instance(e: &Env) {
    // NOTE: TTL extension operations are performed automatically during read operations
    // Authorization is handled at the contract function level (e.g., admin-only operations)
    // rather than at the TTL extension level to maintain operational efficiency
    e.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, MAX_INSTANCE_TTL);
}

pub fn bump_persistent<K>(e: &Env, key: &K)
where
    K: IntoVal<Env, Val>,
{
    e.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, MAX_PERSISTENT_TTL);
}

pub fn bump_temporary<K>(e: &Env, key: &K)
where
    K: IntoVal<Env, Val>,
{
    e.storage()
        .temporary()
        .extend_ttl(key, TEMPORARY_TTL_THRESHOLD, MAX_TEMPORARY_TTL);
}
