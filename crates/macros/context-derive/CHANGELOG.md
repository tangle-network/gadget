# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.2](https://github.com/tangle-network/gadget/compare/gadget-context-derive-v0.3.1...gadget-context-derive-v0.3.2) - 2025-01-23

### Added

- eigenlayer incredible squaring blueprint ([#43](https://github.com/tangle-network/gadget/pull/43))
- Add more networking tests, add CI potential fix
- tangle SDK
- integrate clients into macros
- use new contexts in macros
- macro migration

### Fixed

- *(context-derive)* get tests building
- use sdk in macros
- eigensdk-rs git dep
- add extra args for networking
- change bind addr for IN_CI tests, add timeout
- remove old sdk paths from tests
- remove target addr from p2p client
- get tests compiling
- get macro tests compiling
- get job macro building
- *(macros)* get TangleClientContext derive building
- *(runners)* get runners building
- use `gadget_std` more
- get macro tests closer to building
- rename services client method

### Other

- *(keystore)* cleanup Tangle{Bls}Backend traits
- *(crypto)* remove crypto suffix from exports
- cleanup job macro impl
- clippy fixes, renaming

## [0.3.1](https://github.com/tangle-network/gadget/compare/gadget-context-derive-v0.3.0...gadget-context-derive-v0.3.1) - 2024-12-11

### Other

- Download Tangle binary against specific hash ([#537](https://github.com/tangle-network/gadget/pull/537))
- Call ID Insertion and Resolution For [#520](https://github.com/tangle-network/gadget/pull/520) ([#533](https://github.com/tangle-network/gadget/pull/533))

## [0.3.0](https://github.com/tangle-network/gadget/compare/gadget-context-derive-v0.2.2...gadget-context-derive-v0.3.0) - 2024-11-29

### Added

- add MPCContext derive + test utils refactor ([#497](https://github.com/tangle-network/gadget/pull/497))

### Other

- [**breaking**] update `eigensdk` ([#506](https://github.com/tangle-network/gadget/pull/506))
- *(gadget-sdk)* [**breaking**] update to latest tangle ([#503](https://github.com/tangle-network/gadget/pull/503))

## [0.2.2](https://github.com/tangle-network/gadget/compare/gadget-context-derive-v0.2.1...gadget-context-derive-v0.2.2) - 2024-11-20

### Added

- add more service ctx methods for Tangle ([#477](https://github.com/tangle-network/gadget/pull/477))

## [0.2.1](https://github.com/tangle-network/gadget/compare/gadget-context-derive-v0.2.0...gadget-context-derive-v0.2.1) - 2024-11-16

### Added

- improved eigenlayer context and testing ([#453](https://github.com/tangle-network/gadget/pull/453))

## [0.2.0](https://github.com/tangle-network/gadget/compare/gadget-context-derive-v0.1.3...gadget-context-derive-v0.2.0) - 2024-11-05

### Added

- symbiotic initial integration ([#411](https://github.com/tangle-network/gadget/pull/411))

### Fixed

- *(sdk)* [**breaking**] allow for zero-based `blueprint_id` ([#426](https://github.com/tangle-network/gadget/pull/426))

### Other

- Continue Improving Event Flows ([#399](https://github.com/tangle-network/gadget/pull/399))
- improve blueprint-manager and blueprint-test-utils ([#421](https://github.com/tangle-network/gadget/pull/421))

## [0.1.3](https://github.com/tangle-network/gadget/compare/gadget-context-derive-v0.1.2...gadget-context-derive-v0.1.3) - 2024-10-25

### Other

- Leverage blueprint in incredible squaring aggregator ([#365](https://github.com/tangle-network/gadget/pull/365))

## [0.1.2](https://github.com/tangle-network/gadget/compare/gadget-context-derive-v0.1.1...gadget-context-derive-v0.1.2) - 2024-10-24

### Other

- updated the following local packages: gadget-sdk

## [0.1.1](https://github.com/tangle-network/gadget/compare/gadget-context-derive-v0.1.0...gadget-context-derive-v0.1.1) - 2024-09-30

### Added

- add ServicesContext Extension ([#321](https://github.com/tangle-network/gadget/pull/321))
- add EVM Provider and Tangle Client Context Extensions ([#319](https://github.com/tangle-network/gadget/pull/319))
