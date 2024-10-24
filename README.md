<p align="center">
  <img src="https://github.com/tangle-network/dkg-substrate/raw/master/assets/webb_banner_light.png" alt="Gadget Logo">
</p>

[![License](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust Version](https://img.shields.io/badge/rust-1.74.0%2B-blue.svg)](https://www.rust-lang.org)

# Gadget: A Powerful distributed AVS Framework

[Gadget SDK](./sdk)
| [CLI](./cli)
| [Tangle Operator Docs](https://docs.tangle.tools/operators/validator/introduction)
| [Tangle Developer Docs](https://docs.tangle.tools/developers)

Gadget is a comprehensive framework for building AVS services on Tangle and Eigenlayer.
It provides a standardized framework for building task based systems and enables developers
to clearly specify jobs, slashing reports, benchmarks, and tests for offchain and onchain
service interactions. We plan to integrate with other restaking infrastructures over time,
if you are a project that is interested please reach out!

## Features

- Modular and extensible architecture
- Integration with [Tangle](https://twitter.com/tangle_network) and [Eigenlayer](https://www.eigenlayer.xyz/)
- Standardized job execution and submission mechanisms
- Protocol-specific blockchain connections, networking layers, and application logic
- Comprehensive testing framework

## Getting Started

Deploying a Blueprint to Tangle is made easy with commands provided by our CLI crate [cargo-tangle](./cli).
Let's get started!

### Installing the CLI

To install the Tangle CLI, run the following command:

> Supported on Linux, MacOS, and Windows (WSL2)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/tangle-network/gadget/releases/download/cargo-tangle/v0.1.1-beta.7/cargo-tangle-installer.sh | sh
```

Or, if you prefer to install the CLI from source:

```bash
cargo install cargo-tangle --git https://github.com/tangle-network/gadget --force
```

### Creating a Blueprint

To create a new blueprint/gadget using the Tangle CLI:

```bash
cargo tangle blueprint create --name <blueprint_name>
```

### Deploying a Blueprint

Finally, the blueprint can be deployed to a local Tangle node using the following command:

```bash
export SIGNER="//Alice" # Substrate Signer account
export EVM_SIGNER="0xcb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854" # EVM signer account
cargo tangle blueprint deploy --rpc-url <rpc_url> --package <package_name>
```

More information on this process can be found in the [CLI documentation](./cli/README.md)

### Testing a blueprint (beta)

In order to test a blueprint, you must first have a local Tangle node running. When setting up a local testnet for
integration testing, we recommend running this script for
testing: [run-standalone-local.sh](https://github.com/tangle-network/tangle/blob/main/scripts/run-standalone-local.sh),
passing `--clean` as an argument to reset the chain and any keys.

Then, you can run:

```bash
cargo test --package blueprint-test-utils tests_standard::test_externalities_gadget_starts -- --nocapture
```

Since testing is in beta stage, each time the blueprint is run, you
must cancel the testnet and restart it to ensure storage is reset.
All these nuances and manual requirement of setting up a testnet will be resolved in the near future and will be
testable via `cargo tangle blueprint test`

## License

Gadget is licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your discretion.

## Contributing

We welcome contributions to Gadget! If you have any ideas, suggestions, or bug reports, please open an issue or submit a
pull request.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## Contact

If you have any questions or need further information, please contact the developers [here](https://webb.tools/)
