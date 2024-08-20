# Weather Oracle Contract

This repository contains a Soroban smart contract written in Rust that implements a simple Weather Oracle. The oracle records and validates weather data across different epochs and triggers a token transfer when certain conditions are met. This contract can be used for various decentralized applications that require weather data as a criterion for triggering events.

## Overview

The Weather Oracle contract allows a designated relayer to submit weather data for each epoch. The contract maintains the continuity of data submissions and verifies if certain thresholds are met. If the threshold conditions are satisfied over a required number of consecutive epochs, a token transfer is triggered to a specified recipient.

## Features

- Initialize Contract: Set up the contract with the contract owner, relayer, epoch duration, continuity requirements, threshold value, token, and recipient address.
- Submit Weather Data: The designated relayer submits the weather data for a specific epoch.
- Continuity Check: The contract checks if the submitted data meets the threshold condition. If the condition is met over a required number of consecutive epochs, a token transfer to the recipient is triggered.
- Data Retrieval: The contract provides functions to retrieve the weather data for any epoch, as well as the current contract state.

## Contract Structure

### Data Structures

- EpochData: A struct to store the weather data for each epoch.
- DataKey: An enum representing the different keys used to store contract state data.

### Main Functions

- initialize: Initializes the contract, setting up the contract owner, relayer, epoch duration, continuity requirements, threshold, token, and recipient.
- set_value: Allows the relayer to submit the weather data for a specific epoch.
- get_value: Retrieves the weather data for a specific epoch.
- set_continuity_requirement: Allows the contract owner to set the number of consecutive epochs required to trigger the token transfer.
- set_threshold: Allows the contract owner to set the threshold value for the weather data.
- get_contract_owner: Retrieves the contract owner's address.
- get_relayer: Retrieves the relayer's address.
- get_continuity_requirement: Retrieves the current continuity requirement.
- get_threshold: Retrieves the current threshold value.
- get_last_update_time: Retrieves the timestamp of the last update.
- get_epoch_data: Retrieves the weather data for a specific epoch.
- get_current_epoch: Retrieves the current epoch number.
- get_continuity: Retrieves the current continuity count.

## Usage

### Prerequisites

- Rust with the wasm32-unknown-unknown target installed.
- Soroban CLI to deploy and interact with the contract.

### Compilation

To compile the contract, run the following command:

`cargo build --target wasm32-unknown-unknown --release`

### Deployment

You can deploy the contract to the Soroban network using the Soroban CLI:

`soroban contract deploy --wasm <path_to_compiled_wasm>`

### Interacting with the Contract

Use the Soroban CLI or other Soroban-compatible tools to call the functions of the contract.

## Testing

Unit tests for the contract are provided in the `test` module. To run the tests, execute:

`cargo test`
