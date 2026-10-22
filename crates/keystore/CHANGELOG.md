# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-alpha.15](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.2.0-alpha.14...blueprint-keystore-v0.2.0-alpha.15) - 2026-10-22

### Fixed

- preserve gossip clocks and current RNG APIs

## [0.2.0-alpha.14](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.2.0-alpha.13...blueprint-keystore-v0.2.0-alpha.14) - 2026-10-01

### Fixed

- *(keystore)* add sole_local and adopt it for operator identity selection (#1556)

## [0.2.0-alpha.13](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.2.0-alpha.12...blueprint-keystore-v0.2.0-alpha.13) - 2026-10-01

### Other

- updated the following local packages: blueprint-crypto

## [0.2.0-alpha.11](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.2.0-alpha.10...blueprint-keystore-v0.2.0-alpha.11) - 2026-09-29

### Other

- *(agents)* remove generated maps and preserve local contracts (#1513)

## [0.2.0-alpha.10](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.2.0-alpha.9...blueprint-keystore-v0.2.0-alpha.10) - 2026-08-02

### Fixed

- *(deps)* clear the gix advisories via cargo-generate; MSRV 1.91 -> 1.93 (#1484)

### Other

- remove EigenLayer support and fix two pre-existing build breakages (#1479)

## [0.2.0-alpha.9](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.2.0-alpha.8...blueprint-keystore-v0.2.0-alpha.9) - 2026-06-13

### Other

- update Cargo.toml dependencies

## [0.2.0-alpha.8](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.2.0-alpha.7...blueprint-keystore-v0.2.0-alpha.8) - 2026-05-21

### Other

- update Cargo.toml dependencies

## [0.2.0-alpha.7](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.2.0-alpha.6...blueprint-keystore-v0.2.0-alpha.7) - 2026-05-20

### Other

- updated the following local packages: blueprint-std, blueprint-crypto

## [0.2.0-alpha.6](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.2.0-alpha.5...blueprint-keystore-v0.2.0-alpha.6) - 2026-05-17

### Other

- updated the following local packages: blueprint-crypto

## [0.2.0-alpha.4](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.2.0-alpha.3...blueprint-keystore-v0.2.0-alpha.4) - 2026-04-22

### Other

- update Cargo.toml dependencies

### Removed

- drop Substrate keystore backend and legacy `tangle` feature flag in favor of the new `tangle` stack (EVM-only)

## [0.1.0-alpha.15](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.1.0-alpha.14...blueprint-keystore-v0.1.0-alpha.15) - 2025-10-02

### Other

- bump eigensdk & alloy version (#1164)

## [0.1.0-alpha.14](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.1.0-alpha.13...blueprint-keystore-v0.1.0-alpha.14) - 2025-08-11

### Fixed

- *(ci)* fix Clippy & build issues (#1125)

## [0.1.0-alpha.13](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.1.0-alpha.12...blueprint-keystore-v0.1.0-alpha.13) - 2025-08-06

### Other

- update Cargo.toml dependencies

## [0.1.0-alpha.12](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.1.0-alpha.11...blueprint-keystore-v0.1.0-alpha.12) - 2025-07-03

### Other

- update Cargo.toml dependencies

## [0.1.0-alpha.11](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.1.0-alpha.10...blueprint-keystore-v0.1.0-alpha.11) - 2025-07-03

### Added

- quality of service (#968)

## [0.1.0-alpha.10](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.1.0-alpha.9...blueprint-keystore-v0.1.0-alpha.10) - 2025-06-16

### Other

- update Cargo.toml dependencies

## [0.1.0-alpha.9](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.1.0-alpha.8...blueprint-keystore-v0.1.0-alpha.9) - 2025-05-14

### Other

- update Cargo.toml dependencies

## [0.1.0-alpha.8](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.1.0-alpha.7...blueprint-keystore-v0.1.0-alpha.8) - 2025-05-13

### Other

- updated the following local packages: blueprint-crypto

## [0.1.0-alpha.7](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.1.0-alpha.6...blueprint-keystore-v0.1.0-alpha.7) - 2025-05-09

### Other

- remove legacy name references (#967)

## [0.1.0-alpha.6](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.1.0-alpha.5...blueprint-keystore-v0.1.0-alpha.6) - 2025-05-01

### Other

- remove unused dependencies (#915)

## [0.1.0-alpha.5](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.1.0-alpha.4...blueprint-keystore-v0.1.0-alpha.5) - 2025-04-21

### Other

- update Cargo.toml dependencies

## [0.1.0-alpha.4](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.1.0-alpha.3...blueprint-keystore-v0.1.0-alpha.4) - 2025-04-15

### Fixed

- *(keystore)* only verify `FileStorage` paths that exist (#878)

## [0.1.0-alpha.3](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.1.0-alpha.2...blueprint-keystore-v0.1.0-alpha.3) - 2025-04-14

### Other

- update Cargo.toml dependencies

## [0.1.0-alpha.2](https://github.com/tangle-network/blueprint/compare/blueprint-keystore-v0.1.0-alpha.1...blueprint-keystore-v0.1.0-alpha.2) - 2025-04-11

### Other

- update Cargo.toml dependencies

## [0.1.0-alpha.1](https://github.com/tangle-network/blueprint/releases/tag/blueprint-keystore-v0.1.0-alpha.1) - 2025-04-08

### Other

- Initial release
