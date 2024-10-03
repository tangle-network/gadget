# Key Management with `GenericKeyStore`

This document provides detailed instructions on how to generate and manage keys using the `GenericKeyStore` implementation in the `gadget-sdk` crate. Proper key management is crucial for ensuring the security and integrity of cryptographic operations within your application.

## Why `GenericKeyStore` is Needed

The `GenericKeyStore` provides a unified interface for managing cryptographic keys across different backends, such as in-memory and filesystem-based storage. This abstraction allows for flexibility in key management, making it easier to switch between different storage mechanisms without changing the core logic of your application.
Notably, it allows seamless intercompatibility between `sp_core` and the `sp_core` used in `subxt`. Finally, it ensures consistency and compatibility between the different key types and algorithms supported by the `gadget-sdk` crate.

## Key Insertion

The `GenericKeyStore` supports generating keys for various cryptographic algorithms, including `sr25519`, `ecdsa`, and `ed25519`. Below are examples of how to generate keys for each supported algorithm. We recommend the use of `create_sr25519_from_pair`, `create_ecdsa_from_pair`, and `create_ed25519_from_pair` to insert keys into the keystore, instead of using the `sr25519_generate_new`, `ecdsa_generate_new`, and `ed25519_generate_new` functions.

### Inserting `sr25519` Key Pair

```rust
use crate::keystore::GenericKeyStore;
use sp_core::sr25519;

let keystore = GenericKeyStore::Mem(Default::default());
let pair = sr25519::Pair::generate_from_string("//Alice", None).expect("Failed to generate sr25519 key pair");
let signer = keystore.create_sr25519_from_pair(pair).expect("Failed to create sr25519 key pair");
```

### Inserting `ecdsa` Key Pair

```rust
use crate::keystore::GenericKeyStore;
use sp_core::ecdsa;

let keystore = GenericKeyStore::Mem(Default::default());
let pair = ecdsa::Pair::generate_from_string("//Alice", None).expect("Failed to generate ecdsa key pair");
let signer = keystore.create_ecdsa_from_pair(pair).expect("Failed to create ecdsa key pair");
```

### Inserting `ed25519` Key Pair

```rust
use crate::keystore::GenericKeyStore;
use sp_core::ed25519;

let keystore = GenericKeyStore::Mem(Default::default());
let pair = ed25519::Pair::generate_from_string("//Alice", None).expect("Failed to generate ed25519 key pair");
let signer = keystore.create_ed25519_from_pair(pair).expect("Failed to create ed25519 key pair");
```

### Accessing ecdsa alloy keys
    
```rust
use crate::keystore::GenericKeyStore;
use sp_core::ecdsa;

let keystore = GenericKeyStore::Mem(Default::default());
let pair = ecdsa::Pair::generate_from_string("//Alice", None).expect("Failed to generate ecdsa key pair");
let signer = keystore.create_ecdsa_from_pair(pair).expect("Failed to create ecdsa key pair");
let alloy_key = signer.alloy_key().expect("Failed to get alloy key");
```
