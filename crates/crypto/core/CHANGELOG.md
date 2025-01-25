# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tangle-network/gadget/releases/tag/gadget-crypto-core-v0.1.0) - 2025-01-25

### Added

- add cli to workspace ([#19](https://github.com/tangle-network/gadget/pull/19))
- separate key types entirely from keystore/put into crypto crates

### Fixed

- no-std tangle-pair-signer
- correctly de/serialize keys in keystore

### Other

- cleanup networking, get clippy --tests for networking passing ([#10](https://github.com/tangle-network/gadget/pull/10))
- get networking building
