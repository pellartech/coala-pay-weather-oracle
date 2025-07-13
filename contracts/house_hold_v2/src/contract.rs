use soroban_sdk::{contract, contractimpl, symbol_short, token, Address, Env, Symbol, Vec};

use crate::{
    event::{
        BatchCreatedEvent, BatchPaidEvent, ContractPausedEvent, FundRecoveredEvent,
    },
    state::*,
};

const COALA_HOUSE_HOLD_V2: Symbol = symbol_short!("COALA_V2");

#[contract]
pub struct HouseHoldV2;

#[contractimpl]
impl HouseHoldV2 {
    pub fn initialize(
        e: Env,
        admin: Address,
        fee_receiver: Address,
        fee_percent: u128,
    ) {
        assert!(!get_initialized(&e), "Contract already initialized");

        set_initialized(&e, &true);
        set_admin(&e, &admin);
        set_fee_receiver(&e, &fee_receiver);
        set_fee_percent(&e, &fee_percent);
    }

    fn check_auth(e: &Env, admin: &Address) {
        let ad = admin.clone();
        ad.require_auth();
        assert_eq!(ad, get_admin(e), "Caller is not the admin");
    }

    pub fn set_is_contract_paused(e: Env, admin: Address, is_paused: bool) {
        Self::check_auth(&e, &admin);
        set_is_contract_paused(&e, &is_paused);

        let event = ContractPausedEvent { is_paused };
        e.events()
            .publish((COALA_HOUSE_HOLD_V2, "contract_paused"), event);
    }

    pub fn create_batch(
        e: Env, 
        admin: Address, 
        batch_id: u128, 
        token_address: Address,
        batch_data: Batch
    ) {
        Self::check_auth(&e, &admin);
        assert!(!get_is_contract_paused(&e), "Contract is paused");
        assert!(get_batch(&e, &batch_id).is_none(), "Batch already exists");

        // Create batch with token address
        let batch_with_token = Batch {
            token_address: token_address.clone(),
            amount_per_beneficiary: batch_data.amount_per_beneficiary,
            reduced_amount_per_beneficiary: batch_data.reduced_amount_per_beneficiary,
        };

        set_batch(&e, &batch_id, &batch_with_token);

        let event = BatchCreatedEvent {
            batch_id,
            token_address,
            amount_per_beneficiary: batch_data.amount_per_beneficiary,
            reduced_amount_per_beneficiary: batch_data.reduced_amount_per_beneficiary,
        };
        e.events()
            .publish((COALA_HOUSE_HOLD_V2, "batch_created"), event);
    }

    pub fn pay_batch(
        e: Env,
        admin: Address,
        batch_id: u128,
        addresses: Vec<Address>,
        use_reduced: bool,
        funding_account: Address,
    ) {
        Self::check_auth(&e, &admin);
        assert!(!get_is_contract_paused(&e), "Contract is paused");
        assert!(get_batch(&e, &batch_id).is_some(), "Batch does not exist");

        let batch = get_batch(&e, &batch_id).unwrap();

        let amount_per_beneficiary = if use_reduced {
            batch.reduced_amount_per_beneficiary
        } else {
            batch.amount_per_beneficiary
        };

        // Use the token address from the batch
        let token_client: token::Client = token::Client::new(&e, &batch.token_address);
        let contract_address = e.current_contract_address();

        // calculate fee
        let count = addresses.len() as u128;
        let total = amount_per_beneficiary * count;
        let fee_pct = get_fee_percent(&e);
        let fee = total * fee_pct / 100;

        for address in addresses {
            assert!(
                !get_paid_address(&e, &batch_id, &address),
                "Address already paid"
            );
            set_paid_address(&e, &batch_id, &address);
            token_client.transfer_from(
                &contract_address,
                &funding_account,
                &address,
                &(amount_per_beneficiary as i128),
            );
        }

        // single fee transfer
        token_client.transfer_from(
            &contract_address,
            &funding_account,
            &get_fee_receiver(&e),
            &(fee as i128),
        );

        let event = BatchPaidEvent {
            batch_id,
            use_reduced,
        };
        e.events().publish((COALA_HOUSE_HOLD_V2, "batch_paid"), event);
    }

    pub fn recover_funds(
        e: Env, 
        admin: Address, 
        token_address: Address,
        from: Address, 
        recipient: Address, 
        amount: u128
    ) {
        Self::check_auth(&e, &admin);
        assert!(!get_is_contract_paused(&e), "Contract is paused");

        let token_client: token::Client = token::Client::new(&e, &token_address);

        token_client.transfer(&from, &recipient, &(amount as i128));

        let event = FundRecoveredEvent {
            from,
            recipient,
            amount,
        };
        e.events()
            .publish((COALA_HOUSE_HOLD_V2, "fund_recovered"), event);
    }


    
    // -----------------
    // Getter methods
    // -----------------
    pub fn get_initialized(e: Env) -> bool {
        get_initialized(&e)
    }

    pub fn get_admin(e: Env) -> Address {
        get_admin(&e)
    }

    pub fn get_is_contract_paused(e: Env) -> bool {
        get_is_contract_paused(&e)
    }

    pub fn get_batch(e: Env, batch_id: u128) -> Option<Batch> {
        get_batch(&e, &batch_id)
    }

    pub fn get_paid_address(e: Env, batch_id: u128, address: Address) -> bool {
        get_paid_address(&e, &batch_id, &address)
    }

    pub fn get_paid_addresses(e: Env, batch_id: u128, addresses: Vec<Address>) -> Vec<bool> {
        get_paid_addresses(&e, &batch_id, &addresses)
    }

    pub fn set_fee_receiver(e: Env, admin: Address, fee: Address) {
        Self::check_auth(&e, &admin);
        set_fee_receiver(&e, &fee);
    }
    
    pub fn get_fee_receiver(e: Env) -> Address {
        get_fee_receiver(&e)
    }
    
    pub fn set_fee_percent(e: Env, admin: Address, pct: u128) {
        Self::check_auth(&e, &admin);
        set_fee_percent(&e, &pct);
    }
    
    pub fn get_fee_percent(e: Env) -> u128 {
        get_fee_percent(&e)
    }
} 