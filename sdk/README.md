# Gadget SDK

Development tools for Gadgets targeting Tangle or EigenLayer.

## Table of Contents

1. [Overview](#overview)
2. [Installation](#installation)
3. [Features](#features)
4. [Examples](#examples)
5. [Building and Deploying Blueprints](#building-and-deploying-blueprints)

## Overview

The Gadget SDK is a comprehensive toolkit for Gadget development, providing tools and libraries for
Blueprint development on both Tangle and EigenLayer.

## Features

The Gadget SDK includes the following key features:

- Benchmarking tools
- Keystore management
- Metrics collection and reporting
- Networking utilities
- Logging systems
- Process execution and management
- Support for Native, WASM, and no-std contexts

## Installation

To add the Gadget SDK to your project, add the following to your `Cargo.toml`:

```toml
[dependencies]
gadget-sdk = { git = "https://github.com/webb-tools/gadget" }
```

### Examples

Example Blueprints can be found [here](./../blueprints):

- [Incredible Squaring](./../blueprints/incredible-squaring)
- [ECDSA Threshold MPC](./../blueprints/ecdsa-threshold-mpc)
- [Tangle AVS](./../blueprints/tangle-avs-blueprint)

## Building and Deploying Blueprints

Information on building a Blueprint and then Deploying it can be found [here](./../cli/README.md).