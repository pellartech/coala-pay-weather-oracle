#![cfg(test)]
extern crate std;

use crate::contract::{HouseHoldV2, HouseHoldV2Client};
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
// Helper: create_house_hold_v2_contract
// ----------------------------------------------------------------------
fn create_house_hold_v2_contract<'a>(
    e: &Env,
    admin: &Address,
    fee_receiver: &Address,
    fee_percent: &u128,
) -> HouseHoldV2Client<'a> {
    let contract_id = e.register(HouseHoldV2, {});
    let house_hold = HouseHoldV2Client::new(e, &contract_id);
    house_hold.initialize(
        &admin.clone(),
        &fee_receiver.clone(),
        &fee_percent,
    );
    house_hold
}

fn initialize_house_hold_v2_contract<'a>(
    e: &Env,
) -> (
    Address,
    Address,
    Address,
    HouseHoldV2Client<'a>,
    TokenClient<'a>,
    TokenAdminClient<'a>,
) {
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let (token, token_admin) = create_token_contract(&e, &admin);
    let funding = Address::generate(&e);
    token_admin.mint(&funding, &1000);

    let fee_receiver = Address::generate(&e);
    let fee_percent = 5u128;

    let house_hold = create_house_hold_v2_contract(
        &e,
        &admin,
        &fee_receiver,
        &fee_percent,
    );
    token.approve(&funding, &house_hold.address, &1_000_000, &0);
    (
        admin,
        funding,
        fee_receiver,
        house_hold,
        token,
        token_admin,
    )
}

// ----------------------------------------------------------------------
// Test: Initialization
// ----------------------------------------------------------------------
#[test]
fn test_initialization() {
    let (admin, _funding, fee_receiver, house_hold, _, _) =
        initialize_house_hold_v2_contract(&Env::default());
    assert_eq!(house_hold.get_initialized(), true);
    assert_eq!(house_hold.get_is_contract_paused(), false);
    assert_eq!(house_hold.get_admin(), admin);
    assert_eq!(house_hold.get_fee_receiver(), fee_receiver);
    assert_eq!(house_hold.get_fee_percent(), 5u128);
}

// ----------------------------------------------------------------------
// Test: Initialization twice (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Contract already initialized")]
fn test_initialization_twice() {
    let e = Env::default();
    let fee_percent = 5u128;
    let (admin, funding, fee_receiver, house_hold, _, _) = initialize_house_hold_v2_contract(&e);
    house_hold.initialize(
        &admin.clone(),
        &fee_receiver.clone(),
        &fee_percent,
    );
}

// ----------------------------------------------------------------------
// Test: Contract Pause
// ----------------------------------------------------------------------
#[test]
fn test_contract_pause() {
    let e = Env::default();
    let (admin, _funding, _fee_receiver, house_hold, _, _) = initialize_house_hold_v2_contract(&e);

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
    let (_admin, _funding, _fee_receiver, house_hold, _, _) = initialize_house_hold_v2_contract(&e);

    let unauthorized = Address::generate(&e);
    house_hold.set_is_contract_paused(&unauthorized, &true);
}

// ----------------------------------------------------------------------
// Test: Batch Creation with Token
// ----------------------------------------------------------------------
#[test]
fn test_batch_creation() {
    let e = Env::default();
    let (admin, _funding, _fee_receiver, house_hold, token, _) = initialize_house_hold_v2_contract(&e);

    let batch_data = Batch {
        token_address: token.address.clone(),
        amount_per_beneficiary: 100,
        reduced_amount_per_beneficiary: 50,
    };

    house_hold.create_batch(&admin, &100, &token.address, &batch_data);

    let created_batch = house_hold.get_batch(&100).unwrap();
    assert_eq!(
        created_batch.amount_per_beneficiary,
        100,
        "Batch amount per beneficiary not set correctly"
    );

    assert_eq!(
        created_batch.reduced_amount_per_beneficiary,
        50,
        "Batch reduced amount per beneficiary not set correctly"
    );

    assert_eq!(
        created_batch.token_address,
        token.address,
        "Batch token address not set correctly"
    );

    // Test that creating the same batch ID fails
    let res = house_hold.try_create_batch(&admin, &100, &token.address, &batch_data);
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

    let (admin, _funding, _fee_receiver, house_hold, token, _) = initialize_house_hold_v2_contract(&e);

    let unauthorized = Address::generate(&e);

    let batch_data = Batch {
        token_address: token.address.clone(),
        amount_per_beneficiary: 100,
        reduced_amount_per_beneficiary: 50,
    };

    house_hold.create_batch(&unauthorized, &100, &token.address, &batch_data);
}

// ----------------------------------------------------------------------
// Test: Contract Paused Batch Creation (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Contract is paused")]
fn test_contract_paused_batch_creation() {
    let e = Env::default();
    let (admin, _funding, _fee_receiver, house_hold, token, _) = initialize_house_hold_v2_contract(&e);
    house_hold.set_is_contract_paused(&admin, &true);
    house_hold.create_batch(&admin, &100, &token.address, &Batch {
        token_address: token.address.clone(),
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
    let (admin, funding, fee_receiver, house_hold, token, _) =
        initialize_house_hold_v2_contract(&e);

    let batch_data = Batch {
        token_address: token.address.clone(),
        amount_per_beneficiary: 100,
        reduced_amount_per_beneficiary: 50,
    };
    house_hold.create_batch(&admin, &1, &token.address, &batch_data);

    let addr1 = Address::generate(&e);
    let addr2 = Address::generate(&e);
    let addresses = vec![&e, addr1.clone(), addr2.clone()];
    house_hold.pay_batch(&admin, &1, &addresses, &false, &funding);

    // 2×100=200 + 5% of 200 =10 ⇒ starting 1000 – 210 = 790
    assert_eq!(token.balance(&funding), 790);
    assert_eq!(token.balance(&addr1), 100);
    assert_eq!(token.balance(&addr2), 100);
    assert_eq!(token.balance(&fee_receiver), 10);
}

// ----------------------------------------------------------------------
// Test: Batch Payment with Reduced Amount
// ----------------------------------------------------------------------
#[test]
fn test_batch_payment_with_reduced_amount() {
    let e = Env::default();
    let (admin, funding, fee_receiver, house_hold, token, _) =
        initialize_house_hold_v2_contract(&e);

    let batch_data = Batch {
        token_address: token.address.clone(),
        amount_per_beneficiary: 100,
        reduced_amount_per_beneficiary: 50,
    };
    house_hold.create_batch(&admin, &1, &token.address, &batch_data);

    let addr1 = Address::generate(&e);
    let addr2 = Address::generate(&e);
    let addresses = vec![&e, addr1.clone(), addr2.clone()];
    house_hold.pay_batch(&admin, &1, &addresses, &true, &funding);

    // 2×50=100 + 5% of 100=5 ⇒ 1000 – 105 = 895
    assert_eq!(token.balance(&funding), 895);
    assert_eq!(token.balance(&addr1), 50);
    assert_eq!(token.balance(&addr2), 50);
    assert_eq!(token.balance(&fee_receiver), 5);
}

// ----------------------------------------------------------------------
// Test: Unauthorized Batch Payment (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Caller is not the admin")]
fn test_unauthorized_batch_payment() {
    let e = Env::default();
    let (_admin, _funding, _fee_receiver, house_hold, _, _) = initialize_house_hold_v2_contract(&e);

    let unauthorized = Address::generate(&e);
    let batch_id = 1;
    let addresses = vec![&e];
    house_hold.pay_batch(&unauthorized, &batch_id, &addresses, &false, &_funding);
}

// ----------------------------------------------------------------------
// Test: Batch Payment not existing (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Batch does not exist")]
fn test_batch_payment_not_existing() {
    let e = Env::default();
    let (admin, _funding, _fee_receiver, house_hold, _, _) = initialize_house_hold_v2_contract(&e);

    let batch_id = 1;
    let addresses = vec![&e];
    house_hold.pay_batch(&admin, &batch_id, &addresses, &false, &_funding);
}

// ----------------------------------------------------------------------
// Test: Contract Paused Batch Payment (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Contract is paused")]
fn test_contract_paused_batch_payment() {
    let e = Env::default();
    let (admin, _funding, _fee_receiver, house_hold, _, _) = initialize_house_hold_v2_contract(&e);
    house_hold.set_is_contract_paused(&admin, &true);
    house_hold.pay_batch(&admin, &1, &vec![&e], &false, &_funding);
}

// ----------------------------------------------------------------------
// Test: Recover Funds
// ----------------------------------------------------------------------
#[test]
fn test_recover_funds() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, _funding, _fee_receiver, house_hold, token, token_admin) = initialize_house_hold_v2_contract(&e);

    let address1 = Address::generate(&e);

    token_admin.mint(&admin, &1000);

    house_hold.recover_funds(&admin, &token.address, &admin, &admin, &1000);

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
    let (_admin, _funding, _fee_receiver, house_hold, token, _) = initialize_house_hold_v2_contract(&e);
    let unauthorized = Address::generate(&e);
    house_hold.recover_funds(&unauthorized, &token.address, &unauthorized, &unauthorized, &1000);
}

// ----------------------------------------------------------------------
// Test: Contract Paused Recover Funds (should panic)
// ----------------------------------------------------------------------
#[test]
#[should_panic(expected = "Contract is paused")]
fn test_contract_paused_recover_funds() {
    let e = Env::default();
    let (admin, _funding, _fee_receiver, house_hold, token, _) = initialize_house_hold_v2_contract(&e);
    house_hold.set_is_contract_paused(&admin, &true);
    house_hold.recover_funds(&admin, &token.address, &admin, &admin, &1000);
}

// ----------------------------------------------------------------------
// Test: Fee Receiver and Percent Setters
// ----------------------------------------------------------------------
#[test]
fn test_fee_receiver_and_percent_setters() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, _funding, _, house_hold, _, _) =
        initialize_house_hold_v2_contract(&e);

    let new_receiver = Address::generate(&e);
    house_hold.set_fee_receiver(&admin, &new_receiver);
    assert_eq!(house_hold.get_fee_receiver(), new_receiver);

    house_hold.set_fee_percent(&admin, &10u128);
    assert_eq!(house_hold.get_fee_percent(), 10u128);
}

// ----------------------------------------------------------------------
// V2 SPECIFIC TESTS: Multi-Token Support
// ----------------------------------------------------------------------

// ----------------------------------------------------------------------
// Test: Multiple Tokens in Different Batches
// ----------------------------------------------------------------------
#[test]
fn test_multiple_tokens_different_batches() {
    let e = Env::default();
    let (admin, funding, fee_receiver, house_hold, token1, token1_admin) = initialize_house_hold_v2_contract(&e);
    
    // Create second token
    let (token2, token2_admin) = create_token_contract(&e, &admin);
    token2_admin.mint(&funding, &1000);
    token1_admin.mint(&funding, &1000);
    token2.approve(&funding, &house_hold.address, &1_000_000, &0);

    // Create batch with token1
    let batch_data_1 = Batch {
        token_address: token1.address.clone(),
        amount_per_beneficiary: 100,
        reduced_amount_per_beneficiary: 50,
    };
    house_hold.create_batch(&admin, &1, &token1.address, &batch_data_1);

    // Create batch with token2
    let batch_data_2 = Batch {
        token_address: token2.address.clone(),
        amount_per_beneficiary: 200,
        reduced_amount_per_beneficiary: 100,
    };
    house_hold.create_batch(&admin, &2, &token2.address, &batch_data_2);

    // Verify batches have different tokens
    let batch1 = house_hold.get_batch(&1).unwrap();
    let batch2 = house_hold.get_batch(&2).unwrap();
    
    assert_eq!(batch1.token_address, token1.address);
    assert_eq!(batch2.token_address, token2.address);
    assert_ne!(batch1.token_address, batch2.token_address);

    // Pay batch1 with token1
    let addr1 = Address::generate(&e);
    let addresses1 = vec![&e, addr1.clone()];
    house_hold.pay_batch(&admin, &1, &addresses1, &false, &funding);

    // Pay batch2 with token2
    let addr2 = Address::generate(&e);
    let addresses2 = vec![&e, addr2.clone()];
    house_hold.pay_batch(&admin, &2, &addresses2, &false, &funding);

    // Verify balances
    // token1: 1×100=100 + 5% fee=5, total=105, remaining=2000 - 105 = 1895
    assert_eq!(token1.balance(&funding), 1895);
    assert_eq!(token1.balance(&addr1), 100);
    
    // token2: 1×200=200 + 5% fee=10, total=210, remaining=1000 - 210 = 790
    assert_eq!(token2.balance(&funding), 790);
    assert_eq!(token2.balance(&addr2), 200);
}

// ----------------------------------------------------------------------
// Test: Same Token in Multiple Batches
// ----------------------------------------------------------------------
#[test]
fn test_same_token_multiple_batches() {
    let e = Env::default();
    let (admin, funding, fee_receiver, house_hold, token, token_admin) = initialize_house_hold_v2_contract(&e);

    // Mint enough tokens for both batches
    token_admin.mint(&funding, &1000);

    // Create two batches with the same token
    let batch_data_1 = Batch {
        token_address: token.address.clone(),
        amount_per_beneficiary: 100,
        reduced_amount_per_beneficiary: 50,
    };
    house_hold.create_batch(&admin, &1, &token.address, &batch_data_1);

    let batch_data_2 = Batch {
        token_address: token.address.clone(),
        amount_per_beneficiary: 150,
        reduced_amount_per_beneficiary: 75,
    };
    house_hold.create_batch(&admin, &2, &token.address, &batch_data_2);

    // Verify both batches use the same token
    let batch1 = house_hold.get_batch(&1).unwrap();
    let batch2 = house_hold.get_batch(&2).unwrap();
    
    assert_eq!(batch1.token_address, token.address);
    assert_eq!(batch2.token_address, token.address);

    // Pay both batches
    let addr1 = Address::generate(&e);
    let addr2 = Address::generate(&e);
    
    house_hold.pay_batch(&admin, &1, &vec![&e, addr1.clone()], &false, &funding);
    house_hold.pay_batch(&admin, &2, &vec![&e, addr2.clone()], &false, &funding);

    // Verify balances
    // batch1: 1×100=100 + 5% fee=5, total=105
    // batch2: 1×150=150 + 5% fee=7.5, total=157.5 (rounded to 157)
    // total spent: 105 + 157 = 262, remaining: 2000 - 262 = 1738
    assert_eq!(token.balance(&funding), 1738);
    assert_eq!(token.balance(&addr1), 100);
    assert_eq!(token.balance(&addr2), 150);
}

// ----------------------------------------------------------------------
// Test: Batch Creation with Different Token Addresses
// ----------------------------------------------------------------------
#[test]
fn test_batch_creation_different_tokens() {
    let e = Env::default();
    let (admin, _funding, _fee_receiver, house_hold, token1, _) = initialize_house_hold_v2_contract(&e);
    
    // Create second token
    let (token2, _) = create_token_contract(&e, &admin);

    let batch_data = Batch {
        token_address: token1.address.clone(),
        amount_per_beneficiary: 100,
        reduced_amount_per_beneficiary: 50,
    };

    // Create batches with different tokens
    house_hold.create_batch(&admin, &1, &token1.address, &batch_data);
    
    let batch_data_2 = Batch {
        token_address: token2.address.clone(),
        amount_per_beneficiary: 100,
        reduced_amount_per_beneficiary: 50,
    };
    house_hold.create_batch(&admin, &2, &token2.address, &batch_data_2);

    let batch1 = house_hold.get_batch(&1).unwrap();
    let batch2 = house_hold.get_batch(&2).unwrap();

    assert_eq!(batch1.token_address, token1.address);
    assert_eq!(batch2.token_address, token2.address);
    assert_ne!(batch1.token_address, batch2.token_address);
}

// ----------------------------------------------------------------------
// Test: Get Paid Addresses
// ----------------------------------------------------------------------
#[test]
fn test_get_paid_addresses() {
    let e = Env::default();
    let (admin, _funding, _fee_receiver, house_hold, token, _) = initialize_house_hold_v2_contract(&e);

    let batch_data = Batch {
        token_address: token.address.clone(),
        amount_per_beneficiary: 100,
        reduced_amount_per_beneficiary: 50,
    };
    house_hold.create_batch(&admin, &1, &token.address, &batch_data);

    let addr1 = Address::generate(&e);
    let addr2 = Address::generate(&e);
    let addresses = vec![&e, addr1.clone(), addr2.clone()]; // 2 addresses

    // Pay only first address
    house_hold.pay_batch(&admin, &1, &vec![&e, addr1.clone()], &false, &_funding);

    // Check paid status
    let paid_statuses = house_hold.get_paid_addresses(&1, &addresses);
    assert_eq!(paid_statuses.len(), 2);
    assert_eq!(paid_statuses.get(0), Some(true));   // addr1 (paid)
    assert_eq!(paid_statuses.get(1), Some(false));  // addr2 (not paid)
}

 