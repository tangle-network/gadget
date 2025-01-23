# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tangle-network/gadget/releases/tag/gadget-keystore-v0.1.0) - 2025-01-23

### Added

- eigenlayer incredible squaring blueprint ([#43](https://github.com/tangle-network/gadget/pull/43))
- runner test utilities and cleanup
- migrate blueprint-manager
- add cli to workspace ([#19](https://github.com/tangle-network/gadget/pull/19))
- Test all key types
- implement runners ([#11](https://github.com/tangle-network/gadget/pull/11))
- separate key types entirely from keystore/put into crypto crates
- *(gadget-contexts)* contexts, clients, and features
- add `KeystoreConfig`, automatic storage registration
- add BLS377
- add feature for `bls377`
- add Result type alias
- use more concrete errors when possible
- enable `doc_auto_cfg`
- update `thiserror` for no_std
- keystore compiling, starting on config
- initial working feature separation
- get building, start on keystore traits
- add std and keystore initial impl

### Fixed

- *(keystore)* allow warnings when no features enabled
- get macro tests building
- eigensdk-rs git dep
- add extra args for networking
- get Tangle tests passing again
- *(keystore)* get tangle bls tests passing
- properly handle de/serialization of sp-core BLS pairs
- *(keystore)* fix test
- *(gadget-client-eigenlayer)* test fix
- *(incredible-squaring)* fix ecdsa generate from string
- *(keystore)* re-export gadget_crypto
- *(incredible-squaring)* debugging blueprint test
- *(testing-utils)* debugging blueprint tests
- correctly de/serialize keys in keystore
- rename SchnorrkelEcdsa
- cleanup a bunch of warnings
- needless unwraps
- reorder trait impls
- re-export eigen BLS backend
- stop exporting sp_core macros

### Other

- clean up refs to std
- *(keystore)* cleanup Tangle{Bls}Backend traits
- cleanup, fix tangle bls tests
- cleanup
- *(clippy)* fmt
- *(crypto)* remove crypto suffix from exports
- *(keystore)* splitup local operations tests
- clippy
- debugging blueprint
- clippy
- workspace builds
- get networking building
- working off donovan/contexts, adding networking, logging, keystore change
- document `KeystoreConfig`
- add docs for storage
- cleanup key iter methods
- add fs keystore test
- use `KeyTypeId` for storage
- remove unused imports
- reorganize backends
- condense backing storage types
- condense `remote` module
- remove unused imports
- use `assert_ne!`
- updates
