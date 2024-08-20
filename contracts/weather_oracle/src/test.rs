#![cfg(test)]
extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};
use token::Client as TokenClient;
use token::StellarAssetClient as TokenAdminClient;

fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let contract_address = e.register_stellar_asset_contract(admin.clone());
    (
        TokenClient::new(e, &contract_address),
        TokenAdminClient::new(e, &contract_address),
    )
}

fn create_weather_oracle_contract<'a>(
    e: &Env,
    caller: &Address,
    relayer: &Address,
    epoch_duration: &u32,
    continuity_requirement: &u32,
    threshold: &u32,
    token: &Address,
    recipient: &Address

) -> WeatherOracleClient<'a> {
    let weather_oracle = WeatherOracleClient::new(e, &e.register_contract(None, crate::WeatherOracle {}));
    weather_oracle.initialize(
        caller,
        relayer,
        epoch_duration,
        continuity_requirement,
        threshold,
        token,
        recipient,
    );
    weather_oracle
}

#[test]
fn test() {
    let e = Env::default();
    e.mock_all_auths();

    let owner = Address::generate(&e);
    let relayer = Address::generate(&e);
    let recipient: Address = Address::generate(&e);

    let (token, token_admin) = create_token_contract(&e, &owner);

    let weather_oracle = create_weather_oracle_contract(
        &e,
        &owner,
        &relayer,
        &(60 * 60 * 24),
        &2,
        &10,
        &token.address,
        &recipient,
    );

    // test relayer set correctly with get_relayer
    assert_eq!(weather_oracle.get_relayer(), relayer);

    // test continuity requirement set correctly with get_continuity_requirement
    assert_eq!(weather_oracle.get_continuity_requirement(), 2);

    // test threshold set correctly with get_threshold
    assert_eq!(weather_oracle.get_threshold(), 10);

    // test contract owner set correctly with get_contract_owner
    assert_eq!(weather_oracle.get_contract_owner(), owner);

    // mint and send tokens to contract
    token_admin.mint(&owner, &1000);
    token.transfer(&owner, &weather_oracle.address, &1000);

    assert_eq!(token.balance(&weather_oracle.address), 1000);

    let current_epoch = weather_oracle.get_current_epoch();

    std::println!("current epoch: {:?}", current_epoch);
    assert!(current_epoch == 0);

    let epoch_data = weather_oracle.get_epoch_data(&0);
    std::println!("current epoch: {:?}", epoch_data);

    assert!(epoch_data.value == 0);

    e.ledger().set_timestamp(e.ledger().timestamp() + 2 * 60 * 60 * 24);

    let current_epoch = weather_oracle.get_current_epoch();
    // log current epoch
    std::println!("current epoch: {:?}", current_epoch);
    assert!(current_epoch == 2);

    // test set value
    weather_oracle.set_value(&relayer, &50, &1);

    // expect continuity to be 1
    let continuity = weather_oracle.get_continuity();
    assert!(continuity == 1);

    // do another update
    e.ledger().set_timestamp(e.ledger().timestamp() + (60 * 60 * 24));

    let updated_epoch = weather_oracle.get_current_epoch();

    // expect current epoch to increment
    assert!(updated_epoch == 3);

    weather_oracle.set_value(&relayer, &50, &2);

    // expect continuity to be 1
    let continuity = weather_oracle.get_continuity();
    assert!(continuity == 2);
    

    // expect tokens to be transferred

    // expect contract to have 0 balance
    assert_eq!(token.balance(&weather_oracle.address), 0);

    // expect recipient to have 1000 tokens
    assert_eq!(token.balance(&recipient), 1000);

}
