extern crate std;

use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Ledger},
    vec, Address, Env, IntoVal, String, Symbol,
};

use crate::{
    error::ContractError,
    tests::setup::{DAY_AS_TIMESTAMP, TWO_WEEKS_EXPIRATION_DATE},
};

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
    assert_eq!(
        env.auths(),
        std::vec![(
            // Address for which authorization check is performed
            member1.clone(),
            // Invocation tree that needs to be authorized
            AuthorizedInvocation {
                // Function that is authorized. Can be a contract function or
                // a host function that requires authorization.
                function: AuthorizedFunction::Contract((
                    // Address of the called contract
                    multisig.address.clone(),
                    // Name of the called function
                    Symbol::new(&env, "create_update_proposal"),
                    // Arguments used to call `create_transaction_proposal`
                    (&member1.clone(), new_wasm_hash, None::<u64>).into_val(&env),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );

    let proposal_id = multisig.query_last_proposal_id();
    multisig.sign_proposal(&member1, &proposal_id);
    assert_eq!(
        env.auths(),
        std::vec![(
            // Address for which authorization check is performed
            member1.clone(),
            // Invocation tree that needs to be authorized
            AuthorizedInvocation {
                // Function that is authorized. Can be a contract function or
                // a host function that requires authorization.
                function: AuthorizedFunction::Contract((
                    // Address of the called contract
                    multisig.address.clone(),
                    // Name of the called function
                    Symbol::new(&env, "sign_proposal"),
                    // Arguments used to call `create_transaction_proposal`
                    (&member1.clone(), proposal_id).into_val(&env),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );

    multisig.sign_proposal(&member2, &proposal_id);
    assert_eq!(
        env.auths(),
        std::vec![(
            // Address for which authorization check is performed
            member2.clone(),
            // Invocation tree that needs to be authorized
            AuthorizedInvocation {
                // Function that is authorized. Can be a contract function or
                // a host function that requires authorization.
                function: AuthorizedFunction::Contract((
                    // Address of the called contract
                    multisig.address.clone(),
                    // Name of the called function
                    Symbol::new(&env, "sign_proposal"),
                    // Arguments used to call `create_transaction_proposal`
                    (&member2.clone(), proposal_id).into_val(&env),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );

    multisig.sign_proposal(&member3, &proposal_id);
    assert_eq!(
        env.auths(),
        std::vec![(
            // Address for which authorization check is performed
            member3.clone(),
            // Invocation tree that needs to be authorized
            AuthorizedInvocation {
                // Function that is authorized. Can be a contract function or
                // a host function that requires authorization.
                function: AuthorizedFunction::Contract((
                    // Address of the called contract
                    multisig.address.clone(),
                    // Name of the called function
                    Symbol::new(&env, "sign_proposal"),
                    // Arguments used to call `create_transaction_proposal`
                    (&member3.clone(), proposal_id).into_val(&env),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );

    let version_proposal = multisig.query_multisig_info().version_proposal;
    assert_eq!(version_proposal, 0);

    multisig.execute_proposal(&member1, &proposal_id);
    assert_eq!(
        env.auths(),
        std::vec![(
            // Address for which authorization check is performed
            member1.clone(),
            // Invocation tree that needs to be authorized
            AuthorizedInvocation {
                // Function that is authorized. Can be a contract function or
                // a host function that requires authorization.
                function: AuthorizedFunction::Contract((
                    // Address of the called contract
                    multisig.address.clone(),
                    // Name of the called function
                    Symbol::new(&env, "execute_proposal"),
                    // Arguments used to call `create_transaction_proposal`
                    (&member1.clone(), proposal_id).into_val(&env),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );

    let version_proposal = multisig.query_multisig_info().version_proposal;
    assert_eq!(version_proposal, 1);
}

#[test]
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
    assert_eq!(
        multisig.try_create_update_proposal(&not_a_member, &new_wasm_hash, &None),
        Err(Ok(ContractError::UnauthorizedNotAMember))
    );
}

#[test]
fn create_update_proposal_should_fail_when_invalid_expiration_date() {
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
    assert_eq!(
        multisig.try_create_update_proposal(&member1, &new_wasm_hash, &Some(3_599)),
        Err(Ok(ContractError::InvalidExpirationDate))
    );
}

#[test]
fn create_and_execute_update_proposal_with_expiration_date() {
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
    multisig.create_update_proposal(&member1, &new_wasm_hash, &Some(TWO_WEEKS_EXPIRATION_DATE));

    let proposal_id = multisig.query_last_proposal_id();

    env.ledger().with_mut(|li| li.timestamp = DAY_AS_TIMESTAMP);
    multisig.sign_proposal(&member1, &proposal_id);

    env.ledger()
        .with_mut(|li| li.timestamp = DAY_AS_TIMESTAMP * 2);
    multisig.sign_proposal(&member2, &proposal_id);

    env.ledger()
        .with_mut(|li| li.timestamp = DAY_AS_TIMESTAMP * 3);
    multisig.sign_proposal(&member3, &proposal_id);

    let version_proposal = multisig.query_multisig_info().version_proposal;
    assert_eq!(version_proposal, 0);

    // the day before the endtime we sign the update proposal
    env.ledger()
        .with_mut(|li| li.timestamp = TWO_WEEKS_EXPIRATION_DATE - DAY_AS_TIMESTAMP);
    multisig.execute_proposal(&member1, &proposal_id);

    let version_proposal = multisig.query_multisig_info().version_proposal;
    assert_eq!(version_proposal, 1);
}
