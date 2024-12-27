# Tangle CLI

Create and Deploy blueprints on Tangle Network.

## Table of Contents

- [Tangle CLI](#tangle-cli)
  - [Table of Contents](#table-of-contents)
  - [Overview](#overview)
  - [Installation](#installation)
    - [Feature flags](#feature-flags)
  - [Creating a New Blueprint/Gadget](#creating-a-new-blueprintgadget)
    - [Example](#example)
  - [Build The Blueprint and the Gadget](#build-the-blueprint-and-the-gadget)
  - [Unit Testing](#unit-testing)
  - [Deploying the Blueprint to a Local Tangle Node](#deploying-the-blueprint-to-a-local-tangle-node)
    - [Example](#example-1)
  - [Required Environment Variables for Deployment](#required-environment-variables-for-deployment)
    - [Example of ENV Variables](#example-of-env-variables)
  - [Generating Keys from the Command Line](#generating-keys-from-the-command-line)
    - [Flags](#flags)

## Overview

The Tangle CLI is a command-line tool that allows you to create and deploy blueprints/gadgets on the Tangle network. It
provides a simple and efficient way to manage your blueprints and gadgets, making it easy to get started with Tangle
Blueprints.

## Installation

To install the Tangle CLI, run the following command:

> Supported on Linux, MacOS, and Windows (WSL2)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/tangle-network/gadget/releases/download/cargo-tangle/v0.1.1-beta.7/cargo-tangle-installer.sh | sh
```

Or, if you prefer to install the CLI from source:

```bash
cargo install cargo-tangle --git https://github.com/tangle-network/gadget --force
```

### Feature flags

The CLI supports installation that targets specific environments if reducing dependencies is desired. The following feature flags are available:

```bash
# Just Tangle support (includes EVM) - default
cargo install cargo-tangle

# Just EVM support without Tangle/EigenLayer
cargo install cargo-tangle --features evm

# EigenLayer support (includes EVM)
cargo install cargo-tangle --features eigenlayer

# Key generation tools
cargo install cargo-tangle --features keys

# Full installation
cargo install cargo-tangle --features "tangle,eigenlayer,keys"
```

## Creating a New Blueprint/Gadget

To create a new blueprint/gadget using the Tangle CLI, use the following command:

```bash
cargo tangle blueprint create --name <blueprint_name>
```

Replace `<blueprint_name>` with the desired name for your blueprint.

### Example

```bash
cargo tangle blueprint create --name my_blueprint
```

## Build The Blueprint and the Gadget

To build the blueprint and the gadget, you can simply use cargo as you would with any rust project:

```bash
cargo build
```

## Unit Testing

To run the unit tests, use the following command:

```bash
cargo test
```

## Deploying the Blueprint to a Local Tangle Node

To deploy the blueprint to a local Tangle node, use the following command:

```bash
export SIGNER="//Alice" # Substrate Signer account
export EVM_SIGNER="0xcb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854" # EVM signer account
cargo tangle blueprint deploy --rpc-url <rpc_url> --package <package_name>
```

Replace `<rpc_url>` with the RPC URL of your local Tangle node and `<package_name>` with the name of the package to
deploy.

### Example

```bash
cargo tangle blueprint deploy --rpc-url ws://localhost:9944 --package my_blueprint
```

Expected output:

```
Blueprint #0 created successfully by 5F3sa2TJAWMqDhXG6jhV4N8ko9rUjC2q7z6z5V5s5V5s5V5s with extrinsic hash: 0x1234567890abcdef
```

## Required Environment Variables for Deployment

The following environment variables are required for deploying the blueprint:

- `SIGNER`: The SURI of the signer account.
- `EVM_SIGNER`: The SURI of the EVM signer account.

### Example of ENV Variables

```bash
export SIGNER="//Alice" # Substrate Signer account
export EVM_SIGNER="0xcb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854" # EVM signer account
```

## Generating Keys from the Command Line

The following command will generate a keypair for a given key type:

```shell
cargo tangle blueprint generate-keys -k <KEY_TYPE> -p <PATH> -s <SURI/SEED> --show-secret
```

where it is optional to include the path, seed, or the show-secret flags.

### Flags

- `-k` or `--key-type`: Required flag. The key type to generate (sr25519, ecdsa, bls_bn254, ed25519, bls381).
- `-p` or `--path`: The path to write the generated keypair to. If not provided, the keypair will be written solely to stdout.
- `-s` or `--seed`: The suri/seed to generate the keypair from. If not provided, a random keypair will be generated.
- `--show-secret`: Denotes that the Private Key should also be printed to stdout. If not provided, only the public key will be printed.
