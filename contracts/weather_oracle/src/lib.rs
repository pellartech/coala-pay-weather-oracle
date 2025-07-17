#![no_std]
pub mod contract {
    pub use soroban_sdk::contract;
}
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Env, Symbol,
};

const COALA: Symbol = symbol_short!("COALA");
const INIT_FEE_PERCENT: u128 = 5;
fn get_init_fee_receiver(env: &Env) -> Address {
    Address::from_str(env, "GD6VVDDNR2KCR3OGD27KQPQ6ITG7OEQR25Z7OWOD5YPDNMRXZBWXUOA7")
}

// Define Events
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct ValueSetEvent {
    pub relayer: Address,
    pub epoch: u32,
    pub value: u32,
    pub continuity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct FunderChangedEvent {
    pub caller: Address,
    pub old_funder: Address,
    pub new_funder: Address,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct EpochData {
    pub value: u32,
}

#[derive(Clone, Debug)]
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
    Recipient,
    StartTime,
    FundingAmount,
    Funder,
    FundsReleased,
    FeeReceiver,
    FeePercent,
}

// Helper Methods
fn get_contract_owner(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<_, Address>(&DataKey::ContractOwner)
        .expect("Contract not initialized")
}

fn get_relayer(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<_, Address>(&DataKey::Relayer)
        .expect("Contract not initialized")
}

fn get_fee_receiver(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<_, Address>(&DataKey::FeeReceiver)
        .expect("Contract not initialized")
}

fn set_fee_receiver(e: &Env, addr: &Address) {
    e.storage().instance().set(&DataKey::FeeReceiver, addr);
}

fn get_fee_percent(e: &Env) -> u128 {
    e.storage()
        .instance()
        .get::<_, u128>(&DataKey::FeePercent)
        .expect("Contract not initialized")
}

fn set_fee_percent(e: &Env, pct: &u128) {
    e.storage().instance().set(&DataKey::FeePercent, pct);
}

fn get_epoch_data(e: &Env, epoch: u32) -> EpochData {
    e.storage()
        .instance()
        .get::<_, EpochData>(&DataKey::EpochData(epoch))
        .expect("Epoch data not found")
}

fn get_last_update_time(e: &Env) -> u32 {
    e.storage()
        .instance()
        .get::<_, u32>(&DataKey::LatestUpdate)
        .expect("Contract not initialized")
}

fn get_continuity_requirement(e: &Env) -> u32 {
    e.storage()
        .instance()
        .get::<_, u32>(&DataKey::ContinuityRequirement)
        .expect("Contract not initialized")
}

fn get_threshold(e: &Env) -> u32 {
    e.storage()
        .instance()
        .get::<_, u32>(&DataKey::Threshold)
        .expect("Contract not initialized")
}

fn set_continuity_requirement(e: &Env, continuity_requirement: u32) {
    e.storage()
        .instance()
        .set(&DataKey::ContinuityRequirement, &continuity_requirement);
}

fn set_threshold(e: &Env, threshold: u32) {
    e.storage()
        .instance()
        .set(&DataKey::Threshold, &threshold);
}

fn get_epoch_duration(e: &Env) -> u32 {
    e.storage()
        .instance()
        .get::<_, u32>(&DataKey::EpochDuration)
        .expect("Contract not initialized")
}

fn get_continuity(e: &Env) -> u32 {
    e.storage()
        .instance()
        .get::<_, u32>(&DataKey::Continuity)
        .expect("Contract not initialized")
}

fn set_continuity(e: &Env, continuity: u32) {
    e.storage()
        .instance()
        .set(&DataKey::Continuity, &continuity);
}

fn get_start_time(e: &Env) -> u64 {
    e.storage()
        .instance()
        .get::<_, u64>(&DataKey::StartTime)
        .expect("Contract not initialized")
}

fn get_funding_amount(e: &Env) -> u128 {
    e.storage()
        .instance()
        .get::<_, u128>(&DataKey::FundingAmount)
        .expect("Contract not initialized")
}

fn get_funder(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<_, Address>(&DataKey::Funder)
        .expect("Contract not initialized")
}

fn set_funder(e: &Env, funder: &Address) {
    e.storage().instance().set(&DataKey::Funder, funder);
}

fn get_current_epoch(e: &Env) -> u32 {
    let current_timestamp = e.ledger().timestamp();
    let epoch_duration = get_epoch_duration(e);
    let start_time = get_start_time(e);

    let elapsed = current_timestamp.saturating_sub(start_time);
    let current_epoch = elapsed / u64::from(epoch_duration);

    current_epoch.try_into().unwrap()
}

fn set_funds_released(e: &Env, released: bool) {
    e.storage().instance().set(&DataKey::FundsReleased, &released);
}

fn get_funds_released(e: &Env) -> bool {
    e.storage()
        .instance()
        .get::<_, bool>(&DataKey::FundsReleased)
        .unwrap_or(false)
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
        recipient: Address,
        start_time: u64,
        funding_amount: u128,
        funder: Address,
    ) {
        assert!(
            !e.storage().instance().has(&DataKey::Initialized),
            "Contract already initialized"
        );

        // Store basic contract configuration
        e.storage().instance().set(&DataKey::Initialized, &true);
        e.storage().instance().set(&DataKey::ContractOwner, &caller);
        e.storage().instance().set(&DataKey::Relayer, &relayer);
        e.storage()
            .instance()
            .set(&DataKey::EpochDuration, &epoch_duration);
        e.storage()
            .instance()
            .set(&DataKey::ContinuityRequirement, &continuity_requirement);
        e.storage().instance().set(&DataKey::Threshold, &threshold);
        e.storage().instance().set(&DataKey::Token, &token);
        e.storage().instance().set(&DataKey::Recipient, &recipient);
        e.storage().instance().set(&DataKey::FeeReceiver, &get_init_fee_receiver(&e));
        e.storage().instance().set(&DataKey::FeePercent, &INIT_FEE_PERCENT);
        e.storage().instance().set(&DataKey::Continuity, &0u32);
        e.storage().instance().set(&DataKey::StartTime, &start_time);
        e.storage().instance().set(&DataKey::FundingAmount, &funding_amount);
        e.storage().instance().set(&DataKey::Funder, &funder);
        e.storage().instance().set(&DataKey::FundsReleased, &false);

        // Initialize epoch data for the current epoch
        let initial_value: u32 = 0;
        let epoch_data = EpochData { value: initial_value };
        e.storage()
            .instance()
            .set(&DataKey::EpochData(initial_value), &epoch_data);
        e.storage()
            .instance()
            .set(&DataKey::LatestUpdate, &initial_value);
    }

    pub fn set_value(e: Env, caller: Address, value: u32, epoch: u32) {
        caller.require_auth();
        assert_eq!(
            caller,
            Self::get_relayer(e.clone()),
            "Caller is not the relayer"
        );

        let current_epoch = get_current_epoch(&e);
        assert!(
            epoch <= current_epoch,
            "Value can only be updated for previous or current epoch"
        );

        let latest_update: u32 = e
            .storage()
            .instance()
            .get::<_, u32>(&DataKey::LatestUpdate)
            .expect("Contract not initialized");

        assert!(epoch == latest_update + 1, "Epoch must be sequential");

        let epoch_data = EpochData { value };
        e.storage()
            .instance()
            .set(&DataKey::EpochData(epoch), &epoch_data);
        e.storage().instance().set(&DataKey::LatestUpdate, &epoch);

        let threshold = get_threshold(&e);
        let continuity_requirement = get_continuity_requirement(&e);

        if value >= threshold {
            let continuity: u32 = get_continuity(&e);
            set_continuity(&e, continuity + 1);

            if continuity + 1 >= continuity_requirement {
                // Transfer directly from funder to recipients
                let funder = get_funder(&e);
                let token = e
                    .storage()
                    .instance()
                    .get::<_, Address>(&DataKey::Token)
                    .expect("Contract not initialized");
                let token_client: token::Client = token::Client::new(&e, &token);

                let fee_receiver = get_fee_receiver(&e);
                let fee_pct = get_fee_percent(&e);
                let funding_amount = get_funding_amount(&e);
                let fee_amount = funding_amount * fee_pct / 100;
                let recipient = e
                    .storage()
                    .instance()
                    .get::<_, Address>(&DataKey::Recipient)
                    .expect("Contract not initialized");

                // Transfer directly from funder to recipients using transfer_from
                let contract_address = e.current_contract_address();
                token_client.transfer_from(&contract_address, &funder, &fee_receiver, &(fee_amount as i128));
                token_client.transfer_from(&contract_address, &funder, &recipient, &(funding_amount as i128));

                set_funds_released(&e, true);
            }
        } else {
            set_continuity(&e, 0);
        }

        // Emit Value Set Event
        let value_event = ValueSetEvent {
            relayer: caller.clone(),
            epoch,
            value,
            continuity: get_continuity(&e),
        };
        e.events()
            .publish((COALA, symbol_short!("value_set")), value_event);
    }

    // -----------------
    // Getter methods
    // -----------------
    pub fn get_value(e: Env, epoch: u32) -> u32 {
        let epoch_data = get_epoch_data(&e, epoch);
        epoch_data.value
    }

    pub fn get_contract_owner(e: Env) -> Address {
        get_contract_owner(&e)
    }

    pub fn get_relayer(e: Env) -> Address {
        get_relayer(&e)
    }

    pub fn get_fee_receiver(e: Env) -> Address {
        get_fee_receiver(&e)
    }

    pub fn get_fee_percent(e: Env) -> u128 {
        get_fee_percent(&e)
    }

    pub fn get_continuity_requirement(e: Env) -> u32 {
        get_continuity_requirement(&e)
    }

    pub fn get_threshold(e: Env) -> u32 {
        get_threshold(&e)
    }

    pub fn get_last_update_time(e: Env) -> u32 {
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

    pub fn get_start_time(e: Env) -> u64 {
        get_start_time(&e)
    }

    pub fn get_funding_amount(e: Env) -> u128 {
        get_funding_amount(&e)
    }

    pub fn get_funder(e: Env) -> Address {
        get_funder(&e)
    }

    pub fn get_funds_released(e: Env) -> bool {
        get_funds_released(&e)
    }

    // -----------------
    // Setter methods
    // -----------------
    pub fn set_continuity_requirement(e: Env, caller: Address, continuity_requirement: u32) {
        caller.require_auth();
        assert_eq!(
            caller,
            Self::get_contract_owner(e.clone()),
            "Caller is not the contract owner"
        );
        set_continuity_requirement(&e, continuity_requirement);
    }

    pub fn set_threshold(e: Env, caller: Address, threshold: u32) {
        caller.require_auth();
        assert_eq!(
            caller,
            Self::get_contract_owner(e.clone()),
            "Caller is not the contract owner"
        );
        set_threshold(&e, threshold);
    }

    pub fn set_fee_receiver(e: Env, caller: Address, new_fee: Address) {
        caller.require_auth();
        assert_eq!(caller, get_contract_owner(&e), "Caller is not the contract owner");
        set_fee_receiver(&e, &new_fee);
    }

    pub fn set_fee_percent(e: Env, caller: Address, pct: u128) {
        caller.require_auth();
        assert_eq!(caller, get_contract_owner(&e), "Caller is not the contract owner");
        set_fee_percent(&e, &pct);
    }

    pub fn set_funder(e: Env, caller: Address, new_funder: Address) {
        caller.require_auth();
        assert_eq!(caller, get_contract_owner(&e), "Caller is not the contract owner");
        
        let old_funder = get_funder(&e);
        set_funder(&e, &new_funder);
        
        // Emit Funder Changed Event
        let funder_changed_event = FunderChangedEvent {
            caller: caller.clone(),
            old_funder,
            new_funder,
        };
        e.events()
            .publish((COALA, symbol_short!("fund_chg")), funder_changed_event);
    }
}
mod test;
