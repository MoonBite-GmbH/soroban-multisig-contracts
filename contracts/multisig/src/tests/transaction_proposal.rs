extern crate std;

use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Ledger},
    vec, Address, Env, IntoVal, String, Symbol,
};

use super::setup::{
    deploy_token_contract, initialize_multisig_contract, DAY_AS_TIMESTAMP,
    TWO_WEEKS_EXPIRATION_DATE,
};
use crate::{
    error::ContractError,
    storage::{Proposal, ProposalStatus, ProposalType, Transaction},
    SEVEN_DAYS_EXPIRATION_DATE,
};

#[test]
fn propose_transaction_proposal_full_quorum() {
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

    // create some token for the transaction
    let token = deploy_token_contract(&env, &member1);
    token.mint(&multisig.address, &10_000);

    let recipient = Address::generate(&env);

    multisig.create_transaction_proposal(
        &member1,
        &String::from_str(&env, "TxTitle#01"),
        &String::from_str(&env, "TxTestDescription"),
        &recipient,
        &10_000,
        &token.address,
        &None,
    );
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
                    Symbol::new(&env, "create_transaction_proposal"),
                    // Arguments used to call `create_transaction_proposal`
                    (
                        &member1.clone(),
                        "TxTitle#01",
                        "TxTestDescription",
                        &recipient,
                        10_000u64,
                        &token.address,
                        None::<u64>,
                    )
                        .into_val(&env),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );

    assert_eq!(multisig.query_last_proposal_id(), 1);

    assert_eq!(
        multisig.query_proposal(&1),
        Proposal {
            id: 1,
            sender: member1.clone(),
            title: String::from_str(&env, "TxTitle#01"),
            description: String::from_str(&env, "TxTestDescription"),
            proposal: ProposalType::Transaction(Transaction {
                token: token.address.clone(),
                amount: 10_000,
                recipient: recipient.clone(),
            }),
            status: ProposalStatus::Open,
            creation_timestamp: 0,
            expiration_timestamp: SEVEN_DAYS_EXPIRATION_DATE
        }
    );

    // at the beginning, there are no signatures
    assert_eq!(
        multisig.query_signatures(&1),
        vec![
            &env,
            (member1.clone(), false),
            (member2.clone(), false),
            (member3.clone(), false)
        ]
    );

    multisig.sign_proposal(&member1, &1);
    assert_eq!(
        multisig.query_signatures(&1),
        vec![
            &env,
            (member1.clone(), true),
            (member2.clone(), false),
            (member3.clone(), false)
        ]
    );

    multisig.sign_proposal(&member3, &1);
    assert_eq!(
        multisig.query_signatures(&1),
        vec![
            &env,
            (member1.clone(), true),
            (member2.clone(), false),
            (member3.clone(), true)
        ]
    );

    multisig.sign_proposal(&member2, &1);
    assert_eq!(
        multisig.query_signatures(&1),
        vec![
            &env,
            (member1.clone(), true),
            (member2.clone(), true),
            (member3.clone(), true)
        ]
    );

    // before executing the transaction, let's make sure there is no previous balance
    assert_eq!(token.balance(&recipient), 0i128);
    assert_eq!(token.balance(&multisig.address), 10_000i128);

    multisig.execute_proposal(&member1, &1);

    assert_eq!(token.balance(&recipient), 10_000i128);
    assert_eq!(token.balance(&multisig.address), 0i128);

    assert_eq!(multisig.query_proposal(&1).status, ProposalStatus::Closed);
}

#[test]
fn proposal_name_too_long() {
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

    assert_eq!(
        multisig.try_create_transaction_proposal(
            &member1,
            &String::from_bytes(&env, &[0u8; 65]),
            &String::from_str(&env, "TxTestDescription"),
            &Address::generate(&env),
            &10_000,
            &deploy_token_contract(&env, &member1).address,
            &None,
        ),
        Err(Ok(ContractError::TitleTooLong))
    );
}

#[test]
fn proposal_description_too_long() {
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

    assert_eq!(
        multisig.try_create_transaction_proposal(
            &member1,
            &String::from_str(&env, "TxTitle#01"),
            &String::from_bytes(&env, &[0u8; 258]),
            &Address::generate(&env),
            &10_000,
            &deploy_token_contract(&env, &member1).address,
            &None,
        ),
        Err(Ok(ContractError::DescriptionTooLong))
    );
}

#[test]
fn sign_invalid_proposal() {
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

    // create some token for the transaction
    let token = deploy_token_contract(&env, &member1);
    token.mint(&multisig.address, &10_000);

    let recipient = Address::generate(&env);

    multisig.create_transaction_proposal(
        &member1,
        &String::from_str(&env, "TxTitle#01"),
        &String::from_str(&env, "TxTestDescription"),
        &recipient,
        &10_000,
        &token.address,
        &None,
    );

    // only proposal with ID 1 exists now
    assert_eq!(
        multisig.try_sign_proposal(&member1, &2),
        Err(Ok(ContractError::ProposalNotFound))
    );
}

#[test]
fn query_all_proposals() {
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

    let token = deploy_token_contract(&env, &member1);
    token.mint(&multisig.address, &25_000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    multisig.create_transaction_proposal(
        &member1,
        &String::from_str(&env, "TxTitle#01"),
        &String::from_str(&env, "TxTestDescription"),
        &recipient1,
        &10_000,
        &token.address,
        &None,
    );

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
                    Symbol::new(&env, "create_transaction_proposal"),
                    // Arguments used to call `create_transaction_proposal`
                    (
                        &member1.clone(),
                        "TxTitle#01",
                        "TxTestDescription",
                        &recipient1,
                        10_000u64,
                        &token.address,
                        None::<u64>,
                    )
                        .into_val(&env),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );

    multisig.create_transaction_proposal(
        &member1,
        &String::from_str(&env, "TxTitle#02"),
        &String::from_str(&env, "TxTestDescription"),
        &recipient2,
        &15_000,
        &token.address,
        &None,
    );

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
                    Symbol::new(&env, "create_transaction_proposal"),
                    // Arguments used to call `create_transaction_proposal`
                    (
                        &member1.clone(),
                        "TxTitle#02",
                        "TxTestDescription",
                        &recipient2,
                        15_000u64,
                        &token.address,
                        None::<u64>,
                    )
                        .into_val(&env),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );

    multisig.create_transaction_proposal(
        &member1,
        &String::from_str(&env, "TxTitle#03"),
        &String::from_str(&env, "TxTestDescription"),
        &recipient1,
        &5_000,
        &token.address,
        &None,
    );

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
                    Symbol::new(&env, "create_transaction_proposal"),
                    // Arguments used to call `create_transaction_proposal`
                    (
                        &member1.clone(),
                        "TxTitle#03",
                        "TxTestDescription",
                        &recipient1,
                        5_000u64,
                        &token.address,
                        None::<u64>,
                    )
                        .into_val(&env),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );

    assert_eq!(multisig.query_last_proposal_id(), 3);

    // getting all proposals now
    let all_proposals_vec = multisig.query_all_proposals();
    let proposal1 = multisig.query_proposal(&1);
    let proposal2 = multisig.query_proposal(&2);
    let proposal3 = multisig.query_proposal(&3);

    assert_eq!(
        all_proposals_vec,
        vec![&env, proposal1, proposal2, proposal3]
    );
}

mod non_member {
    use super::*;

    #[test]
    fn tries_to_create_proposal() {
        let env = Env::default();
        env.mock_all_auths();

        let member1 = Address::generate(&env);
        let random = Address::generate(&env);
        let members = vec![&env, member1.clone()];

        let multisig = initialize_multisig_contract(
            &env,
            String::from_str(&env, "MultisigName"),
            String::from_str(&env, "Example description of this multisig"),
            members.clone(),
            None,
        );

        let recipient = Address::generate(&env);

        assert_eq!(
            multisig.try_create_transaction_proposal(
                &random,
                &String::from_str(&env, "TxTitle#01"),
                &String::from_str(&env, "TxTestDescription"),
                &recipient,
                &10_000,
                &deploy_token_contract(&env, &member1).address,
                &None
            ),
            Err(Ok(ContractError::UnauthorizedNotAMember))
        )
    }

    #[test]
    fn tries_to_vote() {
        let env = Env::default();
        env.mock_all_auths();

        let member1 = Address::generate(&env);
        let random = Address::generate(&env);
        let members = vec![&env, member1.clone()];
        let token_address = deploy_token_contract(&env, &member1).address;

        let multisig = initialize_multisig_contract(
            &env,
            String::from_str(&env, "MultisigName"),
            String::from_str(&env, "Example description of this multisig"),
            members.clone(),
            None,
        );

        let recipient = Address::generate(&env);

        multisig.create_transaction_proposal(
            &member1,
            &String::from_str(&env, "TxTitle#01"),
            &String::from_str(&env, "TxTestDescription"),
            &recipient,
            &10_000,
            &token_address,
            &None,
        );

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
                        Symbol::new(&env, "create_transaction_proposal"),
                        // Arguments used to call `create_transaction_proposal`
                        (
                            &member1.clone(),
                            "TxTitle#01",
                            "TxTestDescription",
                            &recipient,
                            10_000u64,
                            &token_address,
                            None::<u64>,
                        )
                            .into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        assert_eq!(
            multisig.try_sign_proposal(&random, &1),
            Err(Ok(ContractError::UnauthorizedNotAMember))
        );
    }

    #[test]
    fn tries_to_execute() {
        let env = Env::default();
        env.mock_all_auths();

        let member1 = Address::generate(&env);
        let random = Address::generate(&env);
        let members = vec![&env, member1.clone()];

        let multisig = initialize_multisig_contract(
            &env,
            String::from_str(&env, "MultisigName"),
            String::from_str(&env, "Example description of this multisig"),
            members.clone(),
            None,
        );

        let recipient = Address::generate(&env);

        multisig.create_transaction_proposal(
            &member1,
            &String::from_str(&env, "TxTitle#01"),
            &String::from_str(&env, "TxTestDescription"),
            &recipient,
            &10_000,
            &deploy_token_contract(&env, &member1).address,
            &None,
        );

        multisig.sign_proposal(&member1, &1);

        assert_eq!(
            multisig.try_execute_proposal(&random, &1),
            Err(Ok(ContractError::UnauthorizedNotAMember))
        );
    }
}

mod closed_proposal {
    use super::*;

    #[test]
    fn member_cant_vote() {
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
            5_000, // 50% quorum
        );

        // create some token for the transaction
        let token = deploy_token_contract(&env, &member1);
        token.mint(&multisig.address, &10_000);

        let recipient = Address::generate(&env);

        multisig.create_transaction_proposal(
            &member1,
            &String::from_str(&env, "TxTitle#01"),
            &String::from_str(&env, "TxTestDescription"),
            &recipient,
            &10_000,
            &token.address,
            &None,
        );

        // vote above quorum and execute it
        multisig.sign_proposal(&member1, &1);
        multisig.sign_proposal(&member2, &1);
        multisig.execute_proposal(&member1, &1);

        // now 3rd member can't vote closed proposal
        assert_eq!(multisig.query_proposal(&1).status, ProposalStatus::Closed);
        assert_eq!(
            multisig.try_sign_proposal(&member3, &1),
            Err(Ok(ContractError::ProposalClosed))
        );
    }

    #[test]
    fn member_cant_execute() {
        let env = Env::default();
        env.mock_all_auths();

        let member1 = Address::generate(&env);
        let member2 = Address::generate(&env);
        let members = vec![&env, member1.clone(), member2.clone()];

        let multisig = initialize_multisig_contract(
            &env,
            String::from_str(&env, "MultisigName"),
            String::from_str(&env, "Example description of this multisig"),
            members.clone(),
            5_000, // 50% quorum
        );

        // create some token for the transaction
        let token = deploy_token_contract(&env, &member1);
        token.mint(&multisig.address, &10_000);

        let recipient = Address::generate(&env);

        multisig.create_transaction_proposal(
            &member1,
            &String::from_str(&env, "TxTitle#01"),
            &String::from_str(&env, "TxTestDescription"),
            &recipient,
            &10_000,
            &token.address,
            &None,
        );

        // vote above quorum and execute it
        multisig.sign_proposal(&member1, &1);
        multisig.sign_proposal(&member2, &1);
        multisig.execute_proposal(&member1, &1);

        // now 1rd member can't vote closed proposal
        assert_eq!(multisig.query_proposal(&1).status, ProposalStatus::Closed);
        assert_eq!(
            multisig.try_execute_proposal(&member1, &1),
            Err(Ok(ContractError::ProposalClosed))
        );
    }
}

mod quorum {
    use super::*;

    #[test]
    fn lower_than_one_signature() {
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
            2_500, // 25%
        );

        // create some token for the transaction
        let token = deploy_token_contract(&env, &member1);
        token.mint(&multisig.address, &10_000);

        let recipient = Address::generate(&env);

        multisig.create_transaction_proposal(
            &member1,
            &String::from_str(&env, "TxTitle#01"),
            &String::from_str(&env, "TxTestDescription"),
            &recipient,
            &10_000,
            &token.address,
            &None,
        );
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
                        Symbol::new(&env, "create_transaction_proposal"),
                        // Arguments used to call `create_transaction_proposal`
                        (
                            &member1.clone(),
                            "TxTitle#01",
                            "TxTestDescription",
                            &recipient,
                            10_000u64,
                            &token.address,
                            None::<u64>
                        )
                            .into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        // One signature gives bigger ratio then threshold required
        multisig.sign_proposal(&member1, &1);
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
                        (&member1.clone(), 1u64).into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        assert_eq!(
            multisig.query_signatures(&1),
            vec![
                &env,
                (member1.clone(), true),
                (member2.clone(), false),
                (member3.clone(), false)
            ]
        );

        multisig.execute_proposal(&member1, &1);
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
                        (&member1.clone(), 1u64).into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        assert_eq!(token.balance(&recipient), 10_000i128);
        assert_eq!(token.balance(&multisig.address), 0i128);

        assert_eq!(multisig.query_proposal(&1).status, ProposalStatus::Closed);
    }

    #[test]
    fn equal_to_one_signature() {
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
            3_300, // 33%
        );

        // create some token for the transaction
        let token = deploy_token_contract(&env, &member1);
        token.mint(&multisig.address, &10_000);

        let recipient = Address::generate(&env);

        multisig.create_transaction_proposal(
            &member1,
            &String::from_str(&env, "TxTitle#01"),
            &String::from_str(&env, "TxTestDescription"),
            &recipient,
            &10_000,
            &token.address,
            &None,
        );
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
                        Symbol::new(&env, "create_transaction_proposal"),
                        // Arguments used to call `create_transaction_proposal`
                        (
                            &member1.clone(),
                            "TxTitle#01",
                            "TxTestDescription",
                            &recipient,
                            10_000u64,
                            &token.address,
                            None::<u64>
                        )
                            .into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        // One signature gives ratio equal to the quorum
        multisig.sign_proposal(&member1, &1);

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
                        (&member1.clone(), 1u64,).into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );
        assert_eq!(
            multisig.query_signatures(&1),
            vec![
                &env,
                (member1.clone(), true),
                (member2.clone(), false),
                (member3.clone(), false)
            ]
        );

        multisig.execute_proposal(&member1, &1);
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
                        (&member1.clone(), 1u64,).into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        assert_eq!(token.balance(&recipient), 10_000i128);
        assert_eq!(token.balance(&multisig.address), 0i128);

        assert_eq!(multisig.query_proposal(&1).status, ProposalStatus::Closed);
    }

    #[test]
    fn higher_than_one_signature_fails_to_execute() {
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
            3_400, // 34%
        );

        // create some token for the transaction
        let token = deploy_token_contract(&env, &member1);
        token.mint(&multisig.address, &10_000);

        let recipient = Address::generate(&env);

        multisig.create_transaction_proposal(
            &member1,
            &String::from_str(&env, "TxTitle#01"),
            &String::from_str(&env, "TxTestDescription"),
            &recipient,
            &10_000,
            &token.address,
            &None,
        );
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
                        Symbol::new(&env, "create_transaction_proposal"),
                        // Arguments used to call `create_transaction_proposal`
                        (
                            &member1.clone(),
                            "TxTitle#01",
                            "TxTestDescription",
                            &recipient,
                            10_000u64,
                            &token.address,
                            None::<u64>
                        )
                            .into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        // One signature is not enough and execution will fail
        multisig.sign_proposal(&member1, &1);
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
                        (&member1.clone(), 1u64,).into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );
        assert_eq!(
            multisig.query_signatures(&1),
            vec![
                &env,
                (member1.clone(), true),
                (member2.clone(), false),
                (member3.clone(), false)
            ]
        );

        assert_eq!(
            multisig.try_execute_proposal(&member1, &1),
            Err(Ok(ContractError::QuorumNotReached))
        );
    }

    #[test]
    fn nine_out_of_ten_signatures_full_quorum() {
        let env = Env::default();
        env.mock_all_auths();

        let member1 = Address::generate(&env);
        let member2 = Address::generate(&env);
        let member3 = Address::generate(&env);
        let member4 = Address::generate(&env);
        let member5 = Address::generate(&env);
        let member6 = Address::generate(&env);
        let member7 = Address::generate(&env);
        let member8 = Address::generate(&env);
        let member9 = Address::generate(&env);
        let member0 = Address::generate(&env);
        let members = vec![
            &env,
            member1.clone(),
            member2.clone(),
            member3.clone(),
            member4.clone(),
            member5.clone(),
            member6.clone(),
            member7.clone(),
            member8.clone(),
            member9.clone(),
            member0.clone(),
        ];

        let multisig = initialize_multisig_contract(
            &env,
            String::from_str(&env, "MultisigName"),
            String::from_str(&env, "Example description of this multisig"),
            members.clone(),
            None, // 100% quorum
        );

        // create some token for the transaction
        let token = deploy_token_contract(&env, &member1);
        token.mint(&multisig.address, &10_000);

        let recipient = Address::generate(&env);

        multisig.create_transaction_proposal(
            &member1,
            &String::from_str(&env, "TxTitle#01"),
            &String::from_str(&env, "TxTestDescription"),
            &recipient,
            &10_000,
            &token.address,
            &None,
        );
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
                        Symbol::new(&env, "create_transaction_proposal"),
                        // Arguments used to call `create_transaction_proposal`
                        (
                            &member1.clone(),
                            "TxTitle#01",
                            "TxTestDescription",
                            &recipient,
                            10_000u64,
                            &token.address,
                            None::<u64>
                        )
                            .into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        // One signature is not enough and execution will fail
        multisig.sign_proposal(&member1, &1);
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
                        (&member1.clone(), 1u64).into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        multisig.sign_proposal(&member2, &1);
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
                        (&member2.clone(), 1u64).into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        multisig.sign_proposal(&member3, &1);
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
                        (&member3.clone(), 1u64).into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        multisig.sign_proposal(&member4, &1);
        assert_eq!(
            env.auths(),
            std::vec![(
                // Address for which authorization check is performed
                member4.clone(),
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
                        (&member4.clone(), 1u64).into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        multisig.sign_proposal(&member5, &1);
        assert_eq!(
            env.auths(),
            std::vec![(
                // Address for which authorization check is performed
                member5.clone(),
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
                        (&member5.clone(), 1u64).into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        multisig.sign_proposal(&member6, &1);
        assert_eq!(
            env.auths(),
            std::vec![(
                // Address for which authorization check is performed
                member6.clone(),
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
                        (&member6.clone(), 1u64).into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        multisig.sign_proposal(&member7, &1);
        assert_eq!(
            env.auths(),
            std::vec![(
                // Address for which authorization check is performed
                member7.clone(),
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
                        (&member7.clone(), 1u64).into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        multisig.sign_proposal(&member8, &1);
        assert_eq!(
            env.auths(),
            std::vec![(
                // Address for which authorization check is performed
                member8.clone(),
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
                        (&member8.clone(), 1u64).into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        multisig.sign_proposal(&member0, &1);
        assert_eq!(
            env.auths(),
            std::vec![(
                // Address for which authorization check is performed
                member0.clone(),
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
                        (&member0.clone(), 1u64).into_val(&env),
                    )),
                    sub_invocations: std::vec![]
                }
            )]
        );

        assert_eq!(
            multisig.query_signatures(&1),
            vec![
                &env,
                (member1.clone(), true),
                (member2.clone(), true),
                (member3.clone(), true),
                (member4.clone(), true),
                (member5.clone(), true),
                (member6.clone(), true),
                (member7.clone(), true),
                (member8.clone(), true),
                (member9.clone(), false),
                (member0.clone(), true),
            ]
        );

        assert_eq!(
            multisig.try_execute_proposal(&member1, &1),
            Err(Ok(ContractError::QuorumNotReached))
        );
    }

    #[test]
    fn nine_out_of_ten_signatures_90_quorum() {
        let env = Env::default();
        env.mock_all_auths();

        let member1 = Address::generate(&env);
        let member2 = Address::generate(&env);
        let member3 = Address::generate(&env);
        let member4 = Address::generate(&env);
        let member5 = Address::generate(&env);
        let member6 = Address::generate(&env);
        let member7 = Address::generate(&env);
        let member8 = Address::generate(&env);
        let member9 = Address::generate(&env);
        let member0 = Address::generate(&env);
        let members = vec![
            &env,
            member1.clone(),
            member2.clone(),
            member3.clone(),
            member4.clone(),
            member5.clone(),
            member6.clone(),
            member7.clone(),
            member8.clone(),
            member9.clone(),
            member0.clone(),
        ];

        let multisig = initialize_multisig_contract(
            &env,
            String::from_str(&env, "MultisigName"),
            String::from_str(&env, "Example description of this multisig"),
            members.clone(),
            9_000, // 90% quorum
        );

        // create some token for the transaction
        let token = deploy_token_contract(&env, &member1);
        token.mint(&multisig.address, &10_000);

        let recipient = Address::generate(&env);

        multisig.create_transaction_proposal(
            &member1,
            &String::from_str(&env, "TxTitle#01"),
            &String::from_str(&env, "TxTestDescription"),
            &recipient,
            &10_000,
            &token.address,
            &None,
        );

        // One signature is not enough and execution will fail
        multisig.sign_proposal(&member1, &1);
        multisig.sign_proposal(&member2, &1);
        multisig.sign_proposal(&member3, &1);
        multisig.sign_proposal(&member4, &1);
        multisig.sign_proposal(&member5, &1);
        multisig.sign_proposal(&member6, &1);
        multisig.sign_proposal(&member7, &1);
        multisig.sign_proposal(&member8, &1);
        multisig.sign_proposal(&member0, &1);
        assert_eq!(
            multisig.query_signatures(&1),
            vec![
                &env,
                (member1.clone(), true),
                (member2.clone(), true),
                (member3.clone(), true),
                (member4.clone(), true),
                (member5.clone(), true),
                (member6.clone(), true),
                (member7.clone(), true),
                (member8.clone(), true),
                (member9.clone(), false),
                (member0.clone(), true),
            ]
        );

        // before executing the transaction, let's make sure there is no previous balance
        assert_eq!(token.balance(&recipient), 0i128);
        assert_eq!(token.balance(&multisig.address), 10_000i128);

        multisig.execute_proposal(&member1, &1);

        assert_eq!(token.balance(&recipient), 10_000i128);
        assert_eq!(token.balance(&multisig.address), 0i128);

        assert_eq!(multisig.query_proposal(&1).status, ProposalStatus::Closed);
    }
}

#[test]
fn multiple_active_proposals() {
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

    // create some tokens for the transaction
    let token = deploy_token_contract(&env, &member1);
    token.mint(&multisig.address, &25_000);
    let token2 = deploy_token_contract(&env, &member1);
    token2.mint(&multisig.address, &5_000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    multisig.create_transaction_proposal(
        &member1,
        &String::from_str(&env, "TxTitle#01"),
        &String::from_str(&env, "TxTestDescription"),
        &recipient1,
        &10_000,
        &token.address,
        &None,
    );
    multisig.create_transaction_proposal(
        &member3,
        &String::from_str(&env, "TxTitle#02"),
        &String::from_str(&env, "TxTestDescription"),
        &recipient2,
        &15_000,
        &token.address,
        &None,
    );
    multisig.create_transaction_proposal(
        &member2,
        &String::from_str(&env, "TxTitle#03"),
        &String::from_str(&env, "TxTestDescription"),
        &recipient1,
        &5_000,
        &token2.address,
        &None,
    );
    assert_eq!(multisig.query_last_proposal_id(), 3);

    assert_eq!(
        multisig.query_proposal(&1),
        Proposal {
            id: 1,
            sender: member1.clone(),
            title: String::from_str(&env, "TxTitle#01"),
            description: String::from_str(&env, "TxTestDescription"),
            proposal: ProposalType::Transaction(Transaction {
                token: token.address.clone(),
                amount: 10_000,
                recipient: recipient1.clone(),
            }),
            status: ProposalStatus::Open,
            creation_timestamp: 0,
            expiration_timestamp: SEVEN_DAYS_EXPIRATION_DATE
        }
    );
    assert_eq!(
        multisig.query_proposal(&2),
        Proposal {
            id: 2,
            sender: member3.clone(),
            title: String::from_str(&env, "TxTitle#02"),
            description: String::from_str(&env, "TxTestDescription"),
            proposal: ProposalType::Transaction(Transaction {
                token: token.address.clone(),
                amount: 15_000,
                recipient: recipient2.clone(),
            }),
            status: ProposalStatus::Open,
            creation_timestamp: 0,
            expiration_timestamp: SEVEN_DAYS_EXPIRATION_DATE
        }
    );
    assert_eq!(
        multisig.query_proposal(&3),
        Proposal {
            id: 3,
            sender: member2.clone(),
            title: String::from_str(&env, "TxTitle#03"),
            description: String::from_str(&env, "TxTestDescription"),
            proposal: ProposalType::Transaction(Transaction {
                token: token2.address.clone(),
                amount: 5_000,
                recipient: recipient1.clone(),
            }),
            status: ProposalStatus::Open,
            creation_timestamp: 0,
            expiration_timestamp: SEVEN_DAYS_EXPIRATION_DATE
        }
    );

    // get all proposals
    let all_proposals_vec = multisig.query_all_proposals();
    let proposal1 = multisig.query_proposal(&1);
    let proposal2 = multisig.query_proposal(&2);
    let proposal3 = multisig.query_proposal(&3);

    assert_eq!(
        all_proposals_vec,
        vec![&env, proposal1, proposal2, proposal3]
    );

    // some members sign some of the transactions
    multisig.sign_proposal(&member1, &1);
    multisig.sign_proposal(&member1, &2);
    multisig.sign_proposal(&member1, &3);

    multisig.sign_proposal(&member2, &1);
    multisig.sign_proposal(&member2, &3);

    multisig.sign_proposal(&member3, &2);
    multisig.sign_proposal(&member3, &3);
    assert_eq!(
        multisig.query_signatures(&1),
        vec![
            &env,
            (member1.clone(), true),
            (member2.clone(), true),
            (member3.clone(), false)
        ]
    );

    assert_eq!(
        multisig.query_signatures(&2),
        vec![
            &env,
            (member1.clone(), true),
            (member2.clone(), false),
            (member3.clone(), true)
        ]
    );

    assert_eq!(
        multisig.query_signatures(&3),
        vec![
            &env,
            (member1.clone(), true),
            (member2.clone(), true),
            (member3.clone(), true)
        ]
    );

    // now sign the rest of the transactions for the full quorum
    multisig.sign_proposal(&member2, &2);
    multisig.sign_proposal(&member3, &1);

    // execute transactions
    assert_eq!(token2.balance(&recipient1), 0i128);
    assert_eq!(token2.balance(&multisig.address), 5_000i128);
    multisig.execute_proposal(&member1, &3);
    assert_eq!(token2.balance(&recipient1), 5_000i128);
    assert_eq!(token2.balance(&multisig.address), 0i128);
    assert_eq!(multisig.query_proposal(&3).status, ProposalStatus::Closed);

    assert_eq!(token.balance(&recipient2), 0i128);
    assert_eq!(token.balance(&multisig.address), 25_000i128);
    multisig.execute_proposal(&member1, &2);
    assert_eq!(token.balance(&recipient2), 15_000i128);
    assert_eq!(token.balance(&multisig.address), 10_000i128);
    assert_eq!(multisig.query_proposal(&2).status, ProposalStatus::Closed);

    assert_eq!(token.balance(&recipient1), 0i128);
    assert_eq!(token.balance(&multisig.address), 10_000i128);
    multisig.execute_proposal(&member1, &1);
    assert_eq!(token.balance(&recipient1), 10_000i128);
    assert_eq!(token.balance(&multisig.address), 0i128);
    assert_eq!(multisig.query_proposal(&1).status, ProposalStatus::Closed);
}

#[test]
fn execute_proposal_should_fail_when_after_expiration_date() {
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

    let token = deploy_token_contract(&env, &member1);
    token.mint(&multisig.address, &10_000);

    let recipient = Address::generate(&env);

    multisig.create_transaction_proposal(
        &member1,
        &String::from_str(&env, "TxTitle#01"),
        &String::from_str(&env, "TxTestDescription"),
        &recipient,
        &10_000,
        &token.address,
        &Some(DAY_AS_TIMESTAMP),
    );

    multisig.sign_proposal(&member1, &1);
    multisig.sign_proposal(&member3, &1);
    multisig.sign_proposal(&member2, &1);

    env.ledger()
        .with_mut(|li| li.timestamp = TWO_WEEKS_EXPIRATION_DATE);

    assert_eq!(
        multisig.try_execute_proposal(&member1, &1),
        Err(Ok(ContractError::ProposalExpired))
    );
}

#[test]
fn create_transaction_proposal_should_fail_with_invalid_expiration_date() {
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

    // create some token for the transaction
    let token = deploy_token_contract(&env, &member1);
    token.mint(&multisig.address, &10_000);

    let recipient = Address::generate(&env);

    assert_eq!(
        multisig.try_create_transaction_proposal(
            &member1,
            &String::from_str(&env, "TxTitle#01"),
            &String::from_str(&env, "TxTestDescription"),
            &recipient,
            &10_000,
            &token.address,
            // minimum expiration date is an hour after creation, we set one that is 1 second shorter than that.
            &Some(3_599),
        ),
        Err(Ok(ContractError::InvalidExpirationDate))
    );
}

#[test]
fn create_and_execute_transaction_proposal_within_deadline() {
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

    // create some token for the transaction
    let token = deploy_token_contract(&env, &member1);
    token.mint(&multisig.address, &10_000);

    let recipient = Address::generate(&env);

    multisig.create_transaction_proposal(
        &member1,
        &String::from_str(&env, "TxTitle#01"),
        &String::from_str(&env, "TxTestDescription"),
        &recipient,
        &10_000,
        &token.address,
        // tx proposal with 10 days validity from the date of creation
        &Some(TWO_WEEKS_EXPIRATION_DATE - 4 * DAY_AS_TIMESTAMP),
    );
    assert_eq!(multisig.query_last_proposal_id(), 1);

    assert_eq!(
        multisig.query_proposal(&1),
        Proposal {
            id: 1,
            sender: member1.clone(),
            title: String::from_str(&env, "TxTitle#01"),
            description: String::from_str(&env, "TxTestDescription"),
            proposal: ProposalType::Transaction(Transaction {
                token: token.address.clone(),
                amount: 10_000,
                recipient: recipient.clone(),
            }),
            status: ProposalStatus::Open,
            creation_timestamp: 0,
            expiration_timestamp: TWO_WEEKS_EXPIRATION_DATE - 4 * DAY_AS_TIMESTAMP,
        }
    );

    // each member signs a few days after the proposal has been created
    env.ledger().with_mut(|li| li.timestamp = DAY_AS_TIMESTAMP);
    multisig.sign_proposal(&member1, &1);

    env.ledger()
        .with_mut(|li| li.timestamp = DAY_AS_TIMESTAMP * 2);
    multisig.sign_proposal(&member3, &1);

    env.ledger()
        .with_mut(|li| li.timestamp = DAY_AS_TIMESTAMP * 3);
    multisig.sign_proposal(&member2, &1);

    // before executing the transaction, let's make sure there is no previous balance
    assert_eq!(token.balance(&recipient), 0i128);
    assert_eq!(token.balance(&multisig.address), 10_000i128);

    // proposal is execute within the 1st week, 3 days before expiration date
    env.ledger()
        .with_mut(|li| li.timestamp = TWO_WEEKS_EXPIRATION_DATE / 2);

    multisig.execute_proposal(&member1, &1);

    assert_eq!(token.balance(&recipient), 10_000i128);
    assert_eq!(token.balance(&multisig.address), 0i128);

    assert_eq!(multisig.query_proposal(&1).status, ProposalStatus::Closed);
}
