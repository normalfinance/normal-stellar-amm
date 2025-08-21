use soroban_sdk::{contracttype, Address, Vec};

#[contracttype]
#[derive(Clone)]
pub struct PrivilegedAddresses {
    pub emergency_admin: Address,
    pub rewards_admin: Address,
    pub operations_admin: Address,
    pub pause_admin: Address,
    pub emergency_pause_admins: Vec<Address>,
}
