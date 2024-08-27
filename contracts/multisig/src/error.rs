use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    Unauthorized = 0,
    AlreadyInitialized = 1,
    InitializeTooLowQuorum = 2,
    InitializeTooHighQuorum = 3,
    UnauthorizedNotAMember = 4,
    TitleTooLong = 5,
    DescriptionTooLong = 6,
    ProposalClosed = 7,
    QuorumNotReached = 8,
    ProposalNotFound = 9,
    ProposalExpired = 10,
    InvalidExpirationDate = 11,
    MembersListEmpty = 12,
    ZeroAddressProvided = 13,
}
