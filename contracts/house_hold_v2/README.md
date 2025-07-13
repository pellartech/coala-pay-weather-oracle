# HouseHold V2 Smart Contract

## Overview

HouseHold V2 is an updated version of the HouseHold smart contract that supports the Stellar Impact marketplace requirements. This version introduces multi-token support and per-batch token selection.

## Key Changes from V1

### 1. Multi-Token Support
- **V1**: Single global token address set during initialization
- **V2**: Token address specified per batch during creation
- **Benefit**: Support for different tokens (USDC, XLM, etc.) across different batches

### 2. Per-Batch Token Selection
- **V1**: All batches used the same token address
- **V2**: Each batch can specify its own token address
- **Benefit**: Flexibility to use different tokens for different projects or payment types

### 3. Updated Function Signatures

#### `initialize`
```rust
// V1
pub fn initialize(
    e: Env,
    admin: Address,
    usd_address: Address,  // Global token address
    funding_account: Address,
    fee_receiver: Address,
    fee_percent: u128,
)

// V2
pub fn initialize(
    e: Env,
    admin: Address,
    funding_account: Address,  // No global token address
    fee_receiver: Address,
    fee_percent: u128,
)
```

#### `create_batch`
```rust
// V1
pub fn create_batch(
    e: Env, 
    admin: Address, 
    batch_id: u128, 
    batch_data: Batch
)

// V2
pub fn create_batch(
    e: Env, 
    admin: Address, 
    batch_id: u128, 
    token_address: Address,  // Token specified per batch
    batch_data: Batch
)
```

#### `recover_funds`
```rust
// V1
pub fn recover_funds(
    e: Env, 
    admin: Address, 
    from: Address, 
    recipient: Address, 
    amount: u128
)

// V2
pub fn recover_funds(
    e: Env, 
    admin: Address, 
    token_address: Address,  // Token must be specified
    from: Address, 
    recipient: Address, 
    amount: u128
)
```

### 4. Updated Data Structures

#### Batch Structure
```rust
// V1
pub struct Batch {
    pub amount_per_beneficiary: u128,
    pub reduced_amount_per_beneficiary: u128,
}

// V2
pub struct Batch {
    pub token_address: Address,  // New field
    pub amount_per_beneficiary: u128,
    pub reduced_amount_per_beneficiary: u128,
}
```

#### Events
```rust
// V1
pub struct BatchCreatedEvent {
    pub batch_id: u128,
    pub amount_per_beneficiary: u128,
    pub reduced_amount_per_beneficiary: u128,
}

// V2
pub struct BatchCreatedEvent {
    pub batch_id: u128,
    pub token_address: Address,  // New field
    pub amount_per_beneficiary: u128,
    pub reduced_amount_per_beneficiary: u128,
}
```

## Migration Strategy

### For Clients
1. **Create smaller, separate batches** per group of recipients with the same amount and token
2. **Specify token and amount per batch** instead of per address
3. **Update event handling** to process the new `token_address` field in `BatchCreatedEvent`

### Benefits
- **Backward Compatibility**: V1 contracts remain functional
- **Gradual Migration**: Clients can migrate to V2 at their own pace
- **Enhanced Flexibility**: Support for multiple tokens and more granular batch management

## Usage Example

```rust
// Initialize V2 contract
client.initialize(&admin, &funding_account, &fee_receiver, &fee_percent);

// Create batch with USDC
let usdc_token = Address::from_contract_id(&usdc_contract_id);
let batch_data = Batch {
    amount_per_beneficiary: 100_000_000, // 100 USDC (6 decimals)
    reduced_amount_per_beneficiary: 50_000_000, // 50 USDC
};
client.create_batch(&admin, &1, &usdc_token, &batch_data);

// Create batch with XLM
let xlm_token = Address::from_contract_id(&xlm_contract_id);
let batch_data = Batch {
    amount_per_beneficiary: 100_000_000, // 100 XLM (7 decimals)
    reduced_amount_per_beneficiary: 50_000_000, // 50 XLM
};
client.create_batch(&admin, &2, &xlm_token, &batch_data);
```

## Contract Symbol

The V2 contract uses the symbol `COALA_V2` to distinguish it from V1 events. 