#![no_std]

use soroban_sdk::{
    contract, contractimpl, contractmeta, contracttype, log, vec, Address, BytesN, Env, IntoVal,
    String, Symbol, Val, Vec,
};

// Values used to extend the TTL of storage
pub const DAY_IN_LEDGERS: u32 = 17280;
pub const BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
pub const LIFETIME_THRESHOLD: u32 = BUMP_AMOUNT - DAY_IN_LEDGERS;

// Metadata that is added on to the WASM custom section
contractmeta!(
    key = "Description",
    val = "Soroban Multisig Deployer Contract"
);

#[contract]
pub struct MultisigDeployer;

#[contractimpl]
impl MultisigDeployer {
    #[allow(dead_code)]
    pub fn initialize(env: Env, multisig_wasm_hash: BytesN<32>) {
        if is_initialized(&env) {
            log!(
                &env,
                "Multisig Deployer: Initialize: initializing the contract twice is not allowed"
            );
            panic!("Multisig Deployer: Initialize: initializing the contract twice is not allowed");
        }
        set_initialized(&env);

        set_wasm_hash(&env, &multisig_wasm_hash);
    }

    #[allow(dead_code)]
    pub fn deploy_new_multisig(
        env: Env,
        deployer: Address,
        salt: BytesN<32>,
        name: String,
        description: String,
        members: Vec<Address>,
        quorum_bps: Option<u32>,
    ) -> Address {
        deployer.require_auth();
        let multisig_wasm_hash = get_wasm_hash(&env);

        let deployed_multisig = env
            .deployer()
            .with_address(deployer, salt)
            .deploy(multisig_wasm_hash);

        let init_fn = Symbol::new(&env, "initialize");
        let init_fn_args: Vec<Val> = vec![
            &env,
            name.into_val(&env),
            description.into_val(&env),
            members.into_val(&env),
            quorum_bps.into_val(&env),
        ];
        let _: Val = env.invoke_contract(&deployed_multisig, &init_fn, init_fn_args);

        deployed_multisig
    }
}

// ---------- Storage types ----------

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    IsInitialized,
    MultisigWasmHash,
}

pub fn set_initialized(env: &Env) {
    env.storage().instance().set(&DataKey::IsInitialized, &());
    env.storage()
        .instance()
        .extend_ttl(LIFETIME_THRESHOLD, BUMP_AMOUNT);
}

pub fn is_initialized(env: &Env) -> bool {
    let is_initialized = env
        .storage()
        .instance()
        .get::<_, ()>(&DataKey::IsInitialized)
        .is_some();

    env.storage()
        .instance()
        .has(&DataKey::IsInitialized)
        .then(|| {
            env.storage()
                .instance()
                .extend_ttl(LIFETIME_THRESHOLD, BUMP_AMOUNT)
        });

    is_initialized
}

pub fn set_wasm_hash(env: &Env, hash: &BytesN<32>) {
    env.storage()
        .instance()
        .set(&DataKey::MultisigWasmHash, hash);
    env.storage()
        .instance()
        .extend_ttl(LIFETIME_THRESHOLD, BUMP_AMOUNT);
}

pub fn get_wasm_hash(env: &Env) -> BytesN<32> {
    let wasm_hash = env
        .storage()
        .instance()
        .get(&DataKey::MultisigWasmHash)
        .unwrap();
    env.storage()
        .instance()
        .has(&DataKey::MultisigWasmHash)
        .then(|| {
            env.storage()
                .instance()
                .extend_ttl(LIFETIME_THRESHOLD, BUMP_AMOUNT)
        });

    wasm_hash
}

#[cfg(test)]
mod tests;
