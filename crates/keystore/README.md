# Gadget Keystore

A flexible and secure keystore implementation supporting multiple key types, storage backends, and remote signing capabilities.

## Features

### Core Features

- `std` - Standard library support (default enabled)
  - Enables std-dependent functionality across all enabled features
  - Required for filesystem storage and remote signing
- `no_std` - No standard library support (not yet working)

### Cryptographic Primitives

- `ecdsa` - ECDSA support using k256

  - Enables secp256k1 ECDSA keys
  - Required for EVM compatibility
  - Includes RIPEMD for address generation

- `sr25519-schnorrkel` - Schnorrkel/sr25519 support

  - Substrate's signature scheme
  - Used for Polkadot/Substrate chains

- `zebra` - Ed25519 implementation from Zebra

  - High-performance Ed25519 implementation
  - Used for Zcash compatibility

- `bls381` - BLS signatures on BLS12-381

  - W3F's BLS implementation
  - Used for threshold signatures

- `bn254` - BN254 curve support
  - Used for Ethereum 2.0 BLS signatures
  - Required for EigenLayer compatibility

### Protocol Support

- `evm` - Ethereum Virtual Machine support

  - Enables ECDSA key generation and signing
  - Includes Alloy primitives for EVM compatibility
  - Required for all EVM-based chains

- `tangle` - Tangle protocol support

  - Includes Substrate core functionality
  - Enables BLS381 experimental features
  - Required for Tangle chain interaction

- `eigenlayer` - EigenLayer protocol support

  - Includes EVM compatibility
  - Enables BN254 curve support
  - Required for EigenLayer operator nodes

- `symbiotic` - Symbiotic protocol support
  - Basic EVM compatibility
  - Extensible for future features

### Remote Signing Support

- `remote` - Base remote signing support

  - Required for all remote signing features
  - Enables async signing capabilities

- `aws-signer` - AWS KMS support

  - Sign with AWS KMS keys
  - Requires AWS credentials

- `gcp-signer` - Google Cloud KMS support

  - Sign with GCP KMS keys
  - Requires GCP credentials

- `ledger-browser` - Ledger support (browser)

  - Sign with Ledger devices in browser
  - WebUSB support

- `ledger-node` - Ledger support (node)
  - Sign with Ledger devices in Node.js
  - Node HID support

### Feature Bundles

- `tangle-full` - Complete Tangle support

  - Includes `tangle` and `bn254`
  - All Tangle-specific key types

- `eigenlayer-full` - Complete EigenLayer support

  - Includes `eigenlayer`, `sr25519-schnorrkel`, `zebra`, `bls381`, `bn254`
  - All operator node key types

- `symbiotic-full` - Complete Symbiotic support

  - Includes `symbiotic`, `sr25519-schnorrkel`, `zebra`, `bls381`, `bn254`
  - All supported key types

- `all-remote-signers` - All remote signing capabilities
  - Includes all remote signer features
  - AWS, GCP, and Ledger support

## Usage

Enable the features you need in your `Cargo.toml`:

```toml
[dependencies]
gadget-keystore = { version = "0.1", features = ["std", "ecdsa", "remote", "aws-signer"] }
```

For full functionality:

```toml
gadget-keystore = { version = "0.1", features = ["std", "tangle-full", "eigenlayer-full", "all-remote-signers"] }
```

## Feature Dependencies

- `aws-signer` requires `remote`, `evm`, and `std`
- `gcp-signer` requires `remote`, `evm`, and `std`
- `ledger-browser` requires `remote` and `evm`
- `ledger-node` requires `remote` and `evm`
- `eigenlayer` requires `evm` and `bn254`
- `tangle-full` requires `tangle` and `bn254`
