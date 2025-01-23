# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tangle-network/gadget/releases/tag/gadget-client-tangle-v0.1.0) - 2025-01-23

### Added

- Improve test utils, fix tests ([#34](https://github.com/tangle-network/gadget/pull/34))
- integrate clients into macros
- complete TangleClient, generalize services, contexts
- implement runners ([#11](https://github.com/tangle-network/gadget/pull/11))

### Fixed

- set test mode in harness env
- get Tangle tests passing again
- *(incredible-squaring)* debugging blueprint test
- *(testing-utils)* debugging blueprint tests
- correctly de/serialize keys in keystore
- *(incredible-squaring)* progress on blueprint test and utilized crates
- *(macros)* get TangleClientContext derive building
- make some methods public
- *(gadget-client-tangle)* remove `no_std` feature

### Other

- fmt
- add fake test to tangle client
- *(keystore)* cleanup Tangle{Bls}Backend traits
- *(clippy)* fmt
- *(blueprint-manager)* lots of cleanup
- merge `gadget-tokio-std` into `gadget-std`
- clippy
- workspace builds
