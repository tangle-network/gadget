# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tangle-network/gadget/releases/tag/gadget-networking-v0.1.0) - 2025-01-25

### Added

- Add more networking tests, add CI potential fix
- separate key types entirely from keystore/put into crypto crates

### Fixed

- remove serial test dep
- *(pipeline)* Use MacOS Apple Silicon runner for network tests
- change bind addr for IN_CI tests, add timeout
- *(networking)* use `serial_test`
- use `gadget_std` more
- *(networking)* bugs squashed, tests passing, k256 now working ([#12](https://github.com/tangle-network/gadget/pull/12))

### Other

- clean up refs to std
- fixes
- *(crypto)* remove crypto suffix from exports
- clippy fixes, renaming
- merge `gadget-tokio-std` into `gadget-std`
- clippy
- fix unused imports
- cleanup networking, get clippy --tests for networking passing ([#10](https://github.com/tangle-network/gadget/pull/10))
- test instrumented client and others ([#9](https://github.com/tangle-network/gadget/pull/9))
- workspace builds
- get networking building
- working off donovan/contexts, adding networking, logging, keystore change
