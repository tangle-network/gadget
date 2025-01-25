# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.2](https://github.com/tangle-network/gadget/compare/gadget-blueprint-proc-macro-v0.5.1...gadget-blueprint-proc-macro-v0.5.2) - 2025-01-25

### Added

- *(macros)* use `sdk::Error` instead of `Box<dyn Error>` ([#604](https://github.com/tangle-network/gadget/pull/604))
- eigenlayer incredible squaring blueprint ([#43](https://github.com/tangle-network/gadget/pull/43))
- Add more networking tests, add CI potential fix
- Improve test utils, fix tests ([#34](https://github.com/tangle-network/gadget/pull/34))
- use new contexts in macros
- macro migration

### Fixed

- get macro tests building
- evm job macro exports
- use sdk in macros
- fix!(gadget-testing-utils): improved runner testing utilities for blueprints
- get proc macro tests building
- get macro tests compiling
- get sdk main macro building
- get job macro building
- get crates building again
- use `gadget_std` more
- update type paths in `job` macro

### Other

- *(keystore)* cleanup Tangle{Bls}Backend traits
- cleanup
- clippy
- cleanup job macro impl
- get incredible-squaring building
- fmt
- fmt
