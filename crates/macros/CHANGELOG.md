# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tangle-network/gadget/releases/tag/gadget-macros-v0.1.0) - 2025-01-23

### Added

- eigenlayer incredible squaring blueprint ([#43](https://github.com/tangle-network/gadget/pull/43))
- Add more networking tests, add CI potential fix
- tangle SDK
- Improve test utils, fix tests ([#34](https://github.com/tangle-network/gadget/pull/34))
- add cli to workspace ([#19](https://github.com/tangle-network/gadget/pull/19))
- integrate clients into macros
- use new contexts in macros
- macro migration

### Fixed

- *(context-derive)* get tests building
- get macro tests building
- evm job macro exports
- use sdk in macros
- eigensdk-rs git dep
- add extra args for networking
- change bind addr for IN_CI tests, add timeout
- *(keystore)* re-export gadget_crypto
- *(incredible-squaring)* uncomment metadata
- change incredible-squaring path in test
- remove old sdk paths from tests
- fix!(gadget-testing-utils): improved runner testing utilities for blueprints
- *(tests)* networking must run serially
- remove target addr from p2p client
- get proc macro tests building
- add cargo build to macro test
- get tests compiling
- get macro tests compiling
- get sdk main macro building
- get job macro building
- *(macros)* get TangleClientContext derive building
- *(runners)* get runners building
- get crates building again
- use `gadget_std` more
- get macro tests closer to building
- rename services client method
- update type paths in `job` macro

### Other

- *(keystore)* cleanup Tangle{Bls}Backend traits
- cleanup
- *(crypto)* remove crypto suffix from exports
- clippy
- cleanup job macro impl
- get incredible-squaring building
- clippy fixes, renaming
- fmt
- fmt
