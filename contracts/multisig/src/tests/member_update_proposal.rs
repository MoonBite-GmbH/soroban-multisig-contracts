extern crate std;

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

use crate::{error::ContractError, tests::setup::initialize_multisig_contract};

const NAME: &str = "MultisigName";
const DESC: &str = "Example description of this multisig";

fn s(env: &Env, v: &str) -> String {
    String::from_str(env, v)
}

fn setup_quad(env: &Env) -> (Address, Address, Address, Address) {
    (
        Address::generate(env),
        Address::generate(env),
        Address::generate(env),
        Address::generate(env),
    )
}

#[test]
fn member_update_proposal_replaces_members_after_quorum() {
    let env = Env::default();
    env.mock_all_auths();

    let (m1, m2, m3, m4) = setup_quad(&env);
    let members = vec![&env, m1.clone(), m2.clone(), m3.clone(), m4.clone()];

    let multisig = initialize_multisig_contract(
        &env,
        s(&env, NAME),
        s(&env, DESC),
        members.clone(),
        Some(5_100u32),
    );

    // create a brand-new 3-member set replacing the original 4
    let new_a = Address::generate(&env);
    let new_b = Address::generate(&env);
    let new_c = Address::generate(&env);
    let new_members = vec![&env, new_a.clone(), new_b.clone(), new_c.clone()];

    multisig.create_member_update_proposal(
        &m1,
        &s(&env, "Rotate members"),
        &s(&env, "Replace advisors with new cohort"),
        &new_members,
        &None,
    );
    let proposal_id = multisig.query_last_proposal_id();

    // 3 of 4 sign — passes quorum (51%)
    multisig.sign_proposal(&m1, &proposal_id);
    multisig.sign_proposal(&m2, &proposal_id);
    multisig.sign_proposal(&m3, &proposal_id);

    // before execution, the membership is still the original 4
    let info_before = multisig.query_multisig_info();
    assert_eq!(info_before.members.len(), 4);

    multisig.execute_proposal(&m1, &proposal_id);

    // after execution, members are replaced
    let info_after = multisig.query_multisig_info();
    assert_eq!(info_after.members, new_members);

    // version_proposal must NOT bump (only contract-wasm upgrades bump it)
    assert_eq!(info_after.version_proposal, 0);

    // proposal is closed
    let p = multisig.query_proposal(&proposal_id);
    assert_eq!(p.status, crate::storage::ProposalStatus::Closed);
}

#[test]
fn old_member_cannot_act_after_replacement() {
    let env = Env::default();
    env.mock_all_auths();

    let (m1, m2, m3, m4) = setup_quad(&env);
    let members = vec![&env, m1.clone(), m2.clone(), m3.clone(), m4.clone()];

    let multisig =
        initialize_multisig_contract(&env, s(&env, NAME), s(&env, DESC), members, Some(5_100u32));

    let new_a = Address::generate(&env);
    let new_b = Address::generate(&env);
    let new_c = Address::generate(&env);
    let new_members = vec![&env, new_a.clone(), new_b.clone(), new_c.clone()];

    multisig.create_member_update_proposal(
        &m1,
        &s(&env, "Rotate"),
        &s(&env, "..."),
        &new_members,
        &None,
    );
    let pid = multisig.query_last_proposal_id();
    multisig.sign_proposal(&m1, &pid);
    multisig.sign_proposal(&m2, &pid);
    multisig.sign_proposal(&m3, &pid);
    multisig.execute_proposal(&m1, &pid);

    // m1 (former member) cannot create a new proposal anymore
    let err = multisig.try_create_member_update_proposal(
        &m1,
        &s(&env, "Sneaky"),
        &s(&env, "..."),
        &vec![&env, m1.clone(), m2.clone(), m3.clone()],
        &None,
    );
    assert_eq!(err, Err(Ok(ContractError::UnauthorizedNotAMember)));

    // any of new members can
    multisig.create_member_update_proposal(
        &new_a,
        &s(&env, "Hello"),
        &s(&env, "..."),
        &vec![&env, new_a.clone(), new_b.clone()],
        &None,
    );
}

#[test]
fn member_update_quorum_not_reached_blocks_execution() {
    let env = Env::default();
    env.mock_all_auths();

    let (m1, m2, m3, m4) = setup_quad(&env);
    let members = vec![&env, m1.clone(), m2.clone(), m3.clone(), m4.clone()];

    let multisig = initialize_multisig_contract(
        &env,
        s(&env, NAME),
        s(&env, DESC),
        members,
        Some(5_100u32), // strictly-greater 51% over 4 members ⇒ need 3 sigs
    );

    let new_members = vec![&env, Address::generate(&env), Address::generate(&env)];

    multisig.create_member_update_proposal(&m1, &s(&env, "T"), &s(&env, "D"), &new_members, &None);
    let pid = multisig.query_last_proposal_id();

    // only 2/4 sign — 50% < 51%
    multisig.sign_proposal(&m1, &pid);
    multisig.sign_proposal(&m2, &pid);

    let err = multisig.try_execute_proposal(&m1, &pid);
    assert_eq!(err, Err(Ok(ContractError::QuorumNotReached)));

    // membership unchanged
    assert_eq!(multisig.query_multisig_members().len(), 4);
}

#[test]
fn non_member_cannot_create_member_update_proposal() {
    let env = Env::default();
    env.mock_all_auths();

    let (m1, m2, m3, m4) = setup_quad(&env);
    let intruder = Address::generate(&env);
    let members = vec![&env, m1, m2, m3, m4];

    let multisig =
        initialize_multisig_contract(&env, s(&env, NAME), s(&env, DESC), members, Some(5_100u32));

    let err = multisig.try_create_member_update_proposal(
        &intruder,
        &s(&env, "Takeover"),
        &s(&env, "..."),
        &vec![&env, intruder.clone()],
        &None,
    );
    assert_eq!(err, Err(Ok(ContractError::UnauthorizedNotAMember)));
}

#[test]
#[should_panic(expected = "Multisig: Initialize: cannot initialize multisig without any members!")]
fn empty_new_member_list_panics_at_creation() {
    let env = Env::default();
    env.mock_all_auths();

    let (m1, m2, m3, m4) = setup_quad(&env);
    let members = vec![&env, m1.clone(), m2, m3, m4];

    let multisig =
        initialize_multisig_contract(&env, s(&env, NAME), s(&env, DESC), members, Some(5_100u32));

    multisig.create_member_update_proposal(
        &m1,
        &s(&env, "Bad"),
        &s(&env, "..."),
        &vec![&env], // empty
        &None,
    );
}

#[test]
#[should_panic(
    expected = "Multisig: Initialize: Stellar's zero address provided as member. Aborting"
)]
fn zero_address_in_new_members_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let (m1, m2, m3, m4) = setup_quad(&env);
    let members = vec![&env, m1.clone(), m2, m3, m4];

    let multisig =
        initialize_multisig_contract(&env, s(&env, NAME), s(&env, DESC), members, Some(5_100u32));

    let zero = Address::from_string(&String::from_str(
        &env,
        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
    ));

    multisig.create_member_update_proposal(
        &m1,
        &s(&env, "Bad"),
        &s(&env, "..."),
        &vec![&env, zero],
        &None,
    );
}

#[test]
fn expired_member_update_proposal_cannot_be_executed() {
    use soroban_sdk::testutils::Ledger as _;

    let env = Env::default();
    env.mock_all_auths();

    // pin start time so the +SEVEN_DAYS default expiration is meaningful
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);

    let (m1, m2, m3, m4) = setup_quad(&env);
    let members = vec![&env, m1.clone(), m2.clone(), m3.clone(), m4.clone()];

    let multisig =
        initialize_multisig_contract(&env, s(&env, NAME), s(&env, DESC), members, Some(5_100u32));

    let new_members = vec![&env, Address::generate(&env), Address::generate(&env)];

    multisig.create_member_update_proposal(
        &m1,
        &s(&env, "Expire me"),
        &s(&env, "..."),
        &new_members,
        &None,
    );
    let pid = multisig.query_last_proposal_id();
    multisig.sign_proposal(&m1, &pid);
    multisig.sign_proposal(&m2, &pid);
    multisig.sign_proposal(&m3, &pid);

    // Jump far into the future, past any reasonable expiration
    env.ledger()
        .with_mut(|l| l.timestamp = 1_000_000 + 10_000_000_000);

    let err = multisig.try_execute_proposal(&m1, &pid);
    assert_eq!(err, Err(Ok(ContractError::ProposalExpired)));

    // membership unchanged
    assert_eq!(multisig.query_multisig_members().len(), 4);
}
