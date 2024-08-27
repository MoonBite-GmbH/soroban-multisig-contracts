use soroban_sdk::{
    contract, contractimpl, contractmeta, log, panic_with_error, vec, Address, BytesN, Env, String,
    Vec,
};

use crate::{
    error::ContractError,
    storage::{
        get_last_proposal_id, get_multisig_members, get_name, get_proposal,
        get_proposal_signatures, get_quorum_bps, get_version, increase_version,
        increment_last_proposal_id, is_initialized, save_new_multisig, save_proposal,
        save_proposal_signature, save_quorum_bps, save_version, set_initialized, set_name,
        MultisigInfo, Proposal, ProposalStatus, ProposalType, Transaction,
    },
    token_contract, ONE_HOUR, SEVEN_DAYS_EXPIRATION_DATE, SOROBAN_ZERO_ADDRESS,
};
use soroban_decimal::Decimal;

// Metadata that is added on to the WASM custom section
contractmeta!(key = "Description", val = "Soroban Multisig Contract");

#[contract]
pub struct Multisig;

#[contractimpl]
impl Multisig {
    /// Initialize the contract
    /// members is a vector of addresses that this multisig will consist of
    /// quorum_bps requires to pass the minimum amount of required signers in BPS
    /// if not present, default if 100%
    #[allow(dead_code)]
    pub fn initialize(
        env: Env,
        name: String,
        description: String,
        members: Vec<Address>,
        quorum_bps: Option<u32>,
    ) -> Result<(), ContractError> {
        verify_members(&env, &members);

        if is_initialized(&env) {
            log!(
                &env,
                "Multisig: Initialize: initializing contract twice is not allowed"
            );
            return Err(ContractError::AlreadyInitialized);
        }
        set_initialized(&env);

        // Set a multisig with members passed in the argument
        save_new_multisig(&env, &members);

        // check if title and description aren't too long
        if name.len() > 64 {
            log!(
                &env,
                "Multisig: Initialize: Name longer than 64 characters!"
            );
            return Err(ContractError::TitleTooLong);
        }
        if description.len() > 256 {
            log!(
                &env,
                "Multisig: Initialize: Description longer than 256 characters!"
            );
            return Err(ContractError::DescriptionTooLong);
        }
        set_name(&env, name.clone(), description.clone());

        let quorum_bps = quorum_bps.unwrap_or(10_000);
        if quorum_bps <= 100 {
            log!(
                &env,
                "Multisig: Initialize: Quorum BPS amount set to 100 or lower"
            );
            return Err(ContractError::InitializeTooLowQuorum);
        } else if quorum_bps > 10_000 {
            log!(
                &env,
                "Multisig: Initialize: Quorum BPS amount set to more than 100%!"
            );
            return Err(ContractError::InitializeTooHighQuorum);
        } else {
            save_quorum_bps(&env, quorum_bps);
        }

        save_version(&env, &0);

        env.events().publish(("Multisig", "Initialize name"), name);
        env.events()
            .publish(("Multisig", "Initialize description"), description);

        Ok(())
    }

    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn create_transaction_proposal(
        env: Env,
        sender: Address,
        title: String,
        description: String,
        recipient: Address,
        amount: u64,
        token: Address,
        expiration_date: Option<u64>,
    ) -> Result<(), ContractError> {
        sender.require_auth();

        let multisig = get_multisig_members(&env);

        // check if sender is a member of this multisig
        if multisig.get(sender.clone()).is_none() {
            log!(
                &env,
                "Multisig: Create transaction proposal: Sender is not a member of this multisig!"
            );
            return Err(ContractError::UnauthorizedNotAMember);
        }

        // check if title and description aren't too long
        if title.len() > 64 {
            log!(
                &env,
                "Multisig: Create transaction proposal: Title longer than 64 characters!"
            );
            return Err(ContractError::TitleTooLong);
        }
        if description.len() > 256 {
            log!(
                &env,
                "Multisig: Create transaction proposal: Description longer than 256 characters!"
            );
            return Err(ContractError::DescriptionTooLong);
        }

        // loads the previous id, returns it and increments before saving
        let proposal_id = increment_last_proposal_id(&env);
        let transaction = Transaction {
            token,
            amount,
            recipient,
            title: title.clone(),
            description,
        };

        let creation_timestamp = env.ledger().timestamp();
        let expiration_timestamp = creation_timestamp
            + expiration_date.unwrap_or(creation_timestamp + SEVEN_DAYS_EXPIRATION_DATE);
        if expiration_timestamp < creation_timestamp + ONE_HOUR {
            log!(
                &env,
                "Multisig: Create Transaction proposal: Expiration date cannot be less than an hour."
            );
            return Err(ContractError::InvalidExpirationDate);
        }

        let proposal = Proposal {
            id: proposal_id,
            sender: sender.clone(),
            proposal: ProposalType::Transaction(transaction),
            status: ProposalStatus::Open,
            creation_timestamp,
            expiration_timestamp,
        };

        save_proposal(&env, &proposal);

        env.events()
            .publish(("Multisig", "Create proposal Title"), title);
        env.events()
            .publish(("Multisig", "Create proposal Sender"), sender);

        Ok(())
    }

    #[allow(dead_code)]
    pub fn create_update_proposal(
        env: Env,
        sender: Address,
        new_wasm_hash: BytesN<32>,
        expiration_date: Option<u64>,
    ) -> Result<(), ContractError> {
        sender.require_auth();

        let multisig = get_multisig_members(&env);

        if multisig.get(sender.clone()).is_none() {
            log!(
                &env,
                "Multisig: Create update proposal: Sender is not a member of this multisig!"
            );
            return Err(ContractError::UnauthorizedNotAMember);
        }

        let proposal_id = increment_last_proposal_id(&env);
        let creation_timestamp = env.ledger().timestamp();
        let expiration_timestamp = creation_timestamp
            + expiration_date.unwrap_or(creation_timestamp + SEVEN_DAYS_EXPIRATION_DATE);

        if expiration_timestamp < creation_timestamp + ONE_HOUR {
            log!(
                &env,
                "Multisig: Create Update proposal: Expiration date cannot be less than an hour."
            );
            panic_with_error!(&env, ContractError::InvalidExpirationDate);
        }

        let proposal = Proposal {
            id: proposal_id,
            sender: sender.clone(),
            proposal: ProposalType::UpdateContract(new_wasm_hash),
            status: ProposalStatus::Open,
            creation_timestamp,
            expiration_timestamp,
        };
        save_proposal(&env, &proposal);

        env.events()
            .publish(("Multisig", "Create proposal id"), proposal_id);
        env.events()
            .publish(("Multisig", "Create proposal sender"), sender);

        Ok(())
    }

    #[allow(dead_code)]
    pub fn sign_proposal(env: Env, sender: Address, proposal_id: u64) -> Result<(), ContractError> {
        sender.require_auth();

        let multisig = get_multisig_members(&env);

        let proposal = match get_proposal(&env, proposal_id) {
            Some(proposal) => proposal,
            None => {
                log!(
                    &env,
                    "Multisig: Sign proposal: Proposal with this ID does not exist!"
                );
                return Err(ContractError::ProposalNotFound);
            }
        };

        // check if sender is a member of this multisig
        if multisig.get(sender.clone()).is_none() {
            log!(
                &env,
                "Multisig: Sign proposal: Sender is not a member of this multisig!"
            );
            return Err(ContractError::UnauthorizedNotAMember);
        }

        if proposal.status != ProposalStatus::Open {
            log!(
                &env,
                "Multisig: Sign proposal: Trying to sign a closed proposal!"
            );
            return Err(ContractError::ProposalClosed);
        }

        save_proposal_signature(&env, proposal_id, sender.clone());

        env.events()
            .publish(("Multisig", "Sign proposal ID: "), proposal_id);
        env.events()
            .publish(("Multisig", "Sign proposal sender"), sender);

        Ok(())
    }

    #[allow(dead_code)]
    pub fn execute_proposal(
        env: Env,
        sender: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        sender.require_auth();

        let mut proposal = match get_proposal(&env, proposal_id) {
            Some(proposal) => proposal,
            None => {
                log!(
                    &env,
                    "Multisig: Sign proposal: Proposal with this ID does not exist!"
                );
                return Err(ContractError::ProposalNotFound);
            }
        };

        let multisig = get_multisig_members(&env);

        // check if sender is a member of this multisig
        if multisig.get(sender.clone()).is_none() {
            log!(
                &env,
                "Multisig: Execute proposal: Sender is not a member of this multisig!"
            );
            return Err(ContractError::UnauthorizedNotAMember);
        }

        // to prevent a double execution
        if proposal.status != ProposalStatus::Open {
            log!(
                &env,
                "Multisig: Execute proposal: Trying to execute a closed proposal!"
            );
            return Err(ContractError::ProposalClosed);
        }

        let curr_timestamp = env.ledger().timestamp();
        if curr_timestamp > proposal.expiration_timestamp {
            log!(
                &env,
                "Multisig: Execute proposal: Trying to execute an expired proposal!"
            );
            proposal.status = ProposalStatus::Closed;
            save_proposal(&env, &proposal);

            return Err(ContractError::ProposalExpired);
        }

        // collect all addresses that signed this proposal
        let proposal_signatures = get_proposal_signatures(&env, proposal_id);
        let multisig = multisig.keys();
        let multisig_len = multisig.len();

        let mut signed = 0i8;
        for member in multisig {
            if proposal_signatures.get(member).is_some() {
                signed += 1;
            }
        }

        // get required quorum and compare it with ratio of vote confirmations vs multisig len
        let required_quorum = Decimal::bps(get_quorum_bps(&env) as i64);
        let voted_ratio = Decimal::from_ratio(signed, multisig_len);
        if voted_ratio < required_quorum {
            log!(
                &env,
                "Multisig: Execute proposal: Required quorum has not been reached!"
            );
            return Err(ContractError::QuorumNotReached);
        }

        // execute actual proposal
        match proposal.proposal.clone() {
            // Transaction proposal - transfer tokens to the recipient
            ProposalType::Transaction(t) => {
                token_contract::Client::new(&env, &t.token).transfer(
                    &env.current_contract_address(),
                    &t.recipient,
                    &(t.amount as i128),
                );
            }
            ProposalType::UpdateContract(new_wasm_hash) => {
                env.deployer().update_current_contract_wasm(new_wasm_hash);
                increase_version(&env);
            }
        }

        // after proposal is executed, mark it as closed
        proposal.status = ProposalStatus::Closed;
        save_proposal(&env, &proposal);

        env.events()
            .publish(("Multisig", "Execute proposal ID: "), proposal_id);
        env.events()
            .publish(("Multisig", "Execute proposal sender"), sender);

        Ok(())
    }

    // ----------- QUERY

    #[allow(dead_code)]
    pub fn query_multisig_info(env: Env) -> Result<MultisigInfo, ContractError> {
        let (name, description) = get_name(&env);
        Ok(MultisigInfo {
            name: name.clone(),
            description,
            members: get_multisig_members(&env).keys(),
            quorum_bps: get_quorum_bps(&env),
            version_proposal: get_version(&env),
        })
    }

    #[allow(dead_code)]
    pub fn query_multisig_members(env: Env) -> Result<Vec<Address>, ContractError> {
        let multisig_members = get_multisig_members(&env).keys();
        Ok(multisig_members)
    }

    #[allow(dead_code)]
    pub fn query_proposal(env: Env, proposal_id: u64) -> Result<Proposal, ContractError> {
        get_proposal(&env, proposal_id).ok_or(ContractError::ProposalNotFound)
    }

    #[allow(dead_code)]
    pub fn query_signatures(
        env: Env,
        proposal_id: u64,
    ) -> Result<Vec<(Address, bool)>, ContractError> {
        let multisig = get_multisig_members(&env);
        // collect all addresses that signed this proposal
        let proposal_signatures = get_proposal_signatures(&env, proposal_id);

        let mut response: Vec<(Address, bool)> = vec![&env];

        for (member, _) in multisig {
            if proposal_signatures.get(member.clone()).is_some() {
                response.push_back((member, true));
            } else {
                response.push_back((member, false));
            }
        }

        Ok(response)
    }

    #[allow(dead_code)]
    pub fn query_last_proposal_id(env: Env) -> Result<u64, ContractError> {
        let last_id = get_last_proposal_id(&env);
        Ok(last_id)
    }

    #[allow(dead_code)]
    pub fn query_all_proposals(env: Env) -> Result<Vec<Proposal>, ContractError> {
        let last_proposal_id = get_last_proposal_id(&env);
        let mut proposals: Vec<Proposal> = vec![&env];

        // I think get_proposal should return the option, so that this won't fail in case
        // of deleted proposal
        for i in 1..=last_proposal_id {
            get_proposal(&env, i).is_some().then(|| {
                let current_prosal = get_proposal(&env, i).unwrap();
                proposals.push_back(current_prosal);
            });
        }

        Ok(proposals)
    }

    #[allow(dead_code)]
    pub fn is_proposal_ready(env: Env, proposal_id: u64) -> Result<bool, ContractError> {
        let multisig_len = get_multisig_members(&env).len();
        let signed = get_proposal_signatures(&env, proposal_id).len();

        let required_quorum = Decimal::bps(get_quorum_bps(&env) as i64);
        let voted_ratio = Decimal::from_ratio(signed, multisig_len);

        if voted_ratio >= required_quorum {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

fn verify_members(env: &Env, members: &Vec<Address>) {
    if members.is_empty() {
        log!(
            &env,
            "Multisig: Initialize: cannot initialize multisig without any members!"
        );
        panic_with_error!(&env, ContractError::MembersListEmpty);
    }

    let zero_address = Address::from_string(&String::from_str(env, SOROBAN_ZERO_ADDRESS));

    if members.iter().any(|addr| addr == zero_address) {
        log!(
            &env,
            "Multisig: Initialize: Stellar's zero address provided as member. Aborting"
        );
        panic_with_error!(&env, ContractError::ZeroAddressProvided);
    }
}

#[cfg(test)]
mod test {
    use soroban_sdk::{testutils::Address as _, vec, Address, Env, String, Vec};

    use crate::SOROBAN_ZERO_ADDRESS;

    use super::verify_members;

    #[test]
    #[should_panic(
        expected = "Multisig: Initialize: cannot initialize multisig without any members!"
    )]
    fn verify_members_should_panic_when_members_is_empty() {
        let env = Env::default();
        let members: Vec<Address> = vec![&env];

        verify_members(&env, &members);
    }

    #[test]
    #[should_panic(
        expected = "Multisig: Initialize: Stellar's zero address provided as member. Aborting"
    )]
    fn verify_members_should_panic_when_stellar_zero_addres_is_member() {
        let env = Env::default();

        let zero_address = Address::from_string(&String::from_str(&env, SOROBAN_ZERO_ADDRESS));

        let members: Vec<Address> = vec![
            &env,
            Address::generate(&env),
            Address::generate(&env),
            zero_address,
        ];

        verify_members(&env, &members);
    }

    #[test]
    fn verify_members_should_work() {
        let env = Env::default();

        let members: Vec<Address> = vec![&env, Address::generate(&env), Address::generate(&env)];

        verify_members(&env, &members);
    }
}
