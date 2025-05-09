#![cfg(test)]
extern crate std;

use super::*;
use crate::{ValueSetEvent, WithdrawEvent};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, vec, Address, Env, IntoVal, Symbol,
};
use token::Client as TokenClient;
use token::StellarAssetClient as TokenAdminClient;

const INIT_FEE_PERCENT: u128 = 5;
const INIT_FEE_RECEIVER_STR: &str = "GD6VVDDNR2KCR3OGD27KQPQ6ITG7OEQR25Z7OWOD5YPDNMRXZBWXUOA7";

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
        &start_time,
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
        &start_time, // NEW
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
    );

    // Fund contract with 1,000; we'll trigger a payout of 1,000 + fee
    let funder = Address::generate(&e);
    token_admin.mint(&funder, &1000);

    weather_oracle.fund(&funder);


    let new_receiver = Address::generate(&e);
    weather_oracle.set_fee_receiver(&owner, &new_receiver);


    // move to epoch 1
    e.ledger().set_timestamp(start_time + 2 * 60 * 60 * 24);
    weather_oracle.set_value(&relayer, &50, &1);
    assert_eq!(weather_oracle.get_continuity(), 1);

    // move to epoch 2 -> triggers payout
    e.ledger()
        .set_timestamp(e.ledger().timestamp() + 60 * 60 * 24);
    weather_oracle.set_value(&relayer, &50, &2);
    assert!(weather_oracle.get_funds_released());

    assert_eq!(
        token.balance(&new_receiver),
        5,
        "Fee receiver should get 5%"
    );
    assert_eq!(token.balance(&recipient), 100, "Recipient should get 95%");
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
    );

    // Fund the contract with tokens
    token_admin.mint(&owner, &1000);
    token.transfer(&owner, &weather_oracle.address, &1000);

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

    let new_receiver = Address::generate(&e);
    weather_oracle.set_fee_receiver(&owner, &new_receiver);

    // Fund the contract
    token_admin.mint(&owner, &1000);
    token.transfer(&owner, &weather_oracle.address, &1000);

    e.ledger()
        .set_timestamp(e.ledger().timestamp() + (60 * 60 * 24 * 3));

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

    // Mint tokens to the contract
    token_admin.mint(&owner, &1000);
    token.transfer(&owner, &weather_oracle.address, &1000);
    assert_eq!(token.balance(&weather_oracle.address), 1000);

    // Advance ledger time by 2 days => epoch=2
    let one_day = 60 * 60 * 24;
    e.ledger()
        .set_timestamp(e.ledger().timestamp() + (one_day * 2));

    // Now do a set_value => ...
    weather_oracle.set_value(&relayer, &50, &1);
    assert_eq!(weather_oracle.get_continuity(), 1, "Continuity => 1");
}

// ----------------------------------------------------------------------
// Test: Fund Success + Check `funded_by`
// ----------------------------------------------------------------------
#[test]
fn test_fund_success() {
    let e = Env::default();
    e.mock_all_auths();

    // 1. Setup addresses
    let owner = Address::generate(&e);
    let relayer = Address::generate(&e);
    let recipient = Address::generate(&e);
    let funder = Address::generate(&e); // The actual funder

    // 2. Create token contract
    let (token, token_admin) = create_token_contract(&e, &owner);

    // 3. Configure ledger and define contract parameters
    let start_time = e.ledger().timestamp();
    let epoch_duration = 60 * 60 * 24;   // 1 day
    let continuity_requirement = 2;
    let threshold = 10u32;
    let funding_amount = 100u128;        // <-- This is the one-time funding amount

    // 4. Register & initialize the WeatherOracle contract with "funding_amount=100"
    let weather_oracle = WeatherOracleClient::new(&e, &e.register_contract(None, WeatherOracle {}));
    weather_oracle.initialize(
        &owner,
        &relayer,
        &epoch_duration,
        &continuity_requirement,
        &threshold,
        &token.address,
        &recipient,
        &start_time,
        &funding_amount,
    );

    // Mint 105 = 100 + 5% fee
    let needed = 100 + (100 * INIT_FEE_PERCENT / 100);
    token_admin.mint(&owner, &(needed as i128));
    token.transfer(&owner, &funder, &(needed as i128));

    weather_oracle.fund(&funder);

    assert!(weather_oracle.is_funded());
    assert_eq!(weather_oracle.get_funded_balance(), needed);
    assert_eq!(token.balance(&weather_oracle.address), needed as i128);
    assert_eq!(token.balance(&funder), 0);
}

// ----------------------------------------------------------------------
// Test: Fund Twice Should Fail
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Contract has already been funded")]
fn test_fund_twice_should_fail() {
    let e = Env::default();
    e.mock_all_auths();

    let owner = Address::generate(&e);
    let relayer = Address::generate(&e);
    let recipient = Address::generate(&e);
    let funder = Address::generate(&e);

    let (token, token_admin) = create_token_contract(&e, &owner);
    let start_time = e.ledger().timestamp();
    let funding_amount = 50u128;

    let weather_oracle = WeatherOracleClient::new(&e, &e.register_contract(None, WeatherOracle {}));
    weather_oracle.initialize(
        &owner,
        &relayer,
        &(60 * 60 * 24),
        &2,
        &10,
        &token.address,
        &recipient,
        &start_time,
        &funding_amount,
    );

    let new_receiver = Address::generate(&e);
    weather_oracle.set_fee_receiver(&owner, &new_receiver);

    // mint enough (including fee)
    let fee = funding_amount * INIT_FEE_PERCENT / 100;
    let needed = (funding_amount + fee) as i128;
    token_admin.mint(&owner, &needed);
    token.transfer(&owner, &funder, &needed);

    weather_oracle.fund(&funder);
    weather_oracle.fund(&funder);
}

// ----------------------------------------------------------------------
// Test: Fund Insufficient Balance
// ----------------------------------------------------------------------
#[test]
fn test_fund_insufficient_balance() {
    let e = Env::default();
    e.mock_all_auths();

    let owner = Address::generate(&e);
    let relayer = Address::generate(&e);
    let recipient = Address::generate(&e);
    let funder = Address::generate(&e);

    let (token, _) = create_token_contract(&e, &owner);

    let start_time = e.ledger().timestamp();
    let funding_amount = 100u128;

    // Initialize contract requiring 100 tokens for funding
    let weather_oracle = WeatherOracleClient::new(&e, &e.register_contract(None, WeatherOracle {}));
    weather_oracle.initialize(
        &owner,
        &relayer,
        &(60 * 60 * 24),
        &2,
        &10,
        &token.address,
        &recipient,
        &start_time,
        &funding_amount,
    );

    // `get_funded_by` should be None at this point
    assert_eq!(
        weather_oracle.get_funded_by(),
        None,
        "Should not have a funder yet"
    );

    // We intentionally do NOT mint any tokens to `funder` => balance=0
    // Attempt to fund, which should fail due to insufficient token balance
    // We'll call the "try_fund" version to catch the error rather than panic the entire test.
    let result = weather_oracle.try_fund(&funder);
    assert!(
        result.is_err() || result.unwrap().is_err(),
        "Funding without enough tokens should fail"
    );

    // Confirm the contract is still not funded
    assert_eq!(weather_oracle.is_funded(), false);
    assert_eq!(weather_oracle.get_funded_balance(), 0);
    assert_eq!(
        weather_oracle.get_funded_by(),
        None,
        "No funder should be recorded yet"
    );
    assert_eq!(
        token.balance(&weather_oracle.address),
        0,
        "Contract's token balance should still be 0"
    );
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
    );

    // change fee receiver
    let new_receiver = Address::generate(&e);
    weather_oracle.set_fee_receiver(&owner, &new_receiver);
    assert_eq!(weather_oracle.get_fee_receiver(), new_receiver);

    // change fee percent
    weather_oracle.set_fee_percent(&owner, &42u128);
    assert_eq!(weather_oracle.get_fee_percent(), 42u128);
}
