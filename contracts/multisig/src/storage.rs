use soroban_sdk::{contracttype, map, Address, BytesN, Env, Map, String, Vec};

use crate::{BUMP_AMOUNT, LIFETIME_THRESHOLD};

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proposal {
    pub id: u64,
    pub sender: Address,
    pub proposal: ProposalType,
    pub status: ProposalStatus,
    pub creation_timestamp: u64,
    pub expiration_timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProposalType {
    // Transfer tokens from Multisig contract to the recipient
    Transaction(Transaction),
    // Update the multisig's wasm bytecode with this wasm hash
    UpdateContract(BytesN<32>),
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProposalStatus {
    Open,
    Closed,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    pub token: Address,
    pub amount: u64,
    pub recipient: Address,
    pub title: String,
    pub description: String,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultisigInfo {
    pub name: String,
    pub description: String,
    pub members: Vec<Address>,
    pub quorum_bps: u32,
    pub version_proposal: u32,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    // Extra security measurement to not overwrite initial setup
    IsInitialized,
    NameDescription,
    // BPS representation of a configured quorum that is required to the transaction
    // to be executed
    QuorumBps,
    // A vector of all participants of the multisig
    Multisig,
    // Unique identifier for each new proposal to sign
    LastProposalId,
    // Details of the tranasction proposal
    Proposal(u64),
    // Record of signatures to each transaction proposal
    // TODO: Add method to clean up memory when the transaction is executed
    ProposalSignatures(u64),
    Version,
}

pub fn set_initialized(env: &Env) {
    env.storage().persistent().set(&DataKey::IsInitialized, &());
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::IsInitialized, LIFETIME_THRESHOLD, BUMP_AMOUNT);
}

pub fn is_initialized(env: &Env) -> bool {
    let is_initialized = env
        .storage()
        .persistent()
        .get::<_, ()>(&DataKey::IsInitialized)
        .is_some();

    env.storage()
        .persistent()
        .has(&DataKey::IsInitialized)
        .then(|| {
            env.storage().persistent().extend_ttl(
                &DataKey::IsInitialized,
                LIFETIME_THRESHOLD,
                BUMP_AMOUNT,
            )
        });

    is_initialized
}

// -------------

pub fn set_name(env: &Env, name: String, description: String) {
    env.storage()
        .persistent()
        .set(&DataKey::NameDescription, &(name, description));
    env.storage().persistent().extend_ttl(
        &DataKey::NameDescription,
        LIFETIME_THRESHOLD,
        BUMP_AMOUNT,
    );
}

pub fn get_name(env: &Env) -> (String, String) {
    let name_tuple = env
        .storage()
        .persistent()
        .get(&DataKey::NameDescription)
        .unwrap();
    env.storage()
        .persistent()
        .has(&DataKey::NameDescription)
        .then(|| {
            env.storage().persistent().extend_ttl(
                &DataKey::NameDescription,
                LIFETIME_THRESHOLD,
                BUMP_AMOUNT,
            );
        });

    name_tuple
}

// -------------

pub fn get_quorum_bps(env: &Env) -> u32 {
    let quorum_bps = env.storage().persistent().get(&DataKey::QuorumBps).unwrap();

    env.storage()
        .persistent()
        .has(&DataKey::QuorumBps)
        .then(|| {
            env.storage().persistent().extend_ttl(
                &DataKey::QuorumBps,
                LIFETIME_THRESHOLD,
                BUMP_AMOUNT,
            );
        });

    quorum_bps
}

pub fn save_quorum_bps(env: &Env, quorum_bps: u32) {
    env.storage()
        .persistent()
        .set(&DataKey::QuorumBps, &quorum_bps);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::QuorumBps, LIFETIME_THRESHOLD, BUMP_AMOUNT);
}

// -------------

pub fn get_multisig_members(env: &Env) -> Map<Address, ()> {
    let members = env
        .storage()
        .persistent()
        .get(&DataKey::Multisig)
        // This vector is set during initialization
        // if it fails to load, it's a critical error
        .unwrap();

    env.storage().persistent().has(&DataKey::Multisig).then(|| {
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Multisig, LIFETIME_THRESHOLD, BUMP_AMOUNT);
    });

    members
}

#[allow(dead_code)]
pub fn add_multisig_member(env: &Env, member: Address) {
    let mut multisig = get_multisig_members(env);
    multisig.set(member, ());

    env.storage()
        .persistent()
        .set(&DataKey::Multisig, &multisig);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Multisig, LIFETIME_THRESHOLD, BUMP_AMOUNT);
}

pub fn save_new_multisig(env: &Env, members: &Vec<Address>) {
    let mut multisig: Map<Address, ()> = map!(env);
    for member in members.iter() {
        multisig.set(member, ());
    }

    env.storage()
        .persistent()
        .set(&DataKey::Multisig, &multisig);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Multisig, LIFETIME_THRESHOLD, BUMP_AMOUNT);
}

// -------------

// Returns ID for the new proposal and increments the value in the memory for the future one
pub fn increment_last_proposal_id(env: &Env) -> u64 {
    let id = env
        .storage()
        .persistent()
        .get::<_, u64>(&DataKey::LastProposalId)
        .unwrap_or_default()
        + 1u64;
    env.storage()
        .persistent()
        .set(&DataKey::LastProposalId, &id);

    env.storage().persistent().extend_ttl(
        &DataKey::LastProposalId,
        LIFETIME_THRESHOLD,
        BUMP_AMOUNT,
    );

    id
}

pub fn get_last_proposal_id(env: &Env) -> u64 {
    let last_id = env
        .storage()
        .persistent()
        .get(&DataKey::LastProposalId)
        .unwrap_or_default();

    env.storage()
        .persistent()
        .has(&DataKey::LastProposalId)
        .then(|| {
            env.storage().persistent().extend_ttl(
                &DataKey::LastProposalId,
                LIFETIME_THRESHOLD,
                BUMP_AMOUNT,
            );
        });

    last_id
}

// -------------

pub fn save_proposal(env: &Env, proposal: &Proposal) {
    env.storage()
        .persistent()
        .set(&DataKey::Proposal(proposal.id), proposal);
    env.storage().persistent().extend_ttl(
        &DataKey::Proposal(proposal.id),
        LIFETIME_THRESHOLD,
        BUMP_AMOUNT,
    );
}

pub fn get_proposal(env: &Env, proposal_id: u64) -> Option<Proposal> {
    let proposal = env
        .storage()
        .persistent()
        .get(&DataKey::Proposal(proposal_id));

    env.storage()
        .persistent()
        .has(&DataKey::Proposal(proposal_id))
        .then(|| {
            env.storage().persistent().extend_ttl(
                &DataKey::Proposal(proposal_id),
                LIFETIME_THRESHOLD,
                BUMP_AMOUNT,
            );
        });

    proposal
}

// -------------

// When user signes the given proposal, save an information about it
pub fn save_proposal_signature(e: &Env, proposal_id: u64, signer: Address) {
    let mut proposal_signatures: Map<Address, ()> = get_proposal_signatures(e, proposal_id);
    proposal_signatures.set(signer, ());

    e.storage().persistent().set(
        &DataKey::ProposalSignatures(proposal_id),
        &proposal_signatures,
    );
    e.storage().persistent().extend_ttl(
        &DataKey::ProposalSignatures(proposal_id),
        LIFETIME_THRESHOLD,
        BUMP_AMOUNT,
    );
}

pub fn get_proposal_signatures(env: &Env, proposal_id: u64) -> Map<Address, ()> {
    let proposal_signatures = env
        .storage()
        .persistent()
        .get(&DataKey::ProposalSignatures(proposal_id))
        .unwrap_or(map![&env]);

    env.storage()
        .persistent()
        .has(&DataKey::ProposalSignatures(proposal_id))
        .then(|| {
            env.storage().persistent().extend_ttl(
                &DataKey::ProposalSignatures(proposal_id),
                LIFETIME_THRESHOLD,
                BUMP_AMOUNT,
            )
        });

    proposal_signatures
}

pub fn save_version(env: &Env, version: &u32) {
    env.storage().persistent().set(&DataKey::Version, version);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Version, LIFETIME_THRESHOLD, BUMP_AMOUNT);
}

pub fn get_version(env: &Env) -> u32 {
    let version = env
        .storage()
        .persistent()
        .get(&DataKey::Version)
        .unwrap_or_default();

    env.storage().persistent().has(&DataKey::Version).then(|| {
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Version, LIFETIME_THRESHOLD, BUMP_AMOUNT)
    });

    version
}

pub fn increase_version(env: &Env) {
    let version = get_version(env) + 1;
    save_version(env, &version);
}
