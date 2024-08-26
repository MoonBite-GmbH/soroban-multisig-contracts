#![no_std]

mod contract;
mod error;
mod storage;

pub mod token_contract {
    // The import will code generate:
    // - A ContractClient type that can be used to invoke functions on the contract.
    // - Any types in the contract that were annotated with #[contracttype].
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
    );
}

pub const ONE_HOUR: u64 = 3_600u64;
pub const SEVEN_DAYS_EXPIRATION_DATE: u64 = 604_800u64;

#[cfg(test)]
mod tests;
