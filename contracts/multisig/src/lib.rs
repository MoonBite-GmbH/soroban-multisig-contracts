#![no_std]
// We intentionally keep several pre-SDK-26 APIs in use to preserve the exact
// behavior of the deployed contract:
//   - `env.events().publish((..), ..)` emits the same legacy event topic shape
//     that off-chain indexers currently consume. Migrating to `#[contractevent]`
//     would silently change topics.
//   - `env.register_contract` and `env.budget()` in tests are equivalent to
//     their renamed counterparts and don't affect the produced wasm.
#![allow(deprecated)]

mod contract;
mod error;
mod storage;

pub mod token_contract {
    // The import will code generate:
    // - A ContractClient type that can be used to invoke functions on the contract.
    // - Any types in the contract that were annotated with #[contracttype].
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/soroban_token_contract.wasm"
    );
}
// Values used to extend the TTL of storage
pub const DAY_IN_LEDGERS: u32 = 17280;
pub const BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
pub const LIFETIME_THRESHOLD: u32 = BUMP_AMOUNT - DAY_IN_LEDGERS;

// Values used to track time of proposals lifespan
pub const ONE_HOUR: u64 = 3_600u64;
pub const SEVEN_DAYS_EXPIRATION_DATE: u64 = 604_800u64;

// helper value that represents Soroban's zero address
pub const SOROBAN_ZERO_ADDRESS: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

#[cfg(test)]
mod tests;
