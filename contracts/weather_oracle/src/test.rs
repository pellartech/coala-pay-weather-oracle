#![cfg(test)]
extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};
use token::Client as TokenClient;
use token::StellarAssetClient as TokenAdminClient;

const INIT_FEE_PERCENT: u128 = 5;
const INIT_FEE_RECEIVER_STR: &str = "GDHUXXPDBB7VAOAZKGSXJ7SOLTA4TS5R5L74IQX3AIESSLFJL5TGMWIH";

// ----------------------------------------------------------------------
// Helper: create_token_contract
// ----------------------------------------------------------------------
fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let contract_address = e.register_stellar_asset_contract_v2(admin.clone()).address();
    (
        TokenClient::new(e, &contract_address),
        TokenAdminClient::new(e, &contract_address),
    )
}

// ----------------------------------------------------------------------
// Helper: create_weather_oracle_contract
// ----------------------------------------------------------------------
fn create_weather_oracle_contract<'a>(
    e: &Env,
    caller: &Address,
    relayer: &Address,
    epoch_duration: &u32,
    continuity_requirement: &u32,
    threshold: &u32,
    token: &Address,
    recipient: &Address,
    start_time: &u64,
    funder: &Address,
) -> WeatherOracleClient<'a> {
    let weather_oracle =
        WeatherOracleClient::new(e, &e.register_contract(None, crate::WeatherOracle {}));
    weather_oracle.initialize(
        caller,
        relayer,
        epoch_duration,
        continuity_requirement,
        threshold,
        token,
        recipient,
        start_time,
        &100u128,
        funder,
    );
    weather_oracle
}

// ----------------------------------------------------------------------
// Test: Initialization
// ----------------------------------------------------------------------
#[test]
fn test_initialization() {
    let e = Env::default();
    e.mock_all_auths();

    let owner = Address::generate(&e);
    let relayer = Address::generate(&e);
    let recipient = Address::generate(&e);

    // Create token for testing
    let (token, _) = create_token_contract(&e, &owner);

    // Use the current ledger timestamp as start_time
    let start_time = e.ledger().timestamp();

    // Create weather oracle, passing in start_time and funder
    let weather_oracle = create_weather_oracle_contract(
        &e,
        &owner,
        &relayer,
        &(60 * 60 * 24),
        &2,
        &10,
        &token.address,
        &recipient,
        &start_time,
        &owner, // funder is the owner
    );

    assert_eq!(
        weather_oracle.get_relayer(),
        relayer,
        "Relayer not set correctly"
    );
    assert_eq!(
        weather_oracle.get_continuity_requirement(),
        2,
        "Continuity requirement not set correctly"
    );
    assert_eq!(
        weather_oracle.get_threshold(),
        10,
        "Threshold not set correctly"
    );
    assert_eq!(
        weather_oracle.get_contract_owner(),
        owner,
        "Contract owner not set correctly"
    );

    // epoch should be 0
    assert_eq!(
        weather_oracle.get_current_epoch(),
        0,
        "Initial epoch should be 0"
    );

    let epoch_data = weather_oracle.get_epoch_data(&0);
    assert_eq!(epoch_data.value, 0, "Initial epoch value should be 0");

    let default_fee_receiver = Address::from_str(&e, INIT_FEE_RECEIVER_STR);
    assert_eq!(
        weather_oracle.get_fee_receiver(),
        default_fee_receiver,
        "Default fee receiver not set"
    );
    assert_eq!(
        weather_oracle.get_fee_percent(),
        INIT_FEE_PERCENT,
        "Default fee percent not set"
    );
}

// ----------------------------------------------------------------------
// Test: Epoch Progression
// ----------------------------------------------------------------------
#[test]
fn test_epoch_progression() {
    let e = Env::default();
    e.mock_all_auths();

    let owner = Address::generate(&e);
    let relayer = Address::generate(&e);
    let recipient = Address::generate(&e);

    // Create token
    let (token, _) = create_token_contract(&e, &owner);

    // Use the current ledger timestamp as start_time
    let start_time = e.ledger().timestamp();

    // Initialize contract
    let weather_oracle = create_weather_oracle_contract(
        &e,
        &owner,
        &relayer,
        &(60 * 60 * 24), // 1 day
        &2,
        &10,
        &token.address,
        &recipient,
        &start_time,
        &owner, // funder is the owner
    );

    // Initially, no time has passed
    assert_eq!(
        weather_oracle.get_current_epoch(),
        0,
        "Initial epoch should be 0"
    );

    // Advance time by 2 days
    e.ledger()
        .set_timestamp(e.ledger().timestamp() + 2 * 60 * 60 * 24);

    // Because start_time was the original ledger time, after 2 days -> epoch=2
    assert_eq!(
        weather_oracle.get_current_epoch(),
        2,
        "Epoch should be 2 after 2 days"
    );

    // Advance time by another day
    e.ledger()
        .set_timestamp(e.ledger().timestamp() + 60 * 60 * 24);
    assert_eq!(
        weather_oracle.get_current_epoch(),
        3,
        "Epoch should be 3 after 3 days"
    );
}

// ----------------------------------------------------------------------
// Test: Value Updates and Continuity
// ----------------------------------------------------------------------
#[test]
fn test_value_updates_and_continuity() {
    let e = Env::default();
    e.mock_all_auths();

    let owner = Address::generate(&e);
    let relayer = Address::generate(&e);
    let recipient = Address::generate(&e);

    // Create token
    let (token, token_admin) = create_token_contract(&e, &owner);

    // Use the current ledger timestamp as start_time
    let start_time = e.ledger().timestamp();

    // Initialize WeatherOracle
    let weather_oracle = create_weather_oracle_contract(
        &e,
        &owner,
        &relayer,
        &(60 * 60 * 24),
        &2,
        &10,
        &token.address,
        &recipient,
        &start_time,
        &owner, // funder is the owner
    );

    // Fund the owner with tokens for payout (no need to transfer to contract)
    token_admin.mint(&owner, &105); // 100 + 5 for fees

    // IMPORTANT: The funder (owner) must authorize the contract to transfer tokens
    // This is required for the transfer-from approach to work
    token.approve(&owner, &weather_oracle.address, &105, &0);

    let new_receiver = Address::generate(&e);
    weather_oracle.set_fee_receiver(&owner, &new_receiver);

    // move to epoch 1
    e.ledger().set_timestamp(start_time + 2 * 60 * 60 * 24);
    weather_oracle.set_value(&relayer, &50, &1);
    assert_eq!(weather_oracle.get_continuity(), 1);

    // move to epoch 2 -> triggers payout (transfer from owner to recipients)
    e.ledger()
        .set_timestamp(e.ledger().timestamp() + 60 * 60 * 24);
    weather_oracle.set_value(&relayer, &50, &2);
    assert!(weather_oracle.get_funds_released());

    // Check that tokens were transferred from owner to recipients
    assert_eq!(
        token.balance(&new_receiver),
        5,
        "Fee receiver should get 5%"
    );
    assert_eq!(token.balance(&recipient), 100, "Recipient should get full funding amount");
    // Owner should have 0 tokens left (105 - 100 - 5)
    assert_eq!(token.balance(&owner), 0, "Owner should have 0 tokens left");
}

#[test]
fn test_value_below_threshold_resets_continuity() {
    let e = Env::default();
    e.mock_all_auths();

    let owner = Address::generate(&e);
    let relayer = Address::generate(&e);
    let recipient = Address::generate(&e);

    // Create token
    let (token, token_admin) = create_token_contract(&e, &owner);

    // Use the current ledger timestamp as start_time
    let start_time = e.ledger().timestamp();

    // We'll set continuity_requirement = 2 and threshold = 10
    let weather_oracle = create_weather_oracle_contract(
        &e,
        &owner,
        &relayer,
        &(60 * 60 * 24), // 1-day epoch
        &2,              // continuity_requirement
        &10,             // threshold
        &token.address,
        &recipient,
        &start_time,
        &owner, // funder is the owner
    );

    // Fund the owner with tokens (no need to transfer to contract)
    token_admin.mint(&owner, &1000);

    e.ledger()
        .set_timestamp(e.ledger().timestamp() + 2 * 60 * 60 * 24 * 2);
    // First, set value above threshold => continuity should increment to 1
    weather_oracle.set_value(&relayer, &50, &1);
    assert_eq!(
        weather_oracle.get_continuity(),
        1,
        "Continuity should be 1 after setting value above threshold"
    );

    // Next, set value below threshold => continuity should reset to 0
    weather_oracle.set_value(&relayer, &5, &2); // 5 < threshold=10
    assert_eq!(
        weather_oracle.get_continuity(),
        0,
        "Continuity should reset to 0 when value < threshold"
    );
}

// ----------------------------------------------------------------------
// Test: Unauthorized Value Update (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Caller is not the relayer")]
fn test_unauthorized_value_update() {
    let e = Env::default();
    e.mock_all_auths();

    let owner = Address::generate(&e);
    let relayer = Address::generate(&e);
    let recipient = Address::generate(&e);
    let unauthorized = Address::generate(&e);

    let (token, _) = create_token_contract(&e, &owner);

    // Use the current ledger timestamp as start_time
    let start_time = e.ledger().timestamp();

    // Initialize
    let weather_oracle = create_weather_oracle_contract(
        &e,
        &owner,
        &relayer,
        &(60 * 60 * 24),
        &2,
        &10,
        &token.address,
        &recipient,
        &start_time, // NEW
        &owner, // funder is the owner
    );

    // Attempting to set_value using an unauthorized address => panic
    weather_oracle.set_value(&unauthorized, &50, &0);
}

#[test]
fn test_timestamp_behavior() {
    let e = Env::default();
    e.mock_all_auths();

    // ------------------------------------------------------------------
    // 1. Generate Addresses (similar to other tests)
    // ------------------------------------------------------------------
    let owner = Address::generate(&e);
    let relayer = Address::generate(&e);
    let recipient = Address::generate(&e);

    // Create a token for the contract to manage or transfer
    let (token, token_admin) = create_token_contract(&e, &owner);

    // ------------------------------------------------------------------
    // 2. Set the ledger to a specific “start_time” (1737061157)
    // ------------------------------------------------------------------
    e.ledger().set_timestamp(1737061157);

    // We'll use the same epoch_duration, continuity_requirement, threshold
    let epoch_duration = 60 * 60 * 24; // 1 day
    let continuity_requirement = 2;
    let threshold = 10u32;

    // ------------------------------------------------------------------
    // 3. Initialize Contract
    // ------------------------------------------------------------------
    let start_time = 1736899200u64; // which is now 1737061157
    let weather_oracle = create_weather_oracle_contract(
        &e,
        &owner,
        &relayer,
        &epoch_duration,
        &continuity_requirement,
        &threshold,
        &token.address,
        &recipient,
        &start_time,
        &owner, // funder is the owner
    );

    // Basic checks
    assert_eq!(weather_oracle.get_contract_owner(), owner);

    // Mint tokens to the owner (no need to transfer to contract)
    token_admin.mint(&owner, &1000);

    // Advance ledger time by 2 days => epoch=2
    let one_day = 60 * 60 * 24;
    e.ledger()
        .set_timestamp(e.ledger().timestamp() + (one_day * 2));

    // Now do a set_value => ...
    weather_oracle.set_value(&relayer, &50, &1);
    assert_eq!(weather_oracle.get_continuity(), 1, "Continuity => 1");
}

#[test]
fn test_fee_setters() {
    let e = Env::default();
    e.mock_all_auths();

    let owner = Address::generate(&e);
    let relayer = Address::generate(&e);
    let recipient = Address::generate(&e);

    let (token, _) = create_token_contract(&e, &owner);
    let start_time = e.ledger().timestamp();

    let weather_oracle = create_weather_oracle_contract(
        &e,
        &owner,
        &relayer,
        &(60 * 60 * 24),
        &2,
        &10,
        &token.address,
        &recipient,
        &start_time,
        &owner, // funder is the owner
    );

    // change fee receiver
    let new_receiver = Address::generate(&e);
    weather_oracle.set_fee_receiver(&owner, &new_receiver);
    assert_eq!(weather_oracle.get_fee_receiver(), new_receiver);

    // change fee percent
    weather_oracle.set_fee_percent(&owner, &42u128);
    assert_eq!(weather_oracle.get_fee_percent(), 42u128);
}
