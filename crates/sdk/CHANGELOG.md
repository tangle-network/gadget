# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tangle-network/gadget/releases/tag/blueprint-sdk-v0.1.0) - 2025-03-10

### Added

- re-integrate the blueprint configs ([#28](https://github.com/tangle-network/gadget/pull/28))
- debug macros, sdk crate ([#23](https://github.com/tangle-network/gadget/pull/23))
- new networking ([#664](https://github.com/tangle-network/gadget/pull/664))
- re-export `gadget_clients` from the sdk ([#655](https://github.com/tangle-network/gadget/pull/655))
- gadget workspace migration

### Fixed

- finish migration of new job system ([#699](https://github.com/tangle-network/gadget/pull/699))
- update some tests
- update blueprint examples ([#628](https://github.com/tangle-network/gadget/pull/628))

### Other

- remove old event listeners and runners
- rustdoc and READMEs for crates ([#27](https://github.com/tangle-network/gadget/pull/27))
- Generalize networking key type ([#685](https://github.com/tangle-network/gadget/pull/685))
- *(networking)* stop using `Box<dyn Error>` ([#657](https://github.com/tangle-network/gadget/pull/657))
- cleanup crate features & update `tangle-subxt` ([#642](https://github.com/tangle-network/gadget/pull/642))
- add descriptions to crates ([#616](https://github.com/tangle-network/gadget/pull/616))
