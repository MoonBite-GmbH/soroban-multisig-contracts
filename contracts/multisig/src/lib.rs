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

pub mod ttl {
    pub const DAY_IN_LEDGERS: u32 = 17280;

    pub const BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
    pub const LIFETIME_THRESHOLD: u32 = BUMP_AMOUNT - DAY_IN_LEDGERS;
}

#[cfg(test)]
mod tests;
