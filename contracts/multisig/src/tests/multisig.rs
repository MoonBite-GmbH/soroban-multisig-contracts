use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

use super::setup::initialize_multisig_contract;
use crate::storage::MultisigInfo;

#[test]
fn initialize_multisig() {
    let env = Env::default();
    env.mock_all_auths();

    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    let members = vec![&env, member1.clone(), member2.clone(), member3.clone()];

    let multisig = initialize_multisig_contract(
        &env,
        String::from_str(&env, "MultisigName"),
        String::from_str(&env, "Example description of this multisig"),
        members.clone(),
        None,
    );

    let expected_response = MultisigInfo {
        name: String::from_str(&env, "MultisigName"),
        description: String::from_str(&env, "Example description of this multisig"),
        members: members.clone(),
        quorum_bps: 10_000u32,
        version_proposal: 0u32,
    };
    assert_eq!(multisig.query_multisig_info(), expected_response);
    assert_eq!(multisig.query_multisig_members(), members);
    assert_eq!(multisig.query_last_proposal_id(), 0u32);
}

#[test]
fn initialize_multisig_with_custom_quorum() {
    let env = Env::default();
    env.mock_all_auths();

    let member1 = Address::generate(&env);
    let members = vec![&env, member1.clone()];

    let multisig = initialize_multisig_contract(
        &env,
        String::from_str(&env, "MultisigName"),
        String::from_str(&env, "Example description of this multisig"),
        members.clone(),
        5_000u32,
    );

    let expected_response = MultisigInfo {
        name: String::from_str(&env, "MultisigName"),
        description: String::from_str(&env, "Example description of this multisig"),
        members: members.clone(),
        quorum_bps: 5_000u32,
        version_proposal: 0u32,
    };

    assert_eq!(multisig.query_multisig_info(), expected_response);
}

#[test]
#[should_panic = "Multisig: Initialize: initializing contract twice is not allowed"]
fn double_initialize_is_forbidden() {
    let env = Env::default();
    env.mock_all_auths();

    let member1 = Address::generate(&env);
    let members = vec![&env, member1.clone()];

    let multisig = initialize_multisig_contract(
        &env,
        String::from_str(&env, "MultisigName"),
        String::from_str(&env, "Example description of this multisig"),
        members.clone(),
        None,
    );

    multisig.initialize(
        &String::from_str(&env, "MultisigName"),
        &String::from_str(&env, "Example description of this multisig"),
        &members.clone(),
        &None,
    )
}

#[test]
#[should_panic = "Multisig: Initialize: Name longer than 64 characters!"]
fn initialize_name_too_long() {
    let env = Env::default();
    env.mock_all_auths();

    let member1 = Address::generate(&env);
    let members = vec![&env, member1.clone()];

    initialize_multisig_contract(
        &env,
        String::from_bytes(&env, &[0u8; 65]),
        String::from_str(&env, "Example description of this multisig"),
        members.clone(),
        None,
    );
}

#[test]
#[should_panic = "Multisig: Initialize: Description longer than 256 characters!"]
fn initialize_description_too_long() {
    let env = Env::default();
    env.mock_all_auths();

    let member1 = Address::generate(&env);
    let members = vec![&env, member1.clone()];

    initialize_multisig_contract(
        &env,
        String::from_str(&env, "MultisigName"),
        String::from_bytes(&env, &[0u8; 257]),
        members.clone(),
        None,
    );
}

#[test]
#[should_panic = "Multisig: Initialize: Quorum BPS amount set to 100 or lower"]
fn initialize_bps_too_small() {
    let env = Env::default();
    env.mock_all_auths();

    let member1 = Address::generate(&env);
    let members = vec![&env, member1.clone()];

    initialize_multisig_contract(
        &env,
        String::from_str(&env, "MultisigName"),
        String::from_str(&env, "Description"),
        members.clone(),
        100u32,
    );
}

#[test]
#[should_panic = "Multisig: Initialize: Quorum BPS amount set to more than 100%!"]
fn initialize_bps_too_big() {
    let env = Env::default();
    env.mock_all_auths();

    let member1 = Address::generate(&env);
    let members = vec![&env, member1.clone()];

    initialize_multisig_contract(
        &env,
        String::from_str(&env, "MultisigName"),
        String::from_str(&env, "Description"),
        members.clone(),
        10_001u32,
    );
}
