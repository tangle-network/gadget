# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tangle-network/blueprint/releases/tag/blueprint-core-v0.1.0) - 2025-03-14

### Added

- core extractors ([#36](https://github.com/tangle-network/blueprint/pull/36))
- EVM Consumers ([#30](https://github.com/tangle-network/blueprint/pull/30))
- allow jobs to return errors
- re-integrate the blueprint configs ([#28](https://github.com/tangle-network/blueprint/pull/28))
- reflection ([#16](https://github.com/tangle-network/blueprint/pull/16))
- add Tangle job layer
- add Tangle result consumer
- more EVM extractors ([#15](https://github.com/tangle-network/blueprint/pull/15))
- allow empty call returns
- EVM impl, contract and block events. ([#3](https://github.com/tangle-network/blueprint/pull/3))

### Fixed

- *(core)* change the return of `JobResult::body()` ([#722](https://github.com/tangle-network/blueprint/pull/722))
- finish migration of new job system ([#699](https://github.com/tangle-network/blueprint/pull/699))

### Other

- *(clippy)* use workspace lints globally ([#710](https://github.com/tangle-network/blueprint/pull/710))
- remove old event listeners and runners
- rustdoc and READMEs for crates ([#27](https://github.com/tangle-network/blueprint/pull/27))
