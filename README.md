# Gadget
This repo contains code for MPC and other restaking service gadgets. A gadget is a service that listens to a job management system (such as a blockchain w/ onchain job management logic) and communicates with other service providers using a peer to peer or alternative networking stack. Currently, the main services the gadget implements are multi-party computation services such as threshold signature MPCs and an MPC proving service for Groth16 zkSNARKs.

- [x] [DFNS CGGMP21](https://github.com/dfns/cggmp21/tree/m/cggmp21)
- [x] [Threshold BLS](https://github.com/mikelodder7/blsful)
- [x] [LIT Protocol fork of ZCash Frost](https://github.com/LIT-Protocol/frost)
- [x] [Groth16 ZK-SaaS](https://github.com/webb-tools/zk-SaaS)

## Design

The core library is `gadget-core`. The core library allows gadgets to hold standardization of use across different blockchains that implement a compatible job management and submission infrastructure. All gadgets should implement the relevant traits from `gadget-core`, which implement job allocation, completion, and submission. Currently, gadgets expect to receive `FinalityNotifications` and `BlockImportNotifications` so blockchains with finality are mainly targetted.

Currently the repo is built around Substrate blockchain logic and networking. The job system implemented by [Tangle](https://github.com/webb-tools/tangle) drives the current job allocation mechanism. Validators of a Substrate chain implementing Tangle's runtime pallets execute jobs assigned to them from an onchain job submission system and use the underlying Substrate p2p layer to communicate with other service peers.

The core library is `gadget-core`. The core library allows gadgets to hold standardization of use across different blockchains. The core library is the base of all gadgets, and expects to receive `FinalityNotifications` and `BlockImportNotifications`.
Once such blockchain is a substrate blockchain. This is where `gadget-common` comes into play. The `gadget-common` is the `core-gadget` endowed with a connection to a substrate blockchain, a networking layer to communicate with other gadgets, and a `SubstrateGadget` that has application-specific logic.
Since `gadget-common` allows varying connections to a substrate blockchain and differing network layers, we thus design above it various *protocols*. Some example protocols are `zk-saas-protocol`, `dfns-cggmp21-protocol`, `threshold-bls-protocol`, and `stub-protocol` (where the latter is for getting a bare minimum skeleton of a protocol crate setup).
These protocols are endowed with the same functionalities as the `gadget-common` but with a (potentially) different blockchain connection, networking layer, and application-specific logic using assistance from macros.
For more information on how to create a new protocol, see the README.md in the `protocols/stub` directory [here](protocols/stub/README.md).
## Testing

`SKIP_WASM_BUILD=true RUST_LOG=debug cargo nextest run` is required to run tests, since 1-program per-program space is required for tests due to the nature of the use of static variables in test-only contexts. There is currently an issue with the WASM build so the `SKIP_WASM_BUILD` flag is required. The `RUST_LOG=debug` flag is optional but useful for debugging.

## Troubleshooting
#### GMP Issues
The linking phase may fail due to not finding libgmp (i.e., "could not find library -lgmp") when building on a mac M1. To fix this problem, run:

```bash
brew install gmp
# make sure to run the commands below each time when starting a new env, or, append them to .zshrc
export LIBRARY_PATH=$LIBRARY_PATH:/opt/homebrew/lib
export INCLUDE_PATH=$INCLUDE_PATH:/opt/homebrew/include
```