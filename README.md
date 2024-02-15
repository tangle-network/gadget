# Gadget
## Introduction
A **gadget** is a peer on a network that queries a blockchain for job information, then, performs work based on these on-chain jobs cooperatively with other gadgets.
There are different types of jobs, such as threshold signatures, zero-knowledge proofs, and more. Each job type has a corresponding **protocol** that the gadgets use to perform the work.
These protocols, under the hood, each make use of an applied cryptographic protocol over a network, and work towards submitting job outputs to the blockchain.



## Design

The core library is `gadget-core`. The core library allows gadgets to hold standardization of use across different blockchains. The core library is the base of all gadgets, and expects to receive `FinalityNotifications` and `BlockImportNotifications`.
Once such blockchain is a substrate blockchain. This is where `gadget-common` comes into play. The `gadget-common` is the `core-gadget` endowed with a connection to a substrate blockchain, a networking layer to communicate with other gadgets, and a `SubstrateGadget` that has application-specific logic.
Since `gadget-common` allows varying connections to a substrate blockchain and differing network layers, we thus design above it various *protocols*. Some example protocols are `zk-saas-protocol`, `dfns-cggmp21-protocol`, `threshold-bls-protocol`, and `stub-protocol` (where the latter is for getting a bare minimum skeleton of a protocol crate setup).
These protocols are endowed with the same functionalities as the `gadget-common` but with a (potentially) different blockchain connection, networking layer, and application-specific logic using assistance from macros.
For more information on how to create a new protocol, see the README.md in the `protocols/stub` directory [here](protocols/stub/README.md).
## Testing

`cargo nextest run` is required to run tests, since 1-program per-program space is required for tests due to the nature of the use of static variables in test-only contexts.

## Troubleshooting
#### GMP Issues
The linking phase may fail due to not finding libgmp (i.e., "could not find library -lgmp") when building on a mac M1. To fix this problem, run:

```bash
brew install gmp
# make sure to run the commands below each time when starting a new env, or, append them to .zshrc
export LIBRARY_PATH=$LIBRARY_PATH:/opt/homebrew/lib
export INCLUDE_PATH=$INCLUDE_PATH:/opt/homebrew/include
```