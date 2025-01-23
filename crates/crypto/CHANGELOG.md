# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tangle-network/gadget/releases/tag/gadget-crypto-v0.1.0) - 2025-01-23

### Added

- eigenlayer incredible squaring blueprint ([#43](https://github.com/tangle-network/gadget/pull/43))
- [**breaking**] incredible squaring blueprint and test wip
- migrate blueprint-manager
- add cli to workspace ([#19](https://github.com/tangle-network/gadget/pull/19))
- use new contexts in macros
- Test all key types
- implement runners ([#11](https://github.com/tangle-network/gadget/pull/11))
- add event listeners
- separate key types entirely from keystore/put into crypto crates

### Fixed

- revert no-std TanglePairSigner
- no-std tangle-pair-signer
- properly handle de/serialization of sp-core BLS pairs
- *(gadget-client-eigenlayer)* test fix
- *(networking)* fixed serialization
- *(testing-utils)* debugging blueprint tests
- correctly de/serialize keys in keystore
- *(crypto)* properly deserialize sp-core sr25519
- *(runners)* get runners building
- get crates building again
- update type paths in `job` macro

### Other

- fix tests
- cleanup
- *(crypto)* remove crypto suffix from exports
- *(crypto)* flatten crates
- clippy fixes, renaming
- clippy
- fix unused imports
- cleanup networking, get clippy --tests for networking passing ([#10](https://github.com/tangle-network/gadget/pull/10))
- get networking building
- working off donovan/contexts, adding networking, logging, keystore change
