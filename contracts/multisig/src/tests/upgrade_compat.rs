//! End-to-end safety test for the on-chain upgrade.
//!
//! Stages a contract built from the *legacy* sdk-21 source (the wasm matching
//! mainnet's `CCGDOYLH…` deployment) in a fresh test env, drives realistic
//! traffic against it (initialize, create transaction + update proposals,
//! collect signatures, close one), then performs an upgrade-by-proposal that
//! swaps the wasm for the *new* sdk-26 build produced by this crate. Finally,
//! it asserts that every storage slot still reads correctly through the new
//! wasm and that the brand-new `create_member_update_proposal` entrypoint
//! works against the inherited storage.
//!
//! Why this file exists: the deployed multisig (`CCGDOYLH…`) holds 10M PHO.
//! An upgrade that fails to deserialize one storage entry would brick it.
//! These tests are the closest reproduction of the mainnet upgrade we can
//! perform locally.

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, BytesN, Env, String,
};

#[allow(clippy::too_many_arguments)]
mod legacy {
    soroban_sdk::contractimport!(file = "tests_fixtures/legacy_multisig.wasm");
}

#[allow(clippy::too_many_arguments)]
mod current {
    // re-export the just-built sdk-26 wasm; this is what the upgrade swaps in
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/soroban_multisig.wasm");
}

fn s(env: &Env, v: &str) -> String {
    String::from_str(env, v)
}

/// Deploy the legacy wasm, populate state that mirrors what mainnet contains,
/// upgrade to the new wasm via a real on-chain-style proposal, then verify
/// that every piece of state survives and that the new UpdateMembers entrypoint
/// works against the inherited storage.
#[test]
fn upgrade_from_legacy_preserves_all_state_and_unlocks_new_entrypoint() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_700_000_000);

    // ---- Stage 1: deploy and seed the legacy contract ---------------------
    let contract_id = env.register_contract_wasm(None, legacy::WASM);
    let legacy_client = legacy::Client::new(&env, &contract_id);

    let m1 = Address::generate(&env);
    let m2 = Address::generate(&env);
    let m3 = Address::generate(&env);
    let m4 = Address::generate(&env);
    let members = vec![&env, m1.clone(), m2.clone(), m3.clone(), m4.clone()];

    legacy_client.initialize(
        &s(&env, "Advisors Reserves"),
        &s(&env, "Long description of the multisig contract"),
        &members,
        &Some(5_100u32),
    );

    // ---- Stage 2: create and execute one transaction proposal (closed) ----
    let token_admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_addr = token.address();
    // mint to the multisig so the transfer succeeds
    soroban_sdk::token::StellarAssetClient::new(&env, &token_addr).mint(&contract_id, &1_000_000);

    legacy_client.create_transaction_proposal(
        &m1,
        &s(&env, "Pay advisor X"),
        &s(&env, "First disbursement"),
        &recipient,
        &500_000u64,
        &token_addr,
        &None,
    );
    let closed_tx_pid = legacy_client.query_last_proposal_id();
    legacy_client.sign_proposal(&m1, &closed_tx_pid);
    legacy_client.sign_proposal(&m2, &closed_tx_pid);
    legacy_client.sign_proposal(&m3, &closed_tx_pid);
    legacy_client.execute_proposal(&m1, &closed_tx_pid);

    // ---- Stage 3: create an open transaction proposal (not yet executed) --
    legacy_client.create_transaction_proposal(
        &m2,
        &s(&env, "Pay advisor Y"),
        &s(&env, "Second disbursement"),
        &recipient,
        &250_000u64,
        &token_addr,
        &None,
    );
    let open_tx_pid = legacy_client.query_last_proposal_id();
    legacy_client.sign_proposal(&m1, &open_tx_pid); // only one signature

    // snapshot state before upgrade
    let pre_info = legacy_client.query_multisig_info();
    let pre_proposals = legacy_client.query_all_proposals();
    let pre_open_sigs = legacy_client.query_signatures(&open_tx_pid);
    let pre_last_id = legacy_client.query_last_proposal_id();

    assert_eq!(pre_info.members.len(), 4);
    assert_eq!(pre_proposals.len(), 2);
    assert_eq!(pre_info.version_proposal, 0);
    assert_eq!(pre_last_id, 2);

    // ---- Stage 4: propose+execute an UpdateContract pointing at the NEW wasm
    let new_wasm_hash: BytesN<32> = env.deployer().upload_contract_wasm(current::WASM);

    legacy_client.create_update_proposal(
        &m1,
        &s(&env, "Upgrade to sdk-26"),
        &s(&env, "Phoenix Multisig v2: UpdateMembers"),
        &new_wasm_hash,
        &None,
    );
    let upgrade_pid = legacy_client.query_last_proposal_id();
    legacy_client.sign_proposal(&m1, &upgrade_pid);
    legacy_client.sign_proposal(&m2, &upgrade_pid);
    legacy_client.sign_proposal(&m3, &upgrade_pid);
    legacy_client.execute_proposal(&m1, &upgrade_pid);

    // ---- Stage 5: rebind client to the new interface and verify state ----
    let new_client = current::Client::new(&env, &contract_id);

    let post_info = new_client.query_multisig_info();
    assert_eq!(post_info.name, pre_info.name);
    assert_eq!(post_info.description, pre_info.description);
    assert_eq!(post_info.members, pre_info.members);
    assert_eq!(post_info.quorum_bps, pre_info.quorum_bps);
    // upgrade itself bumps the version counter
    assert_eq!(post_info.version_proposal, 1);

    // last_proposal_id preserved (and incremented by the upgrade itself)
    assert_eq!(new_client.query_last_proposal_id(), 3);

    // every legacy proposal still deserializes and matches pre-upgrade content
    let post_proposals = new_client.query_all_proposals();
    assert_eq!(post_proposals.len(), 3);

    // proposal #1 was closed by execution
    let p1 = new_client.query_proposal(&closed_tx_pid);
    assert_eq!(p1.status, current::ProposalStatus::Closed);
    assert_eq!(p1.title, s(&env, "Pay advisor X"));

    // proposal #2 is still Open with the one signature it had
    let p2 = new_client.query_proposal(&open_tx_pid);
    assert_eq!(p2.status, current::ProposalStatus::Open);
    assert_eq!(p2.title, s(&env, "Pay advisor Y"));
    let post_open_sigs = new_client.query_signatures(&open_tx_pid);
    assert_eq!(post_open_sigs.len(), pre_open_sigs.len());

    // the upgrade proposal itself is closed
    let pu = new_client.query_proposal(&upgrade_pid);
    assert_eq!(pu.status, current::ProposalStatus::Closed);

    // The token balance held by the multisig is also unchanged
    let token_client = soroban_sdk::token::TokenClient::new(&env, &token_addr);
    assert_eq!(token_client.balance(&contract_id), 500_000);

    // ---- Stage 6: existing Open proposal can still be signed + executed --
    new_client.sign_proposal(&m2, &open_tx_pid);
    new_client.sign_proposal(&m3, &open_tx_pid);
    new_client.execute_proposal(&m1, &open_tx_pid);
    assert_eq!(
        new_client.query_proposal(&open_tx_pid).status,
        current::ProposalStatus::Closed
    );
    assert_eq!(token_client.balance(&contract_id), 250_000);

    // ---- Stage 7: brand-new UpdateMembers entrypoint works on legacy state
    let new_a = Address::generate(&env);
    let new_b = Address::generate(&env);
    let new_c = Address::generate(&env);
    let new_members = vec![&env, new_a.clone(), new_b.clone(), new_c.clone()];

    new_client.create_member_update_proposal(
        &m1,
        &s(&env, "Rotate advisors"),
        &s(&env, "Replace original 4 with new 3"),
        &new_members,
        &None,
    );
    let member_pid = new_client.query_last_proposal_id();
    assert_eq!(member_pid, 4);

    // 3 of the original 4 sign — current members at execution time still apply
    new_client.sign_proposal(&m1, &member_pid);
    new_client.sign_proposal(&m2, &member_pid);
    new_client.sign_proposal(&m3, &member_pid);
    new_client.execute_proposal(&m1, &member_pid);

    let final_members = new_client.query_multisig_members();
    assert_eq!(final_members, new_members);

    // sanity: a previous member can no longer create a proposal
    let err = new_client.try_create_member_update_proposal(
        &m1,
        &s(&env, "Hostile"),
        &s(&env, "..."),
        &vec![&env, m1.clone()],
        &None,
    );
    assert_eq!(err, Err(Ok(current::ContractError::UnauthorizedNotAMember)));

    // the inherited proposal types still match what the upgrade saved
    let upgrade_proposal = new_client.query_proposal(&upgrade_pid);
    match upgrade_proposal.proposal {
        current::ProposalType::UpdateContract(hash) => assert_eq!(hash, new_wasm_hash),
        _ => panic!("expected UpdateContract proposal type"),
    }
}

/// Variant that re-exercises an upgrade where the open proposal between
/// upgrades is itself an UpdateMembers — proving that a fresh new wasm can
/// finish executing the contract logic added in that same wasm.
#[test]
fn upgrade_then_immediate_member_rotation_works() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_700_000_000);

    let contract_id = env.register_contract_wasm(None, legacy::WASM);
    let legacy_client = legacy::Client::new(&env, &contract_id);

    let m1 = Address::generate(&env);
    let m2 = Address::generate(&env);
    let m3 = Address::generate(&env);
    let m4 = Address::generate(&env);
    let members = vec![&env, m1.clone(), m2.clone(), m3.clone(), m4.clone()];

    legacy_client.initialize(&s(&env, "M"), &s(&env, "D"), &members, &Some(5_100u32));

    // upgrade
    let new_wasm_hash: BytesN<32> = env.deployer().upload_contract_wasm(current::WASM);
    legacy_client.create_update_proposal(
        &m1,
        &s(&env, "Up"),
        &s(&env, "Up"),
        &new_wasm_hash,
        &None,
    );
    let upid = legacy_client.query_last_proposal_id();
    legacy_client.sign_proposal(&m1, &upid);
    legacy_client.sign_proposal(&m2, &upid);
    legacy_client.sign_proposal(&m3, &upid);
    legacy_client.execute_proposal(&m1, &upid);

    // rotate members through the new entrypoint
    let new_client = current::Client::new(&env, &contract_id);
    let n1 = Address::generate(&env);
    let n2 = Address::generate(&env);
    let n3 = Address::generate(&env);
    let n4 = Address::generate(&env);
    let n5 = Address::generate(&env);
    let new_members = vec![
        &env,
        n1.clone(),
        n2.clone(),
        n3.clone(),
        n4.clone(),
        n5.clone(),
    ];

    new_client.create_member_update_proposal(
        &m1,
        &s(&env, "Rotate"),
        &s(&env, "..."),
        &new_members,
        &None,
    );
    let mpid = new_client.query_last_proposal_id();
    new_client.sign_proposal(&m1, &mpid);
    new_client.sign_proposal(&m2, &mpid);
    new_client.sign_proposal(&m3, &mpid);
    new_client.execute_proposal(&m1, &mpid);

    assert_eq!(new_client.query_multisig_members(), new_members);
    // version_proposal only bumps on UpdateContract, not UpdateMembers
    assert_eq!(new_client.query_multisig_info().version_proposal, 1);
}
