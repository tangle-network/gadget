# Gadget

## Design


![](./resources/gadget.png)

The core library is `gadget-core`. The core library allows gadgets to hold standardization of use across different blockchains. The core library is the base of all gadgets, and expects to receive `FinalityNotifications` and `BlockImportNotifications`.

Once such blockchain is a substrate blockchain. This is where `webb-gadget` comes into play. The `webb-gadget` is the `core-gadget` endowed with a connection to a substrate blockchain, a networking layer to communicate with other gadgets, and a `WebbGadgetModule` that has application-specific logic. 

Since `webb-gadget` allows varying connections to a substrate blockchain and differing network layers, we can thus design above it the `zk-gadget` and `tss-gadget`. These gadgets are endowed with the same functionalities as the `webb-gadget` but with a (potentially) different blockchain connection, networking layer, and application-specific logic.

## Testing

### ZK-Gadget
To test the ZK-gadget, you can run `./playground/run.sh --create-certs`. Additionally, you may use docker-compose to set up a testnet automatically, running a ZK test protocol in the process:

`docker-compose up --abort-on-container-exit`

## Troubleshooting
The linking phase may fail due to not finding libgmp (i.e., "could not find library -lgmp") when building on a mac M1. To fix this problem, run:

```bash
brew install gmp
# make sure to run the commands below each time when starting a new env, or, append them to .zshrc
export LIBRARY_PATH=$LIBRARY_PATH:/opt/homebrew/lib
export INCLUDE_PATH=$INCLUDE_PATH:/opt/homebrew/include