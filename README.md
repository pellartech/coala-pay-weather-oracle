# Soroban Project

## Project Structure

This repository uses the recommended structure for a Soroban project:
```text
.
├── contracts
│   └── hello_world
│       ├── src
│       │   ├── lib.rs
│       │   └── test.rs
│       └── Cargo.toml
├── Cargo.toml
└── README.md
```

- New Soroban contracts can be put in `contracts`, each in their own directory. There is already a `hello_world` contract in there to get you started.
- If you initialized this project with any other example contracts via `--with-example`, those contracts will be in the `contracts` directory as well.
- Contracts should have their own `Cargo.toml` files that rely on the top-level `Cargo.toml` workspace for their dependencies.
- Frontend libraries can be added to the top-level directory as well. If you initialized this project with a frontend template via `--frontend-template` you will have those files already included.



## Deployment steps

soroban contract install \
  --network testnet \
  --source alice \
  --wasm target/wasm32-unknown-unknown/release/hello_world.wasm

soroban contract deploy \
  --wasm-hash ec16eeba82565a7ed32cc6c645871fc4101129fda174a8babfa8e27ebd9b307b \
  --source alice \
  --network testnet

soroban contract invoke \
  --id CA2APPDO4GXPNMNTGLAPZGV7X4WKL32K7TWAMBNVEUMXSHYMCHHV5WIW \
  --source alice \
  --network testnet \
  -- \
  hello \
  --to RPC
