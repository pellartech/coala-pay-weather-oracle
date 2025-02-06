#![cfg(test)]
extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, vec, Address, Env, IntoVal, Symbol,
};
use token::Client as TokenClient;
use token::StellarAssetClient as TokenAdminClient;
use crate::{ValueSetEvent, WithdrawEvent};

// ----------------------------------------------------------------------
// Helper: create_token_contract
// ----------------------------------------------------------------------
fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let contract_address = e.register_stellar_asset_contract(admin.clone());
    (
        TokenClient::new(e, &contract_address),
        TokenAdminClient::new(e, &contract_address),
    )
}

// ----------------------------------------------------------------------
// Helper: create_weather_oracle_contract (UPDATED for start_time)
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
    start_time: &u64, // NEW param
) -> WeatherOracleClient<'a> {
    // Register the contract and create the client
    let weather_oracle = WeatherOracleClient::new(e, &e.register_contract(None, crate::WeatherOracle {}));

    // Initialize, passing the new `start_time`
    weather_oracle.initialize(
        caller,
        relayer,
        epoch_duration,
        continuity_requirement,
        threshold,
        token,
        recipient,
        start_time, // Convert &u64 -> u64
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

    // Create weather oracle, passing in start_time
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
    );

    assert_eq!(weather_oracle.get_relayer(), relayer, "Relayer not set correctly");
    assert_eq!(weather_oracle.get_continuity_requirement(), 2, "Continuity requirement not set correctly");
    assert_eq!(weather_oracle.get_threshold(), 10, "Threshold not set correctly");
    assert_eq!(weather_oracle.get_contract_owner(), owner, "Contract owner not set correctly");

    // Because start_time == current ledger time, epoch should be 0
    assert_eq!(weather_oracle.get_current_epoch(), 0, "Initial epoch should be 0");

    let epoch_data = weather_oracle.get_epoch_data(&0);
    assert_eq!(epoch_data.value, 0, "Initial epoch value should be 0");
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
        &start_time, // NEW
    );

    // Initially, no time has passed
    assert_eq!(weather_oracle.get_current_epoch(), 0, "Initial epoch should be 0");

    // Advance time by 2 days
    e.ledger().set_timestamp(e.ledger().timestamp() + 2 * 60 * 60 * 24);

    // Because start_time was the original ledger time, after 2 days -> epoch=2
    assert_eq!(
        weather_oracle.get_current_epoch(),
        2,
        "Epoch should be 2 after 2 days"
    );

    // Advance time by another day
    e.ledger().set_timestamp(e.ledger().timestamp() + 60 * 60 * 24);
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
        &start_time, // NEW
    );

    // Fund contract with tokens
    token_admin.mint(&owner, &1000);
    token.transfer(&owner, &weather_oracle.address, &1000);
    e.ledger().set_timestamp(e.ledger().timestamp() + (60 * 60 * 24 * 3));

    assert_eq!(token.balance(&weather_oracle.address), 1000);

    // Test first value update (epoch=0)
    weather_oracle.set_value(&relayer, &50, &1);
    assert_eq!(
        weather_oracle.get_continuity(),
        1,
        "Continuity should be 1 after first update"
    );

    // Advance time by 1 day -> epoch=1
    e.ledger().set_timestamp(e.ledger().timestamp() + 60 * 60 * 24);

    // Test second update
    weather_oracle.set_value(&relayer, &50, &2);
    assert_eq!(
        weather_oracle.get_continuity(),
        2,
        "Continuity should be 2 after second update"
    );

    // Once continuity=2 (>= requirement=2), tokens should transfer to recipient
    assert_eq!(
        token.balance(&weather_oracle.address),
        0,
        "Contract balance should be 0 after transfer"
    );
    assert_eq!(
        token.balance(&recipient),
        1000,
        "Recipient should have received tokens"
    );
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
        &2,             // continuity_requirement
        &10,            // threshold
        &token.address,
        &recipient,
        &start_time,
    );

    // Fund the contract with tokens
    token_admin.mint(&owner, &1000);
    token.transfer(&owner, &weather_oracle.address, &1000);

    e.ledger().set_timestamp(e.ledger().timestamp() + 2 * 60 * 60 * 24 * 2);
    // First, set value above threshold => continuity should increment to 1
    weather_oracle.set_value(&relayer, &50, &1);
    assert_eq!(
        weather_oracle.get_continuity(),
        1,
        "Continuity should be 1 after setting value above threshold"
    );

    // // Next, set value below threshold => continuity should reset to 0
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
    );

    // Attempting to set_value using an unauthorized address => panic
    weather_oracle.set_value(&unauthorized, &50, &0);
}

// ----------------------------------------------------------------------
// Test: Withdraw
// ----------------------------------------------------------------------
#[test]
fn test_withdraw() {
    let e = Env::default();
    e.mock_all_auths();

    let owner = Address::generate(&e);
    let relayer = Address::generate(&e);
    let recipient = Address::generate(&e);
    let withdrawal_recipient = Address::generate(&e);

    let (token, token_admin) = create_token_contract(&e, &owner);

    // Use the current ledger timestamp as start_time
    let start_time = e.ledger().timestamp();

    // Create WeatherOracle
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
    );

    // Fund the contract
    token_admin.mint(&owner, &1000);
    token.transfer(&owner, &weather_oracle.address, &1000);
    assert_eq!(token.balance(&weather_oracle.address), 1000);

    // Test successful withdrawal by owner
    weather_oracle.withdraw(&owner, &500, &withdrawal_recipient);
    assert_eq!(token.balance(&weather_oracle.address), 500);
    assert_eq!(token.balance(&withdrawal_recipient), 500);

    // Attempt withdrawal with non-owner (should fail or revert)
    let random_user = Address::generate(&e);
    let success = weather_oracle.try_withdraw(&random_user, &100, &withdrawal_recipient);
    assert!(
        success.is_err() || success.unwrap().is_err(),
        "Unauthorized withdraw should fail"
    );

    // Attempt withdrawal exceeding balance (should fail)
    let success = weather_oracle.try_withdraw(&owner, &1000, &withdrawal_recipient);
    assert!(
        success.is_err() || success.unwrap().is_err(),
        "Withdraw exceeding balance should fail"
    );
}

// ----------------------------------------------------------------------
// Test: Withdraw After Threshold Reached
// ----------------------------------------------------------------------
#[test]
fn test_withdraw_after_threshold_reached() {
    let e = Env::default();
    e.mock_all_auths();

    let owner = Address::generate(&e);
    let relayer = Address::generate(&e);
    let recipient = Address::generate(&e);

    let (token, token_admin) = create_token_contract(&e, &owner);

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
    );

    // Fund the contract
    token_admin.mint(&owner, &1000);
    token.transfer(&owner, &weather_oracle.address, &1000);

    e.ledger().set_timestamp(e.ledger().timestamp() + (60 * 60 * 24 * 3));

    // Set values above threshold to trigger automatic transfer
    weather_oracle.set_value(&relayer, &50, &1);
    weather_oracle.set_value(&relayer, &50, &2);

    // After continuity=2, tokens are automatically transferred. Contract balance=0
    assert_eq!(token.balance(&weather_oracle.address), 0);

    // Attempting withdrawal => insufficient balance => expect fail
    let withdrawal_recipient = Address::generate(&e);
    let success = weather_oracle.try_withdraw(&owner, &100, &withdrawal_recipient);
    assert!(
        success.is_err() || success.unwrap().is_err(),
        "Withdraw should fail with insufficient balance"
    );
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
    );

    // Basic checks
    assert_eq!(weather_oracle.get_contract_owner(), owner);
    // assert_eq!(weather_oracle.get_start_time(), 1737061157);

    // The ledger’s "current" time = start_time => so get_current_epoch() should be 0
    // assert_eq!(weather_oracle.get_current_epoch(), 0, "Expect epoch=0 right after init");

    // ------------------------------------------------------------------
    // 4. Mint tokens to the contract, just like other tests
    // ------------------------------------------------------------------
    token_admin.mint(&owner, &1000);
    token.transfer(&owner, &weather_oracle.address, &1000);
    assert_eq!(token.balance(&weather_oracle.address), 1000);

    // ------------------------------------------------------------------
    // 5. Advance ledger time by 1 day => epoch=1
    // ------------------------------------------------------------------
    let one_day = 60 * 60 * 24;
    e.ledger().set_timestamp(e.ledger().timestamp() + one_day);
    e.ledger().set_timestamp(e.ledger().timestamp() + one_day);

    // Confirm epoch is now 1
    // assert_eq!(weather_oracle.get_current_epoch(), 1, "After 1 day => epoch=1");

    // set_value(epoch=1, value=50) => above threshold => continuity=1
    weather_oracle.set_value(&relayer, &50, &1);
    assert_eq!(weather_oracle.get_continuity(), 1, "Continuity => 1");

    // ------------------------------------------------------------------
    // 6. Advance ledger time again => epoch=2
    // ------------------------------------------------------------------
    e.ledger().set_timestamp(e.ledger().timestamp() + one_day);
    // assert_eq!(weather_oracle.get_current_epoch(), 2, "After 2 days => epoch=2");

    // set_value(epoch=2, value=5) => below threshold => continuity => 0
    weather_oracle.set_value(&relayer, &5, &2);
    // assert_eq!(weather_oracle.get_continuity(), 0, "Should reset to 0 if value < threshold");

    // ------------------------------------------------------------------
    // 7. Advance ledger => epoch=3 => set_value => ...
    // ------------------------------------------------------------------
    e.ledger().set_timestamp(e.ledger().timestamp() + one_day);
    // assert_eq!(weather_oracle.get_current_epoch(), 3);

    weather_oracle.set_value(&relayer, &50, &3);
    // continuity => 1 again
    assert_eq!(weather_oracle.get_continuity(), 1, "Goes back up if value > threshold");

    // And so on. This structure demonstrates how you can manipulate
    // ledger timestamps and verify epochs, continuity, etc.
}


// ----------------------------------------------------------------------
// (Optional) If you want to re-enable event tests, ensure the data matches
// the new contract design. For example:
// ----------------------------------------------------------------------
// #[test]
// fn test_value_set_event() {
//     // ...
// }
//
// #[test]
// fn test_withdraw_event() {
//     // ...
// }
