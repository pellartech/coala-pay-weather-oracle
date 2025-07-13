use soroban_sdk::{contracttype, Address};

// Define Events
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct ContractPausedEvent {
    pub is_paused: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct BatchCreatedEvent {
    pub batch_id: u128,
    pub token_address: Address,
    pub amount_per_beneficiary: u128,
    pub reduced_amount_per_beneficiary: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct BatchPaidEvent {
    pub batch_id: u128,
    pub use_reduced: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct FundRecoveredEvent {
    pub from: Address,
    pub recipient: Address,
    pub amount: u128,
} 