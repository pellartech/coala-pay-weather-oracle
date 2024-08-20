#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env};

#[derive(Clone, Debug)]
#[contracttype]
pub struct EpochData {
    pub value: u32,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Initialized,
    ContractOwner,
    EpochData(u32),
    Relayer,
    EpochDuration,
    ContinuityRequirement,
    Threshold,
    Continuity,
    LatestUpdate,
    Token,
    Recipient
}

fn get_contract_owner(e: &Env) -> Address {
    e.storage().instance().get::<_, Address>(&DataKey::ContractOwner)
        .expect("Contract not initialized")
}

fn get_relayer(e: &Env) -> Address {
    e.storage().instance().get::<_, Address>(&DataKey::Relayer)
        .expect("Contract not initialized")
}

fn get_epoch_data(e: &Env, epoch: u32) -> EpochData {
    e.storage().instance().get::<_, EpochData>(&DataKey::EpochData(epoch))
        .expect("Epoch data not found")
}

fn get_last_update_time(e: &Env) -> u64 {
    e.storage().instance().get::<_, u64>(&DataKey::LatestUpdate)
        .expect("Contract not initialized")
}

fn get_continuity_requirement(e: &Env) -> u32 {
    e.storage().instance().get::<_, u32>(&DataKey::ContinuityRequirement)
        .expect("Contract not initialized")
}

fn get_threshold(e: &Env) -> u32 {
    e.storage().instance().get::<_, u32>(&DataKey::Threshold)
        .expect("Contract not initialized")
}

fn set_continuity_requirement(e: &Env, continuity_requirement: u32) {
    e.storage().instance().set(&DataKey::ContinuityRequirement, &continuity_requirement);
}

fn set_threshold(e: &Env, threshold: u32) {
    e.storage().instance().set(&DataKey::Threshold, &threshold);
}

fn get_epoch_duration(e: &Env) -> u32 {
    e.storage().instance().get::<_, u32>(&DataKey::EpochDuration)
        .expect("Contract not initialized")
}

fn get_current_epoch(e: &Env) -> u32 {
    let current_timestamp = e.ledger().timestamp();
    let current_epoch = current_timestamp / u64::from(get_epoch_duration(e));
    current_epoch.try_into().unwrap()
}

fn get_continuity(e: &Env) -> u32 {
    e.storage().instance().get::<_, u32>(&DataKey::Continuity)
        .expect("Contract not initialized")
}

fn set_continuity(e: &Env, continuity: u32) {
    e.storage().instance().set(&DataKey::Continuity, &continuity);
}

#[contract]
pub struct WeatherOracle;

#[contractimpl]
impl WeatherOracle {
    pub fn initialize(
        e: Env,
        caller: Address,
        relayer: Address,
        epoch_duration: u32,
        continuity_requirement: u32,
        threshold: u32,
        token: Address,
        recipient: Address
    ) {
        assert!(
            !e.storage().instance().has(&DataKey::Initialized),
            "Contract already initialized"
        );

        let initial_continuity: u32 = 0;

        e.storage().instance().set(&DataKey::ContractOwner, &caller);
        e.storage().instance().set(&DataKey::Initialized, &true);
        e.storage().instance().set(&DataKey::Relayer, &relayer);
        e.storage().instance().set(&DataKey::EpochDuration, &epoch_duration);
        e.storage().instance().set(&DataKey::ContinuityRequirement, &continuity_requirement);
        e.storage().instance().set(&DataKey::Threshold, &threshold);
        e.storage().instance().set(&DataKey::Token, &token);
        e.storage().instance().set(&DataKey::Recipient, &recipient);
        e.storage().instance().set(&DataKey::Continuity, &initial_continuity);

        let current_epoch = get_current_epoch(&e);
        let initial_value: u32 = 0;
        let epoch_data = EpochData { value: initial_value };
        e.storage().instance().set(&DataKey::EpochData(current_epoch), &epoch_data);
        e.storage().instance().set(&DataKey::LatestUpdate, &current_epoch);
    }

    pub fn set_value(
        e: Env,
        caller: Address,
        value: u32,
        epoch: u32,
    ) {
        caller.require_auth();
        assert_eq!(
            caller,
            Self::get_relayer(e.clone()),
            "Caller is not the relayer"
        );

        let current_epoch = get_current_epoch(&e);

        assert!(
            epoch < current_epoch,
            "Value can only be updated for previous epochs"
        );

        let latest_update: u32 = e.storage().instance().get::<_, u32>(&DataKey::LatestUpdate)
            .expect("Contract not initialized");

        assert!(
            epoch == latest_update + 1,
            "Epoch must be sequential"
        );

        let epoch_data = EpochData { value };

        e.storage().instance().set(&DataKey::EpochData(epoch), &epoch_data);
        e.storage().instance().set(&DataKey::LatestUpdate, &epoch);

        let threshold = get_threshold(&e);
        let continuity_requirement = get_continuity_requirement(&e);

        if value > threshold {
            let continuity: u32 = get_continuity(&e);
            set_continuity(&e, continuity + 1);

            if continuity + 1 >= continuity_requirement {
                let contract_address = e.current_contract_address();
                let token = e.storage().instance().get::<_, Address>(&DataKey::Token)
                    .expect("Contract not initialized");
                let token_client: token::TokenClient = token::Client::new(&e, &token);
                let balance = token_client.balance(&e.current_contract_address());
                token_client.transfer(&contract_address, &e.storage().instance().get::<_, Address>(&DataKey::Recipient).expect("Contract not initialized"), &balance);
            }
        } else {
            e.storage().instance().set(&DataKey::Continuity, &0);
        }

    }

    pub fn get_value(
        e: Env,
        epoch: u32,
    ) -> u32 {
        let epoch_data = get_epoch_data(&e, epoch);
        epoch_data.value
    }

    pub fn set_continuity_requirement(
        e: Env,
        caller: Address,
        continuity_requirement: u32,
    ) {
        caller.require_auth();
        assert_eq!(
            caller,
            Self::get_contract_owner(e.clone()),
            "Caller is not the contract owner"
        );
        set_continuity_requirement(&e, continuity_requirement);
    }

    pub fn set_threshold(
        e: Env,
        caller: Address,
        threshold: u32,
    ) {
        caller.require_auth();
        assert_eq!(
            caller,
            Self::get_contract_owner(e.clone()),
            "Caller is not the contract owner"
        );
        set_threshold(&e, threshold);
    }

    pub fn get_contract_owner(e: Env) -> Address {
        get_contract_owner(&e)
    }

    pub fn get_relayer(e: Env) -> Address {
        get_relayer(&e)
    }

    pub fn get_continuity_requirement(e: Env) -> u32 {
        get_continuity_requirement(&e)
    }

    pub fn get_threshold(e: Env) -> u32 {
        get_threshold(&e)
    }

    pub fn get_last_update_time(e: Env) -> u64 {
        get_last_update_time(&e)
    }

    pub fn get_epoch_data(e: Env, epoch: u32) -> EpochData {
        get_epoch_data(&e, epoch)
    }

    pub fn get_current_epoch(e: Env) -> u32 {
        get_current_epoch(&e)
    }

    pub fn get_continuity(e: Env) -> u32 {
        get_continuity(&e)
    }
}

mod test;