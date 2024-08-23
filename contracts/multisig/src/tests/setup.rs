use soroban_sdk::{Address, Env, String, Vec};

use crate::{
    contract::{Multisig, MultisigClient},
    token_contract, SEVEN_DAYS_DEADLINE,
};

pub const DAY_AS_TIMESTAMP: u64 = 86_400u64;
pub const TWO_WEEKS_DEADLINE: u64 = SEVEN_DAYS_DEADLINE * 2;

pub fn deploy_token_contract<'a>(env: &Env, admin: &Address) -> token_contract::Client<'a> {
    token_contract::Client::new(env, &env.register_stellar_asset_contract(admin.clone()))
}

pub fn initialize_multisig_contract<'a>(
    env: &Env,
    name: String,
    description: String,
    members: Vec<Address>,
    quorum_bps: impl Into<Option<u32>>,
) -> MultisigClient<'a> {
    let multisig = MultisigClient::new(env, &env.register_contract(None, Multisig {}));

    multisig.initialize(&name, &description, &members, &quorum_bps.into());

    multisig
}
