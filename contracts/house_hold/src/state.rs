use soroban_sdk::{contracttype, vec, Address, Env, Vec};

#[derive(Clone, Debug)]
#[contracttype]
pub struct Batch {
    pub amount_per_beneficiary: u128,
    pub reduced_amount_per_beneficiary: u128,
}

#[derive(Clone, Debug)]
#[contracttype]
pub enum StorageKey {
    Initialized,
    Admin,
    IsContractPaused,
    UsdAddress,
    FundingAccount,
    FeeReceiver,
    FeePercent,
    Batches(u128),
    PaidAddresses(u128, Address),
}

pub fn set_initialized(e: &Env, initialized: &bool) {
    e.storage().instance().set(&StorageKey::Initialized, initialized);
}

pub fn get_initialized(e: &Env) -> bool {
    e.storage().instance().get::<_, bool>(&StorageKey::Initialized).unwrap_or(false)
}

pub fn set_admin(e: &Env, new_admin: &Address) {
    e.storage().instance().set(&StorageKey::Admin, new_admin);
}

/// Return the admin address.
pub fn get_admin(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<_, Address>(&StorageKey::Admin)
        .expect("Admin not set")
}

    
pub fn set_is_contract_paused(e: &Env, is_paused: &bool) {
    e.storage()
        .instance()
        .set(&StorageKey::IsContractPaused, is_paused);
}

pub fn get_is_contract_paused(e: &Env) -> bool {
    e.storage()
        .instance()
        .get::<_, bool>(&StorageKey::IsContractPaused)
        .unwrap_or(false)
}

pub fn set_usd_address(e: &Env, usd_address: &Address) {
    e.storage()
        .instance()
        .set(&StorageKey::UsdAddress, usd_address);
}

pub fn get_usd_address(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<_, Address>(&StorageKey::UsdAddress)
        .expect("UsdAddress not set")
}

pub fn set_batch(e: &Env, batch_id: &u128, batch: &Batch) {
    e.storage()
        .persistent()
        .set(&StorageKey::Batches(batch_id.clone()), batch);
}

pub fn get_batch(e: &Env, batch_id: &u128) -> Option<Batch> {
    e.storage()
        .persistent()
        .get::<_, Batch>(&StorageKey::Batches(batch_id.clone()))
}

pub fn set_paid_address(e: &Env, batch_id: &u128, address: &Address) {
    e.storage()
        .persistent()
        .set(&StorageKey::PaidAddresses(batch_id.clone(), address.clone()), &true);
}

pub fn get_paid_address(e: &Env, batch_id: &u128, address: &Address) -> bool {
    e.storage()
        .persistent()
        .get::<_, bool>(&StorageKey::PaidAddresses(batch_id.clone(), address.clone()))
        .unwrap_or(false)
}

pub fn get_paid_addresses(e: &Env, batch_id: &u128, addresses: &Vec<Address>) -> Vec<bool> {
    let mut paid_addresses = vec![&e];
    for address in addresses {
        paid_addresses.push_back(get_paid_address(e, &batch_id, &address));
    }
    paid_addresses
}

// FeeReceiver helpers
pub fn set_fee_receiver(e: &Env, fee: &Address) {
    e.storage().instance().set(&StorageKey::FeeReceiver, fee);
}

pub fn get_fee_receiver(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<_, Address>(&StorageKey::FeeReceiver)
        .expect("FeeReceiver not set")
}

// FeePercent helpers
pub fn set_fee_percent(e: &Env, pct: &u128) {
    e.storage().instance().set(&StorageKey::FeePercent, pct);
}

pub fn get_fee_percent(e: &Env) -> u128 {
    e.storage()
        .instance()
        .get::<_, u128>(&StorageKey::FeePercent)
        .unwrap_or(0)
}
