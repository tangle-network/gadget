## How to run the shell locally

To be able to run the demo locally, you will need a tangle node and this repository.

### Setup Tangle Node

1. Clone [`tangle`](https://github.com/webb-tools/tangle) Locally
```bash
git clone https://github.com/webb-tools/tangle.git
```
2. Checkout the `shady/gadget-shell-testing` branch
```bash
git checkout shady/gadget-shell-sdk-testing
```
3. Compile tangle in release mode.
```bash
cargo build --release
```
4. Run the `target/release/tangle` binary using the following script:
```bash
bash ./scripts/run-standalone-local.sh --clean
```

### Setup Gadget Shell

1. Compile the Gadget Shell in release mode
```bash
cargo build --release
```
2. Run the `target/release/gadget-shell` binary using the following command:

```bash
./target/release/gadget-shell-sdk --config shell-sdk-configs/local-testnet-0.toml -vvv
```

> Note: The `--config` flag is used to specify the configuration file to use. In this case, we are using the `local-testnet-0.toml` configuration file.

3. Run different shells with different configurations to simulate a network of shells.

```bash
./target/release/gadget-shell-sdk --config shell-sdk-configs/local-testnet-1.toml -vvv
```

```bash
./target/release/gadget-shell-sdk --config shell-sdk-configs/local-testnet-2.toml -vvv
```

> Note: You will notice they automatically connect to each other and start exchanging messages.

### Polkadot JS Apps

Additionally, you may also want to run a local instance of Polkadot JS Apps to interact with the node, create jobs, etc.
Go to https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:9944 and connect to the local node.

### Send Jobs Using a Playground Script

You can also use the `playground` script to send jobs to the network. To do this, you will need to have the `tangle` binary running and the `gadget-shell` running.
Then, in the tangle repo, enter the `types` directory and run the following command:

```bash
ts-node playground.ts
```

This will create profiles for `Alice`, `Bob` and `Charlie`, and send a DKG Phase One job to the network. You can then use the Polkadot JS apps to see the job being processed.
