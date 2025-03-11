# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tangle-network/blueprint/releases/tag/blueprint-sdk-v0.1.0) - 2025-03-11

### Added

- re-integrate the blueprint configs ([#28](https://github.com/tangle-network/blueprint/pull/28))
- debug macros, sdk crate ([#23](https://github.com/tangle-network/blueprint/pull/23))
- new networking ([#664](https://github.com/tangle-network/blueprint/pull/664))
- re-export `gadget_clients` from the sdk ([#655](https://github.com/tangle-network/blueprint/pull/655))
- gadget workspace migration

### Fixed

- finish migration of new job system ([#699](https://github.com/tangle-network/blueprint/pull/699))
- update some tests
- update blueprint examples ([#628](https://github.com/tangle-network/blueprint/pull/628))

### Other

- remove `async-trait` ([#717](https://github.com/tangle-network/blueprint/pull/717))
- remove utils crates ([#714](https://github.com/tangle-network/blueprint/pull/714))
- *(clippy)* use workspace lints globally ([#710](https://github.com/tangle-network/blueprint/pull/710))
- remove old event listeners and runners
- rustdoc and READMEs for crates ([#27](https://github.com/tangle-network/blueprint/pull/27))
- Generalize networking key type ([#685](https://github.com/tangle-network/blueprint/pull/685))
- *(networking)* stop using `Box<dyn Error>` ([#657](https://github.com/tangle-network/blueprint/pull/657))
- cleanup crate features & update `tangle-subxt` ([#642](https://github.com/tangle-network/blueprint/pull/642))
- add descriptions to crates ([#616](https://github.com/tangle-network/blueprint/pull/616))
