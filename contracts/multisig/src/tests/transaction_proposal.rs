use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

use super::setup::{deploy_token_contract, initialize_multisig_contract};
use crate::{
    storage::{Proposal, ProposalStatus, ProposalType, Transaction},
    SEVEN_DAYS_DEADLINE,
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
    assert_eq!(multisig.query_last_proposal_id(), 1);

    assert_eq!(
        multisig.query_proposal(&1).unwrap(),
        Proposal {
            id: 1,
            sender: member1.clone(),
            proposal: ProposalType::Transaction(Transaction {
                token: token.address.clone(),
                amount: 10_000,
                recipient: recipient.clone(),
                title: String::from_str(&env, "TxTitle#01"),
                description: String::from_str(&env, "TxTestDescription")
            }),
            status: ProposalStatus::Open,
            creation_timestamp: 0,
            expiration_timestamp: SEVEN_DAYS_DEADLINE
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

    assert_eq!(
        multisig.query_proposal(&1).unwrap().status,
        ProposalStatus::Closed
    );
}

#[test]
fn remove_proposal() {
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
        multisig.query_proposal(&1).unwrap(),
        Proposal {
            id: 1,
            sender: member1.clone(),
            proposal: ProposalType::Transaction(Transaction {
                token: token.address.clone(),
                amount: 10_000,
                recipient: recipient.clone(),
                title: String::from_str(&env, "TxTitle#01"),
                description: String::from_str(&env, "TxTestDescription")
            }),
            status: ProposalStatus::Open,
            creation_timestamp: 0,
            expiration_timestamp: SEVEN_DAYS_DEADLINE
        }
    );

    // now remove this proposal
    multisig.remove_proposal(&member1, &1);

    assert!(multisig.query_proposal(&1).is_none());
}

#[test]
#[should_panic = "Multisig: Create transaction proposal: Title longer than 64 characters!"]
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

    multisig.create_transaction_proposal(
        &member1,
        &String::from_bytes(&env, &[0u8; 65]),
        &String::from_str(&env, "TxTestDescription"),
        &Address::generate(&env),
        &10_000,
        &deploy_token_contract(&env, &member1).address,
        &None,
    );
}

#[test]
#[should_panic = "Multisig: Create transaction proposal: Description longer than 256 characters!"]
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

    multisig.create_transaction_proposal(
        &member1,
        &String::from_str(&env, "TxTitle#01"),
        &String::from_bytes(&env, &[0u8; 257]),
        &Address::generate(&env),
        &10_000,
        &deploy_token_contract(&env, &member1).address,
        &None,
    );
}

#[test]
#[should_panic(expected = "Multisig: Sign proposal: Proposal with this ID does not exist!")]
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
    multisig.sign_proposal(&member1, &2);
}

#[test]
#[should_panic(expected = "Multisig: Sign proposal: Proposal with this ID does not exist!")]
fn sign_removed_proposal() {
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

    // First member is able to vote for this proposal
    multisig.sign_proposal(&member1, &1);

    // then member1 removes the proposal
    multisig.remove_proposal(&member1, &1);

    // This proposal can not be signed anymore
    multisig.sign_proposal(&member2, &1);
}

#[test]
fn query_all_proposals_with_one_removed() {
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
    multisig.create_transaction_proposal(
        &member1,
        &String::from_str(&env, "TxTitle#02"),
        &String::from_str(&env, "TxTestDescription"),
        &recipient2,
        &15_000,
        &token.address,
        &None,
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
    assert_eq!(multisig.query_last_proposal_id(), 3);

    // we get rid of the 2nd proposal
    multisig.remove_proposal(&member1, &2);

    // getting all proposals now
    let all_proposals_vec = multisig.query_all_proposals();
    let proposal1 = multisig.query_proposal(&1).unwrap();
    let proposal3 = multisig.query_proposal(&3).unwrap();

    assert_eq!(all_proposals_vec, vec![&env, proposal1, proposal3]);
}

mod non_member {
    use super::*;

    #[test]
    #[should_panic = "Multisig: Create transaction proposal: Sender is not a member of this multisig!"]
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

        multisig.create_transaction_proposal(
            &random,
            &String::from_str(&env, "TxTitle#01"),
            &String::from_str(&env, "TxTestDescription"),
            &recipient,
            &10_000,
            &deploy_token_contract(&env, &member1).address,
            &None,
        );
    }

    #[test]
    #[should_panic = "Multisig: Sign proposal: Sender is not a member of this multisig!"]
    fn tries_to_vote() {
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

        multisig.sign_proposal(&random, &1);
    }

    #[test]
    #[should_panic = "Multisig: Execute proposal: Sender is not a member of this multisig!"]
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

        multisig.execute_proposal(&random, &1);
    }

    #[test]
    #[should_panic = "Multisig: Remove proposal: Sender is not a member of this multisig!"]
    fn tries_to_remove() {
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

        multisig.remove_proposal(&random, &1);
    }
}

mod closed_proposal {
    use super::*;

    #[test]
    #[should_panic = "Multisig: Sign proposal: Trying to sign a closed proposal!"]
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
        assert_eq!(
            multisig.query_proposal(&1).unwrap().status,
            ProposalStatus::Closed
        );
        multisig.sign_proposal(&member3, &1);
    }

    #[test]
    #[should_panic = "Multisig: Execute proposal: Trying to execute a closed proposal!"]
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
        assert_eq!(
            multisig.query_proposal(&1).unwrap().status,
            ProposalStatus::Closed
        );
        multisig.execute_proposal(&member1, &1);
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

        // One signature gives bigger ratio then threshold required
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

        multisig.execute_proposal(&member1, &1);

        assert_eq!(token.balance(&recipient), 10_000i128);
        assert_eq!(token.balance(&multisig.address), 0i128);

        assert_eq!(
            multisig.query_proposal(&1).unwrap().status,
            ProposalStatus::Closed
        );
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

        // One signature gives ratio equal to the quorum
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

        multisig.execute_proposal(&member1, &1);

        assert_eq!(token.balance(&recipient), 10_000i128);
        assert_eq!(token.balance(&multisig.address), 0i128);

        assert_eq!(
            multisig.query_proposal(&1).unwrap().status,
            ProposalStatus::Closed
        );
    }

    #[test]
    #[should_panic = "Multisig: Execute proposal: Required quorum has not been reached!"]
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

        // One signature is not enough and execution will fail
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

        multisig.execute_proposal(&member1, &1);
    }

    #[test]
    #[should_panic = "Multisig: Execute proposal: Required quorum has not been reached!"]
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

        multisig.execute_proposal(&member1, &1);
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

        assert_eq!(
            multisig.query_proposal(&1).unwrap().status,
            ProposalStatus::Closed
        );
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
        multisig.query_proposal(&1).unwrap(),
        Proposal {
            id: 1,
            sender: member1.clone(),
            proposal: ProposalType::Transaction(Transaction {
                token: token.address.clone(),
                amount: 10_000,
                recipient: recipient1.clone(),
                title: String::from_str(&env, "TxTitle#01"),
                description: String::from_str(&env, "TxTestDescription")
            }),
            status: ProposalStatus::Open,
            creation_timestamp: 0,
            expiration_timestamp: SEVEN_DAYS_DEADLINE
        }
    );
    assert_eq!(
        multisig.query_proposal(&2).unwrap(),
        Proposal {
            id: 2,
            sender: member3.clone(),
            proposal: ProposalType::Transaction(Transaction {
                token: token.address.clone(),
                amount: 15_000,
                recipient: recipient2.clone(),
                title: String::from_str(&env, "TxTitle#02"),
                description: String::from_str(&env, "TxTestDescription")
            }),
            status: ProposalStatus::Open,
            creation_timestamp: 0,
            expiration_timestamp: SEVEN_DAYS_DEADLINE
        }
    );
    assert_eq!(
        multisig.query_proposal(&3).unwrap(),
        Proposal {
            id: 3,
            sender: member2.clone(),
            proposal: ProposalType::Transaction(Transaction {
                token: token2.address.clone(),
                amount: 5_000,
                recipient: recipient1.clone(),
                title: String::from_str(&env, "TxTitle#03"),
                description: String::from_str(&env, "TxTestDescription")
            }),
            status: ProposalStatus::Open,
            creation_timestamp: 0,
            expiration_timestamp: SEVEN_DAYS_DEADLINE
        }
    );

    // get all proposals
    let all_proposals_vec = multisig.query_all_proposals();
    let proposal1 = multisig.query_proposal(&1).unwrap();
    let proposal2 = multisig.query_proposal(&2).unwrap();
    let proposal3 = multisig.query_proposal(&3).unwrap();

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
    assert_eq!(
        multisig.query_proposal(&3).unwrap().status,
        ProposalStatus::Closed
    );

    assert_eq!(token.balance(&recipient2), 0i128);
    assert_eq!(token.balance(&multisig.address), 25_000i128);
    multisig.execute_proposal(&member1, &2);
    assert_eq!(token.balance(&recipient2), 15_000i128);
    assert_eq!(token.balance(&multisig.address), 10_000i128);
    assert_eq!(
        multisig.query_proposal(&2).unwrap().status,
        ProposalStatus::Closed
    );

    assert_eq!(token.balance(&recipient1), 0i128);
    assert_eq!(token.balance(&multisig.address), 10_000i128);
    multisig.execute_proposal(&member1, &1);
    assert_eq!(token.balance(&recipient1), 10_000i128);
    assert_eq!(token.balance(&multisig.address), 0i128);
    assert_eq!(
        multisig.query_proposal(&1).unwrap().status,
        ProposalStatus::Closed
    );
}
