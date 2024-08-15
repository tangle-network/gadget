# Tangle CLI

Create and Deploy blueprints on Tangle Network.

## Table of Contents

1. [Overview](#overview)
2. [Installation](#installation)
3. [Creating a New Blueprint/Gadget](#creating-a-new-blueprintgadget)
4. [Deploying the Blueprint to a Local Tangle Node](#deploying-the-blueprint-to-a-local-tangle-node)
5. [Required Environment Variables for Deployment](#required-environment-variables-for-deployment)
6. [Examples](#examples)

## Overview

The Tangle CLI is a command-line tool that allows you to create and deploy blueprints/gadgets on the Tangle network. It provides a simple and efficient way to manage your blueprints and gadgets, making it easy to get started with Tangle Blueprints.

## Installation

To install the Tangle CLI from crates.io, run the following command:

```bash
cargo install cargo-tangle
```

## Creating a New Blueprint/Gadget

To create a new blueprint/gadget using the Tangle CLI, use the following command:

```bash
cargo tangle gadget create --name <blueprint_name>
```

Replace `<blueprint_name>` with the desired name for your blueprint/gadget.

### Example

```bash
cargo tangle gadget create --name my_blueprint
```

## Deploying the Blueprint to a Local Tangle Node

To deploy the blueprint to a local Tangle node, use the following command:

```bash
cargo tangle gadget deploy --rpc-url <rpc_url> --package <package_name>
```

Replace `<rpc_url>` with the RPC URL of your local Tangle node and `<package_name>` with the name of the package to deploy.

### Example

```bash
cargo tangle gadget deploy --rpc-url ws://localhost:9944 --package my_blueprint
```

Expected output:

```
Blueprint #0 created successfully by 5F3sa2TJAWMqDhXG6jhV4N8ko9rUjC2q7z6z5V5s5V5s5V5s with extrinsic hash: 0x1234567890abcdef
```

## Required Environment Variables for Deployment

The following environment variables are required for deploying the blueprint:

- `SIGNER`: The SURI of the signer account.
- `EVM_SIGNER`: The SURI of the EVM signer account.

### Example

```bash
export SIGNER="//Alice"
export EVM_SIGNER="0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
```
