use crate::{MultisigDeployer, MultisigDeployerClient};
#[cfg(test)]
use soroban_sdk::{testutils::Address as _, vec, Address, BytesN, Env, String, Symbol, Val, Vec};

// The contract that will be deployed by the deployer contract.
#[allow(clippy::too_many_arguments)]
mod multisig {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/soroban_multisig.wasm"
    );
}

#[test]
fn test_deploy_multisig_from_contract() {
    let env = Env::default();
    let deployer_client =
        MultisigDeployerClient::new(&env, &env.register_contract(None, MultisigDeployer));

    // Upload the Wasm to be deployed from the deployer contract.
    // This can also be called from within a contract if needed.
    let wasm_hash = env.deployer().upload_contract_wasm(multisig::WASM);
    deployer_client.initialize(&wasm_hash);

    env.mock_all_auths();

    let salt = BytesN::from_array(&env, &[0; 32]);
    let msig_members = vec![&env, Address::generate(&env), Address::generate(&env)];

    let deployed_multisig = deployer_client.deploy_new_multisig(
        &Address::generate(&env), // deployer / sender
        &salt,
        &String::from_str(&env, "TestMSig"),
        &String::from_str(&env, "TestMSig description"),
        &msig_members,
        &None::<u32>,
    );

    // now verify the deployment
    let query = Symbol::new(&env, "query_multisig_members");
    let arguments: Vec<Val> = vec![&env];
    env.mock_all_auths();
    let members_result: Vec<Address> = env.invoke_contract(&deployed_multisig, &query, arguments);
    assert_eq!(members_result, msig_members);
}

#[test]
#[should_panic(
    expected = "Multisig Deployer: Initialize: initializing the contract twice is not allowed"
)]
fn initialize_twice() {
    let env = Env::default();
    let deployer_client =
        MultisigDeployerClient::new(&env, &env.register_contract(None, MultisigDeployer));

    let wasm_hash = env.deployer().upload_contract_wasm(multisig::WASM);
    deployer_client.initialize(&wasm_hash);
    deployer_client.initialize(&wasm_hash);
}
