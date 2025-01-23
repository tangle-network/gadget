# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tangle-network/gadget/releases/tag/gadget-clients-v0.1.0) - 2025-01-23

### Added

- eigenlayer incredible squaring blueprint ([#43](https://github.com/tangle-network/gadget/pull/43))
- Improve test utils, fix tests ([#34](https://github.com/tangle-network/gadget/pull/34))
- add cli to workspace ([#19](https://github.com/tangle-network/gadget/pull/19))
- integrate clients into macros
- complete TangleClient, generalize services, contexts
- implement runners ([#11](https://github.com/tangle-network/gadget/pull/11))
- separate key types entirely from keystore/put into crypto crates
- *(gadget-contexts)* contexts, clients, and features
- *(gadget-contexts)* wip implementation of contexts
- add tokio-std, bindings rename, config build

### Fixed

- eigenlayer blueprint test fix
- *(ci)* workflow fix
- eigensdk-rs git dep
- use url 2.5.2
- *(gadget-client-evm)* fix test race condition
- set test mode in harness env
- get Tangle tests passing again
- *(gadget-client-eigenlayer)* test fix
- *(incredible-squaring)* debugging blueprint test
- *(testing-utils)* debugging blueprint tests
- correctly de/serialize keys in keystore
- *(incredible-squaring)* progress on blueprint test and utilized crates
- rename anvil test utils
- remove target addr from p2p client
- [**breaking**] refactoring test runners wip
- get tests compiling
- *(macros)* get TangleClientContext derive building
- use `gadget_std` more
- get macro tests closer to building
- update type paths in `job` macro
- *(gadget-client-eigenlayer)* stop dropping anvil containers in tests
- make some methods public
- *(gadget-client-tangle)* remove `no_std` feature
- cleanup a bunch of warnings

### Other

- fmt
- add fake test to tangle client
- *(keystore)* cleanup Tangle{Bls}Backend traits
- *(clippy)* fmt
- clippy fixes, renaming
- *(blueprint-manager)* lots of cleanup
- fix clippy
- merge `gadget-tokio-std` into `gadget-std`
- *(gadget-client-eigenlayer)* move tests to new file
- clippy
- test instrumented client and others ([#9](https://github.com/tangle-network/gadget/pull/9))
- workspace builds
- working off donovan/contexts, adding networking, logging, keystore change
- *(clippy)* fmt
- updates
