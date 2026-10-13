# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0-alpha.20](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.4.0-alpha.19...blueprint-manager-v0.4.0-alpha.20) - 2026-10-07

### Other

- update Cargo.lock dependencies

## [0.4.0-alpha.19](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.4.0-alpha.18...blueprint-manager-v0.4.0-alpha.19) - 2026-10-02

### Other

- update Cargo.lock dependencies

## [0.4.0-alpha.18](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.4.0-alpha.17...blueprint-manager-v0.4.0-alpha.18) - 2026-10-02

### Other

- update Cargo.lock dependencies

## [0.4.0-alpha.17](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.4.0-alpha.16...blueprint-manager-v0.4.0-alpha.17) - 2026-10-01

### Other

- update Cargo.lock dependencies

## [0.4.0-alpha.16](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.4.0-alpha.15...blueprint-manager-v0.4.0-alpha.16) - 2026-10-01

### Other

- update Cargo.lock dependencies

## [0.4.0-alpha.15](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.4.0-alpha.14...blueprint-manager-v0.4.0-alpha.15) - 2026-09-29

### Other

- update Cargo.lock dependencies

## [0.4.0-alpha.13](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.4.0-alpha.12...blueprint-manager-v0.4.0-alpha.13) - 2026-08-02

### Fixed

- *(deps)* bollard 0.20 + testcontainers 0.27.3 to clear the tokio-tar advisory (#1482)

### Other

- remove EigenLayer support and fix two pre-existing build breakages (#1479)

## [0.4.0-alpha.12](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.4.0-alpha.11...blueprint-manager-v0.4.0-alpha.12) - 2026-07-08

### Added

- adopt tnt-core 0.19 bindings (draft — blocked on publishing tnt-core-bindings 0.19.0) (#1467)

### Fixed

- *(manager)* event-source blueprint sources for tnt-core 0.19 (#1468)

## [0.4.0-alpha.11](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.4.0-alpha.10...blueprint-manager-v0.4.0-alpha.11) - 2026-07-03

### Other

- update Cargo.lock dependencies

## [0.4.0-alpha.10](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.4.0-alpha.9...blueprint-manager-v0.4.0-alpha.10) - 2026-06-13

### Added

- *(remote-providers)* verify TEE attestations fail closed (#1453)

## [0.4.0-alpha.8](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.4.0-alpha.7...blueprint-manager-v0.4.0-alpha.8) - 2026-05-22

### Added

- blueprint binary upgrade flow — CLI + manager watcher + ship wizard + local-ack + GH Action (#1434)

## [0.4.0-alpha.7](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.4.0-alpha.6...blueprint-manager-v0.4.0-alpha.7) - 2026-05-21

### Other

- update Cargo.toml dependencies

## [0.4.0-alpha.6](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.4.0-alpha.5...blueprint-manager-v0.4.0-alpha.6) - 2026-05-20

### Other

- updated the following local packages: blueprint-core, blueprint-std, blueprint-faas, blueprint-crypto, blueprint-auth, blueprint-keystore, blueprint-client-tangle, blueprint-manager-bridge, blueprint-pricing-engine, blueprint-remote-providers, blueprint-qos, blueprint-runner, blueprint-clients, blueprint-chain-setup, blueprint-eigenlayer-extra, blueprint-testing-utils

## [0.4.0-alpha.5](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.4.0-alpha.4...blueprint-manager-v0.4.0-alpha.5) - 2026-05-17

### Other

- updated the following local packages: blueprint-crypto, blueprint-auth, blueprint-keystore, blueprint-client-tangle, blueprint-manager-bridge, blueprint-pricing-engine, blueprint-remote-providers, blueprint-qos, blueprint-runner, blueprint-clients, blueprint-chain-setup, blueprint-eigenlayer-extra, blueprint-testing-utils

## [0.4.0-alpha.3](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.4.0-alpha.2...blueprint-manager-v0.4.0-alpha.3) - 2026-04-22

### Added

- security hardening + staking_contract rename + alpha.3 (#1380)
- harness system + x402 E2E + chain target flags (#1379)
- *(cargo-tangle)* dev up/down workspace + fix silent Hetzner/Crusoe provider bug (#1376)
- one-click local Tangle stack script

### Fixed

- *(tangle)* cap log queries and add gas fallbacks (#1382)

## [0.3.0-alpha.23](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.22...blueprint-manager-v0.3.0-alpha.23) - 2025-11-03

### Other

- update Cargo.toml dependencies

## [0.3.0-alpha.22](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.21...blueprint-manager-v0.3.0-alpha.22) - 2025-10-30

### Other

- Add Eigenlayer CLI commands and integrate with blueprint manager (#1193)

## [0.3.0-alpha.21](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.20...blueprint-manager-v0.3.0-alpha.21) - 2025-10-02

### Added

- *(auth)* add mTLS support for authenticated proxy (#1156)

### Other

- bump eigensdk & alloy version (#1164)
- logging import cleanup (#1153)

## [0.3.0-alpha.20](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.19...blueprint-manager-v0.3.0-alpha.20) - 2025-08-20

### Added

- *(manager)* [**breaking**] add optional container source support (#1085)

## [0.3.0-alpha.19](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.18...blueprint-manager-v0.3.0-alpha.19) - 2025-08-11

### Fixed

- *(ci)* fix Clippy & build issues (#1125)

## [0.3.0-alpha.18](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.17...blueprint-manager-v0.3.0-alpha.18) - 2025-08-06

### Other

- update Cargo.toml dependencies

## [0.3.0-alpha.17](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.16...blueprint-manager-v0.3.0-alpha.17) - 2025-07-07

### Other

- update Cargo.toml dependencies

## [0.3.0-alpha.16](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.15...blueprint-manager-v0.3.0-alpha.16) - 2025-07-03

### Added

- *(manager)* feature-gate vm sandbox (#1053)
- automate nftables NAT setup for VMs (#1021)
- quality of service (#968)

## [0.3.0-alpha.15](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.14...blueprint-manager-v0.3.0-alpha.15) - 2025-06-18

### Added

- improve testing source fetcher and error handling (#1052)

## [0.3.0-alpha.14](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.13...blueprint-manager-v0.3.0-alpha.14) - 2025-06-16

### Added

- *(tangle-testing-utils)* support manager bridge in test harness (#1024)
- *(manager)* [**breaking**] allow spawning services without sandboxing (#1022)
- *(manager)* [**breaking**] add manager <-> service bridge (#969)
- *(cargo-tangle)* blueprint create without interaction (#996)
- add auth proxy to blueprint manager (#994)

### Other

- *(blueprint-manager-bridge)* add description (#1044)

## [0.3.0-alpha.13](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.12...blueprint-manager-v0.3.0-alpha.13) - 2025-05-14

### Other

- update Cargo.toml dependencies

## [0.3.0-alpha.12](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.11...blueprint-manager-v0.3.0-alpha.12) - 2025-05-13

### Other

- update Cargo.toml dependencies

## [0.3.0-alpha.11](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.10...blueprint-manager-v0.3.0-alpha.11) - 2025-05-09

### Fixed

- *(manager)* correct platform check (#970)

### Other

- remove legacy name references (#967)

## [0.3.0-alpha.10](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.9...blueprint-manager-v0.3.0-alpha.10) - 2025-05-07

### Added

- *(manager)* [**breaking**] support verifying binaries with gh attestations (#938)

### Other

- *(ci)* fixes from attestation pr (#964)

## [0.3.0-alpha.9](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.8...blueprint-manager-v0.3.0-alpha.9) - 2025-05-06

### Other

- updated the following local packages: blueprint-networking, blueprint-runner, blueprint-clients

## [0.3.0-alpha.8](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.7...blueprint-manager-v0.3.0-alpha.8) - 2025-05-01

### Fixed

- *(manager)* handle github sources (#922)
- *(manager)* set correct host for local endpoints (#919)

### Other

- *(runner)* document crate (#920)
- remove unused dependencies (#915)

## [0.3.0-alpha.7](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.6...blueprint-manager-v0.3.0-alpha.7) - 2025-04-21

### Added

- *(manager)* functional container sources (#883)

## [0.3.0-alpha.6](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.5...blueprint-manager-v0.3.0-alpha.6) - 2025-04-18

### Added

- allow for blueprint manage to take a keystore. (#888)
- Blueprint Runner MVP 0 (#881)

## [0.3.0-alpha.5](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.4...blueprint-manager-v0.3.0-alpha.5) - 2025-04-16

### Other

- updated the following local packages: blueprint-clients

## [0.3.0-alpha.4](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.3...blueprint-manager-v0.3.0-alpha.4) - 2025-04-15

### Other

- updated the following local packages: blueprint-keystore, blueprint-runner, blueprint-clients

## [0.3.0-alpha.3](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.2...blueprint-manager-v0.3.0-alpha.3) - 2025-04-14

### Added

- *(manager)* support multiple sources (#866)

## [0.3.0-alpha.2](https://github.com/tangle-network/blueprint/compare/blueprint-manager-v0.3.0-alpha.1...blueprint-manager-v0.3.0-alpha.2) - 2025-04-11

### Added

- *(tangle-extra)* [**breaking**] support multiple blueprint source types (#864)
