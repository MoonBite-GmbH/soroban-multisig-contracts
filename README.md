# Soroban Multisig contract

## Abstract

This is an attempt to create a smart contract allowing to initialize a multisig with a configurable quorum of required signers.
The first and primar operation that needs to work is a simple transfer of tokens after the quorum is met.

## Workflow

At the moment contract allows you to initialize a multisig by passing a group of members and optionally a required quorum - if it supposed to be other then 100%. At the moment none of the members can be removed and no new can be added (coming through via new proposal types).
Currently supported proposal type is a transaction. The flow is - when specifying the transaction, tokens are transferred from the multisig wallet to the recipient during execution. To execute a proposal, enough users must sign given proposal to meet the quorum.

## License
The smart contracts and associated code in this repository are licensed under the GPL-3.0 License. By contributing to this project, you agree that your contributions will also be licensed under the GPL-3.0 license.

For the full license text, please see the LICENSE file in the root directory of this repository.

## Contact
If you have any questions or issues, please create a new issue on the GitHub repository.

