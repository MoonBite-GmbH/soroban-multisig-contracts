#![no_std]

use soroban_sdk::{
    contract, contractimpl, contractmeta, contracttype, log, vec, Address, BytesN, Env, IntoVal,
    String, Symbol, Val, Vec,
};

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
}

pub fn is_initialized(env: &Env) -> bool {
    env.storage()
        .instance()
        .get::<_, ()>(&DataKey::IsInitialized)
        .is_some()
}

pub fn set_wasm_hash(env: &Env, hash: &BytesN<32>) {
    env.storage()
        .instance()
        .set(&DataKey::MultisigWasmHash, hash);
}

pub fn get_wasm_hash(env: &Env) -> BytesN<32> {
    env.storage()
        .instance()
        .get(&DataKey::MultisigWasmHash)
        .unwrap()
}

#[cfg(test)]
mod tests;
