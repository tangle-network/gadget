# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-alpha.16](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.2.0-alpha.15...blueprint-auth-v0.2.0-alpha.16) - 2026-10-10

### Other

- update Cargo.lock dependencies

## [0.2.0-alpha.15](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.2.0-alpha.14...blueprint-auth-v0.2.0-alpha.15) - 2026-10-02

### Other

- update Cargo.lock dependencies

## [0.2.0-alpha.14](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.2.0-alpha.13...blueprint-auth-v0.2.0-alpha.14) - 2026-10-01

### Other

- updated the following local packages: blueprint-crypto

## [0.2.0-alpha.13](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.2.0-alpha.12...blueprint-auth-v0.2.0-alpha.13) - 2026-09-29

### Other

- update Cargo.lock dependencies

## [0.2.0-alpha.12](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.2.0-alpha.11...blueprint-auth-v0.2.0-alpha.12) - 2026-09-29

### Other

- *(agents)* remove generated maps and preserve local contracts (#1513)

## [0.2.0-alpha.11](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.2.0-alpha.10...blueprint-auth-v0.2.0-alpha.11) - 2026-08-02

### Fixed

- *(deps)* clear the gix advisories via cargo-generate; MSRV 1.91 -> 1.93 (#1484)

### Other

- remove EigenLayer support and fix two pre-existing build breakages (#1479)

## [0.2.0-alpha.10](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.2.0-alpha.9...blueprint-auth-v0.2.0-alpha.10) - 2026-07-08

### Other

- update Cargo.toml dependencies

## [0.2.0-alpha.9](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.2.0-alpha.8...blueprint-auth-v0.2.0-alpha.9) - 2026-07-03

### Added

- *(tangle)* event-source blueprint display data (createBlueprint gas cut companion) (#1463)

## [0.2.0-alpha.8](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.2.0-alpha.7...blueprint-auth-v0.2.0-alpha.8) - 2026-06-13

### Other

- update Cargo.toml dependencies

## [0.2.0-alpha.7](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.2.0-alpha.6...blueprint-auth-v0.2.0-alpha.7) - 2026-05-21

### Other

- update Cargo.lock dependencies

## [0.2.0-alpha.6](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.2.0-alpha.5...blueprint-auth-v0.2.0-alpha.6) - 2026-05-20

### Other

- updated the following local packages: blueprint-core, blueprint-std, blueprint-crypto

## [0.2.0-alpha.5](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.2.0-alpha.4...blueprint-auth-v0.2.0-alpha.5) - 2026-05-17

### Other

- updated the following local packages: blueprint-crypto

## [0.1.0-alpha.10](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.1.0-alpha.9...blueprint-auth-v0.1.0-alpha.10) - 2025-10-02

### Added

- *(auth)* add mTLS support for authenticated proxy (#1156)
- *(auth)* add grpc proxy (#1155)
- add oauth and example blueprints (#1140)

### Other

- logging import cleanup (#1153)

## [0.1.0-alpha.9](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.1.0-alpha.8...blueprint-auth-v0.1.0-alpha.9) - 2025-08-20

### Added

- add multi tenancy auth (#1117)

## [0.1.0-alpha.8](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.1.0-alpha.7...blueprint-auth-v0.1.0-alpha.8) - 2025-08-11

### Fixed

- *(ci)* fix Clippy & build issues (#1125)

## [0.1.0-alpha.7](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.1.0-alpha.6...blueprint-auth-v0.1.0-alpha.7) - 2025-08-06

### Added

- standalone auth proxy (#1103)

## [0.1.0-alpha.6](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.1.0-alpha.5...blueprint-auth-v0.1.0-alpha.6) - 2025-07-07

### Other

- update Cargo.toml dependencies

## [0.1.0-alpha.5](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.1.0-alpha.4...blueprint-auth-v0.1.0-alpha.5) - 2025-07-03

### Added

- *(blueprint-auth)* enhance request tracing, CORS and security headers (#1060)

## [0.1.0-alpha.4](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.1.0-alpha.3...blueprint-auth-v0.1.0-alpha.4) - 2025-06-18

### Other

- update Cargo.toml dependencies

## [0.1.0-alpha.3](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.1.0-alpha.2...blueprint-auth-v0.1.0-alpha.3) - 2025-06-16

### Added

- *(tangle-testing-utils)* support manager bridge in test harness (#1024)
- *(manager)* [**breaking**] add manager <-> service bridge (#969)
- add auth proxy to blueprint manager (#994)

### Fixed

- change the schnorrkel ctx to substrate instead of tangle (#1026)
- *(auth)* use `blueprint_std` rand (#1023)

## [0.1.0-alpha.2](https://github.com/tangle-network/blueprint/compare/blueprint-auth-v0.1.0-alpha.1...blueprint-auth-v0.1.0-alpha.2) - 2025-05-14

### Other

- release (#980)

## [0.1.0-alpha.1](https://github.com/tangle-network/blueprint/releases/tag/blueprint-auth-v0.1.0-alpha.1) - 2025-05-13

### Added

- auth proxy (#962)
