#![cfg(test)]
extern crate std;

use crate::contract::{HouseHold, HouseHoldClient};
use crate::state::Batch;

use soroban_sdk::vec;
use soroban_sdk::{testutils::Address as _, token, Address, Env};
use token::Client as TokenClient;
use token::StellarAssetClient as TokenAdminClient;

// ----------------------------------------------------------------------
// Helper: create_token_contract
// ----------------------------------------------------------------------
fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let contract_address = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (
        TokenClient::new(e, &contract_address),
        TokenAdminClient::new(e, &contract_address),
    )
}

// ----------------------------------------------------------------------
// Helper: create_house_hold_contract
// ----------------------------------------------------------------------
fn create_house_hold_contract<'a>(
    e: &Env,
    admin: &Address,
    usd_address: &Address,
) -> HouseHoldClient<'a> {
    // Register the contract and create the client
    let contract_id = e.register(HouseHold, {});
    let house_hold = HouseHoldClient::new(e, &contract_id);

    // Initialize, passing the new `start_time`
    house_hold.initialize(admin, usd_address);
    house_hold
}

fn initialize_house_hold_contract<'a>(
    e: &Env,
) -> (
    Address,
    Address,
    HouseHoldClient<'a>,
    TokenClient<'a>,
    TokenAdminClient<'a>,
) {
    e.mock_all_auths();

    let admin = Address::generate(&e);

    // Create token for testing
    let (token, token_admin) = create_token_contract(&e, &admin);

    let usd_address = token.address.clone();

    // Create weather oracle, passing in start_time
    let house_hold = create_house_hold_contract(&e, &admin, &usd_address);
    (admin, usd_address, house_hold, token, token_admin)
}

// ----------------------------------------------------------------------
// Test: Initialization
// ----------------------------------------------------------------------
#[test]
fn test_initialization() {
    let e = Env::default();
    let (admin, usd_address, house_hold, _, _) = initialize_house_hold_contract(&e);

    assert_eq!(
        house_hold.get_initialized(),
        true,
        "Contract not initialized"
    );
    assert_eq!(
        house_hold.get_is_contract_paused(),
        false,
        "Contract is paused"
    );
    assert_eq!(house_hold.get_admin(), admin, "Admin not set correctly");
    assert_eq!(
        house_hold.get_usd_address(),
        usd_address,
        "Usd address not set correctly"
    );
}

// ----------------------------------------------------------------------
// Test: Initialization twice (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Contract already initialized")]
fn test_initialization_twice() {
    let e = Env::default();
    let (admin, usd_address, house_hold, _, _) = initialize_house_hold_contract(&e);
    house_hold.initialize(&admin, &usd_address);
}

// ----------------------------------------------------------------------
// Test: Contract Pause
// ----------------------------------------------------------------------
#[test]
fn test_contract_pause() {
    let e = Env::default();
    let (admin, _usd_address, house_hold, _, _) = initialize_house_hold_contract(&e);

    house_hold.set_is_contract_paused(&admin, &true);

    assert_eq!(
        house_hold.get_is_contract_paused(),
        true,
        "Contract is not paused"
    );

    house_hold.set_is_contract_paused(&admin, &false);

    assert_eq!(
        house_hold.get_is_contract_paused(),
        false,
        "Contract is not paused"
    );
}

// ----------------------------------------------------------------------
// Test: Unauthorized Contract Pause (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Caller is not the admin")]
fn test_unauthorized_contract_pause() {
    let e = Env::default();
    let (_admin, _usd_address, house_hold, _, _) = initialize_house_hold_contract(&e);

    let unauthorized = Address::generate(&e);
    house_hold.set_is_contract_paused(&unauthorized, &true);
}

// ----------------------------------------------------------------------
// Test: Set Usd Address
// ----------------------------------------------------------------------
#[test]
fn test_set_usd_address() {
    let e = Env::default();
    let (admin, _usd_address, house_hold, _, _) = initialize_house_hold_contract(&e);

    let new_usd_address = Address::generate(&e);
    house_hold.set_usd_address(&admin, &new_usd_address);

    assert_eq!(
        house_hold.get_usd_address(),
        new_usd_address,
        "Usd address not set correctly"
    );
}

// ----------------------------------------------------------------------
// Test: Unauthorized Set Usd Address (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Caller is not the admin")]
fn test_unauthorized_set_usd_address() {
    let e = Env::default();
    let (_admin, _usd_address, house_hold, _, _) = initialize_house_hold_contract(&e);

    let unauthorized = Address::generate(&e);
    let new_usd_address = Address::generate(&e);
    house_hold.set_usd_address(&unauthorized, &new_usd_address);
}

// ----------------------------------------------------------------------
// Test: Batch Creation
// ----------------------------------------------------------------------
#[test]
fn test_batch_creation() {
    let e = Env::default();
    let (admin, _usd_address, house_hold, _, _) = initialize_house_hold_contract(&e);

    let batch_data = Batch {
        amount_per_beneficiary: 100,
        reduced_amount_per_beneficiary: 50,
    };

    house_hold.create_batch(&admin, &100, &batch_data);

    assert_eq!(
        house_hold.get_batch(&100).unwrap().amount_per_beneficiary,
        100,
        "Batch amount per beneficiary not set correctly"
    );

    assert_eq!(
        house_hold
            .get_batch(&100)
            .unwrap()
            .reduced_amount_per_beneficiary,
        50,
        "Batch reduced amount per beneficiary not set correctly"
    );

    let res = house_hold.try_create_batch(&admin, &100, &batch_data);

    assert!(res.is_err(), "Batch already exists");
}

// ----------------------------------------------------------------------
// Test: Unauthorized Batch Creation (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Caller is not the admin")]
fn test_unauthorized_batch_creation() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, _usd_address, house_hold, _, _) = initialize_house_hold_contract(&e);

    let unauthorized = Address::generate(&e);

    let batch_data = Batch {
        amount_per_beneficiary: 100,
        reduced_amount_per_beneficiary: 50,
    };

    house_hold.create_batch(&unauthorized, &100, &batch_data);
}

// ----------------------------------------------------------------------
// Test: Contract Paused Batch Creation (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Contract is paused")]
fn test_contract_paused_batch_creation() {
    let e = Env::default();
    let (admin, _usd_address, house_hold, _, _) = initialize_house_hold_contract(&e);
    house_hold.set_is_contract_paused(&admin, &true);
    house_hold.create_batch(&admin, &100, &Batch {
        amount_per_beneficiary: 100,
        reduced_amount_per_beneficiary: 50,
    });
}

// ----------------------------------------------------------------------
// Test: Batch Payment
// ----------------------------------------------------------------------
#[test]
fn test_batch_payment() {
    let e = Env::default();
    let (admin, _usd_address, house_hold, token, token_admin) = initialize_house_hold_contract(&e);

    token_admin.mint(&admin, &1000);
    token.transfer(&admin, &house_hold.address, &1000);

    let batch_id = 1;
    let batch_data = Batch {
        amount_per_beneficiary: 100,
        reduced_amount_per_beneficiary: 50,
    };
    house_hold.create_batch(&admin, &batch_id, &batch_data);

    let address1 = Address::generate(&e);
    let address2 = Address::generate(&e);
    let addresses = vec![&e, address1.clone(), address2.clone()];
    house_hold.pay_batch(&admin, &batch_id, &addresses, &false);

    assert_eq!(
        token.balance(&house_hold.address),
        800,
        "Token balance not updated correctly"
    );

    assert_eq!(
        token.balance(&address1),
        100,
        "Token balance not updated correctly"
    );

    assert_eq!(
        token.balance(&address2),
        100,
        "Token balance not updated correctly"
    );

    let res = house_hold.try_pay_batch(&admin, &batch_id, &addresses, &false);
    assert!(res.is_err(), "Batch already paid");
}

// ----------------------------------------------------------------------
// Test: Batch Payment with Reduced Amount
// ----------------------------------------------------------------------
#[test]
fn test_batch_payment_with_reduced_amount() {
    let e = Env::default();
    let (admin, _usd_address, house_hold, token, token_admin) = initialize_house_hold_contract(&e);

    token_admin.mint(&admin, &1000);
    token.transfer(&admin, &house_hold.address, &1000);

    let batch_id = 1;
    let batch_data = Batch {
        amount_per_beneficiary: 100,
        reduced_amount_per_beneficiary: 50,
    };

    house_hold.create_batch(&admin, &batch_id, &batch_data);

    let address1 = Address::generate(&e);
    let address2 = Address::generate(&e);
    let addresses = vec![&e, address1.clone(), address2.clone()];
    house_hold.pay_batch(&admin, &batch_id, &addresses, &true);

    assert_eq!(
        token.balance(&house_hold.address),
        900,
        "Token balance not updated correctly"
    );

    assert_eq!(
        token.balance(&address1),
        50,
        "Token balance not updated correctly"
    );

    assert_eq!(
        token.balance(&address2),
        50,
        "Token balance not updated correctly"
    );

    let res = house_hold.try_pay_batch(&admin, &batch_id, &addresses, &true);
    assert!(res.is_err(), "Batch already paid");
}

// ----------------------------------------------------------------------
// Test: Unauthorized Batch Payment (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Caller is not the admin")]
fn test_unauthorized_batch_payment() {
    let e = Env::default();
    let (_admin, _usd_address, house_hold, _, _) = initialize_house_hold_contract(&e);

    let unauthorized = Address::generate(&e);
    let batch_id = 1;
    let addresses = vec![&e];
    house_hold.pay_batch(&unauthorized, &batch_id, &addresses, &false);
}

// ----------------------------------------------------------------------
// Test: Batch Payment not existing (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Batch does not exist")]
fn test_batch_payment_not_existing() {
    let e = Env::default();
    let (admin, _usd_address, house_hold, _, _) = initialize_house_hold_contract(&e);

    let batch_id = 1;
    let addresses = vec![&e];
    house_hold.pay_batch(&admin, &batch_id, &addresses, &false);
}

// ----------------------------------------------------------------------
// Test: Contract Paused Batch Payment (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Contract is paused")]
fn test_contract_paused_batch_payment() {
    let e = Env::default();
    let (admin, _usd_address, house_hold, _, _) = initialize_house_hold_contract(&e);
    house_hold.set_is_contract_paused(&admin, &true);
    house_hold.pay_batch(&admin, &1, &vec![&e], &false);
}

// ----------------------------------------------------------------------
// Test: Recover Funds
// ----------------------------------------------------------------------
#[test]
fn test_recover_funds() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, _usd_address, house_hold, token, token_admin) = initialize_house_hold_contract(&e);

    let address1 = Address::generate(&e);

    token_admin.mint(&admin, &1000);

    house_hold.recover_funds(&admin, &admin, &admin, &1000);

    assert_eq!(
        token.balance(&address1),
        0,
        "Token balance not updated correctly"
    );
    assert_eq!(
        token.balance(&admin),
        1000,
        "Token balance not updated correctly"
    );
}

// ----------------------------------------------------------------------
// Test: Unauthorized Recover Funds (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Caller is not the admin")]
fn test_unauthorized_recover_funds() {
    let e = Env::default();
    let (_admin, _usd_address, house_hold, _, _) = initialize_house_hold_contract(&e);
    let unauthorized = Address::generate(&e);
    house_hold.recover_funds(&unauthorized, &unauthorized, &unauthorized, &1000);
}

// ----------------------------------------------------------------------
// Test: Contract Paused Recover Funds (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Contract is paused")]
fn test_contract_paused_recover_funds() {
    let e = Env::default();
    let (admin, _usd_address, house_hold, _, _) = initialize_house_hold_contract(&e);
    house_hold.set_is_contract_paused(&admin, &true);
    house_hold.recover_funds(&admin, &admin, &admin, &1000);
}
