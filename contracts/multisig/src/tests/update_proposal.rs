use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

use super::setup::initialize_multisig_contract;

mod utils {
    use soroban_sdk::{BytesN, Env};

    #[allow(clippy::too_many_arguments)]
    mod multisig {
        soroban_sdk::contractimport!(
            file = "../../target/wasm32-unknown-unknown/release/soroban_multisig.wasm"
        );
    }

    pub fn multisig_wasm_hash(env: &Env) -> BytesN<32> {
        env.deployer().upload_contract_wasm(multisig::WASM)
    }
}

#[test]
fn update_proposal_works() {
    let env = Env::default();
    env.mock_all_auths();
    env.budget().reset_unlimited();

    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    let members = vec![&env, member1.clone(), member2.clone(), member3.clone()];

    let multisig = initialize_multisig_contract(
        &env,
        String::from_str(&env, "MultisigName"),
        String::from_str(&env, "Example description of this multisig"),
        members.clone(),
        Some(5_000u32),
    );

    // it's not actually a new wasm hash, but we need to use it to test the update proposal
    let new_wasm_hash = utils::multisig_wasm_hash(&env);
    multisig.create_update_proposal(&member1, &new_wasm_hash, &None);

    let proposal_id = multisig.query_last_proposal_id();
    multisig.sign_proposal(&member1, &proposal_id);
    multisig.sign_proposal(&member2, &proposal_id);
    multisig.sign_proposal(&member3, &proposal_id);

    let version_proposal = multisig.query_multisig_info().version_proposal;
    assert_eq!(version_proposal, 0);

    multisig.execute_proposal(&member1, &proposal_id);

    let version_proposal = multisig.query_multisig_info().version_proposal;
    assert_eq!(version_proposal, 1);
}

#[test]
#[should_panic(
    expected = "Multisig: Create update proposal: Sender is not a member of this multisig!"
)]
fn update_proposal_should_panic_when_sender_not_a_member() {
    let env = Env::default();
    env.mock_all_auths();
    env.budget().reset_unlimited();

    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    let not_a_member = Address::generate(&env);
    let members = vec![&env, member1, member2, member3];

    let multisig = initialize_multisig_contract(
        &env,
        String::from_str(&env, "MultisigName"),
        String::from_str(&env, "Example description of this multisig"),
        members.clone(),
        Some(5_000u32),
    );

    let new_wasm_hash = utils::multisig_wasm_hash(&env);
    multisig.create_update_proposal(&not_a_member, &new_wasm_hash, &None);
}

#[test]
#[should_panic(
    expected = "Multisig: Create Update proposal: Deadline cannot be less than an hour."
)]
fn create_update_proposal_should_fail_when_invalid_deadline() {
    let env = Env::default();
    env.mock_all_auths();
    env.budget().reset_unlimited();

    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    let members = vec![&env, member1.clone(), member2.clone(), member3.clone()];

    let multisig = initialize_multisig_contract(
        &env,
        String::from_str(&env, "MultisigName"),
        String::from_str(&env, "Example description of this multisig"),
        members.clone(),
        Some(5_000u32),
    );

    let new_wasm_hash = utils::multisig_wasm_hash(&env);
    // 1 second less than an hour
    multisig.create_update_proposal(&member1, &new_wasm_hash, &Some(3_599));
}
