# Tangle Network: Gadget SDK

A comprehensive toolkit for building, deploying, and managing blueprints to run on gadgets on the Tangle Network. This workspace provides a collection of Rust crates that enable developers to create and interact with blockchain-based applications.

## Table of Contents

- [Overview](#overview)
- [Features](#-features)
- [Project Structure](#-project-structure)
- [Prerequisites](#-prerequisites)
- [Getting Started](#-getting-started)
  - [Installation](#installation)
  - [Creating Your First Blueprint](#creating-your-first-blueprint)
  - [Building and Testing](#building-and-testing)
  - [Deployment](#deployment)
- [Key Management](#-key-management)
  - [Key Generation](#key-generation)
  - [Supported Key Types](#supported-key-types)
- [Configuration](#-configuration)
  - [Feature Flags](#feature-flags)
  - [Environment Variables](#environment-variables)
- [Core Components](#-core-components)
  - [Blueprint System](#blueprint-system)
  - [Network Clients](#network-clients)
  - [Cryptography](#cryptography)
  - [Event System](#event-system)
  - [Storage](#storage)
- [Development](#-development)
  - [Testing](#testing)
- [Contributing](#-contributing)
- [Support](#-support)

## Overview

Tangle Network's Gadget SDK is a modular framework designed to simplify the development and deployment of blockchain applications (blueprints) on the Tangle Network. It provides a comprehensive set of tools and libraries for blockchain interaction, cryptographic operations, and network communication.

## 🌟 Features

- **Blueprint System**

  - Template-based blueprint creation
  - Automated deployment workflows
  - Metadata Management

- **Multi-Chain Support**

  - Native Tangle Network integration
  - EigenLayer compatibility
  - EVM chain support
  - Cross-chain communication

- **Advanced Cryptography**

  - Multiple signature schemes (BLS, Ed25519, SR25519)
  - Secure key management

- **Networking**

  - P2P communication via libp2p
  - Custom protocol implementations
  - NAT traversal
  - Peer discovery and management

- **Development Tools**
  - CLI for common operations
  - Comprehensive testing framework
  - Performance benchmarking
  - Debugging utilities

## 🛠 Project Structure

```
tangle-network-gadget-workspace
├── blueprints               # Blueprint examples and templates
├── cli                      # Cargo-tangle Command-line interface tool
├── crates                   # Core functionality crates
│   ├── benchmarking         # Performance testing tools
│   ├── blueprint            # Blueprint core system and utilities
│   ├── clients              # Network clients (Tangle, EVM, EigenLayer)
│   ├── config               # Configuration management
│   ├── contexts             # Execution contexts
│   ├── crypto               # Cryptographic implementations
│   ├── eigenlayer-bindings  # EigenLayer smart contract bindings
│   ├── executor             # Task execution system
│   ├── keystore             # Key management and storage
│   ├── logging              # Logging infrastructure
│   ├── macros               # Procedural and derive macros
│   ├── metrics              # Performance and monitoring metrics
│   ├── networking           # P2P networking and communication
│   ├── sdk                  # Software Development Kit
│   ├── std                  # Standard library extensions
│   ├── stores               # Storage implementations
│   ├── testing-utils        # Testing utilities and helpers
│   └── utils                # Common utilities and helpers
├── .config                  # Configuration files
└── rust-toolchain.toml      # Rust version and components
```

## 📋 Prerequisites

- Rust (nightly-2024-10-13)
- Cargo
- OpenSSL development packages
- CMake (for certain dependencies)

For Ubuntu/Debian:

```bash
apt install build-essential cmake libssl-dev pkg-config
```

For macOS:

```bash
brew install openssl cmake
```

## 🚀 Getting Started

### Installation

1. Install the Tangle CLI:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/tangle-network/gadget/releases/download/cargo-tangle/v0.1.1-beta.7/cargo-tangle-installer.sh | sh
```

Or install from source:

```bash
cargo install cargo-tangle --git https://github.com/tangle-network/gadget --force
```

### Creating Your First Blueprint

1. Create a new blueprint:

```bash
cargo tangle blueprint create --name my_blueprint
```

2. Build your blueprint:

```bash
cargo build
```

3. Deploy to Tangle Network:

```bash
cargo tangle blueprint deploy --rpc-url wss://rpc.tangle.tools --package my_blueprint
```

## 🔑 Key Management

### Key Generation

Generate cryptographic keys using the CLI:

```bash
cargo tangle blueprint generate-keys -k <KEY_TYPE> -p <PATH> -s <SURI/SEED> --show-secret
```

### Supported Key Types

| Key Type  | Description                                | Use Case                          |
| --------- | ------------------------------------------ | --------------------------------- |
| sr25519   | Schnorrkel/Ristretto x25519                | Tangle Network account keys       |
| ecdsa     | Elliptic Curve Digital Signature Algorithm | EVM compatible chains             |
| bls_bn254 | BLS signatures on BN254 curve              | EigenLayer validators             |
| ed25519   | Edwards-curve Digital Signature Algorithm  | General purpose signatures        |
| bls381    | BLS signatures on BLS12-381 curve          | Advanced cryptographic operations |

## 🔧 Configuration

### Feature Flags

The CLI and core libraries support various feature flags for customizing functionality when building blueprints:

| Feature Flag | Description               | Components Included                                      |
| ------------ | ------------------------- | -------------------------------------------------------- |
| default      | Standard installation     | Tangle + EVM support with standard library features      |
| std          | Standard library features | Core functionality with std support                      |
| tangle       | Tangle Network support    | Tangle Network client, keystore, and EVM integration     |
| eigenlayer   | EigenLayer integration    | EigenLayer clients, keystore, and EVM support            |
| evm          | EVM chain support         | Ethereum JSON ABI, provider, network, and signer support |

The crypto system supports multiple signature schemes:

- k256 (ECDSA)
- sr25519 (Schnorrkel)
- ed25519
- BLS (including BN254)
- Tangle pair signer
- Substrate crypto (sp-core)

Installation examples:

```bash
# Default installation (includes Tangle + EVM)
cargo install cargo-tangle

# EVM support only
cargo install cargo-tangle --features evm

# Full installation with EigenLayer support
cargo install cargo-tangle --features "tangle,eigenlayer"
```

### Environment Variables

Required environment variables for different operations:

| Variable     | Description                   | Example                                          |
| ------------ | ----------------------------- | ------------------------------------------------ |
| SIGNER       | Substrate signer account SURI | `export SIGNER="//Alice"`                        |
| EVM_SIGNER   | EVM signer private key        | `export EVM_SIGNER="0xcb6df..."`                 |
| RPC_URL      | Tangle Network RPC endpoint   | `export RPC_URL="wss://rpc.tangle.tools"`        |
| HTTP_RPC_URL | HTTP RPC endpoint             | `export HTTP_RPC_URL="https://rpc.tangle.tools"` |

## 🔨 Core Components

### Blueprint System

The Blueprint system is the core of the Tangle Network Gadget framework:

- **Template Engine**: Standardized blueprint creation
- **Metadata Management**: Blueprint information and configuration

### Network Clients

Specialized clients for different blockchain networks:

- **Tangle Client**: Native integration with Tangle Network
- **EigenLayer Client**: AVS (Actively Validated Service) integration
- **EVM Client**: Ethereum and EVM-compatible chain support

### Cryptography

Comprehensive cryptographic implementations:

- **Multiple Schemes**: Support for various signature algorithms
- **Key Management**: Secure key storage and handling

### Event System

Robust event handling system:

- **Event Listeners**: Custom event monitoring
- **Async Processing**: Non-blocking event handling
- **Filtering**: Configurable event filtering
- **Error Handling**: Robust error recovery

### Storage

Flexible storage solutions:

- **Local Database**: Efficient local storage
- **Key-Value Store**: Fast key-value operations
- **File System**: Secure file storage
- **Remote Storage**: Cloud storage integration (e.g., AWS, GCP, Ledger)

## 🧪 Development

### Testing

The framework includes comprehensive testing tools:

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --package my_blueprint

# Run with logging
RUST_LOG=gadget=debug cargo test
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details on how to get started.

## 📮 Support

- **Issues**: Use GitHub Issues for bug reports and feature requests
- **Discussions**: Join our community discussions on GitHub
- **Discord**: Join our [Discord server](https://discord.com/invite/cv8EfJu3Tn)
