use soroban_sdk::{contract, contractimpl, symbol_short, token, Address, Env, Symbol, Vec};

use crate::{
    event::{
        BatchCreatedEvent, BatchPaidEvent, ContractPausedEvent, FundRecoveredEvent,
        UsdAddressSetEvent,
    },
    state::*,
};

const COALA_HOUSE_HOLD: Symbol = symbol_short!("COALA_H_H");

#[contract]
pub struct HouseHold;

#[contractimpl]
impl HouseHold {
    pub fn initialize(e: Env, admin: Address, usd_address: Address, funding_account: Address) {
        assert!(!get_initialized(&e), "Contract already initialized");

        // Store basic contract configuration
        set_initialized(&e, &true);
        set_admin(&e, &admin);
        set_usd_address(&e, &usd_address);
        set_funding_account(&e, &funding_account);
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
            .publish((COALA_HOUSE_HOLD, "contract_paused"), event);
    }

    pub fn set_usd_address(e: Env, admin: Address, usd_address: Address) {
        Self::check_auth(&e, &admin);
        set_usd_address(&e, &usd_address);

        let event = UsdAddressSetEvent { usd_address };
        e.events()
            .publish((COALA_HOUSE_HOLD, "usd_address_set"), event);
    }

    pub fn create_batch(e: Env, admin: Address, batch_id: u128, batch_data: Batch) {
        Self::check_auth(&e, &admin);
        assert!(!get_is_contract_paused(&e), "Contract is paused");
        assert!(get_batch(&e, &batch_id).is_none(), "Batch already exists");

        set_batch(&e, &batch_id, &batch_data);

        let event = BatchCreatedEvent {
            batch_id,
            amount_per_beneficiary: batch_data.amount_per_beneficiary,
            reduced_amount_per_beneficiary: batch_data.reduced_amount_per_beneficiary,
        };
        e.events()
            .publish((COALA_HOUSE_HOLD, "batch_created"), event);
    }

    pub fn pay_batch(
        e: Env,
        admin: Address,
        batch_id: u128,
        addresses: Vec<Address>,
        use_reduced: bool,
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

        let usd_address = get_usd_address(&e);
        let token_client: token::Client = token::Client::new(&e, &usd_address);
        let funding_account = get_funding_account(&e);
        let contract_address = e.current_contract_address();

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

        let event = BatchPaidEvent {
            batch_id,
            use_reduced,
        };
        e.events().publish((COALA_HOUSE_HOLD, "batch_paid"), event);
    }

    pub fn recover_funds(e: Env, admin: Address, from: Address, recipient: Address, amount: u128) {
        Self::check_auth(&e, &admin);
        assert!(!get_is_contract_paused(&e), "Contract is paused");

        let usd_address = get_usd_address(&e);
        let token_client: token::Client = token::Client::new(&e, &usd_address);

        token_client.transfer(&from, &recipient, &(amount as i128));

        let event = FundRecoveredEvent {
            from,
            recipient,
            amount,
        };
        e.events()
            .publish((COALA_HOUSE_HOLD, "fund_recovered"), event);
    }

    pub fn set_funding_account(e: Env, admin: Address, funding_account: Address) {
        Self::check_auth(&e, &admin);
        set_funding_account(&e, &funding_account);
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

    pub fn get_usd_address(e: Env) -> Address {
        get_usd_address(&e)
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
}
