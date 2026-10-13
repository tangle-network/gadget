# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0-alpha.24](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.5.0-alpha.23...cargo-tangle-v0.5.0-alpha.24) - 2026-10-07

### Fixed

- *(cargo-tangle)* surface forge build output in the error instead of dropping it (#1571)
- *(harness)* read HTTP status codes instead of substring-matching responses (#1570)

## [0.5.0-alpha.23](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.5.0-alpha.22...cargo-tangle-v0.5.0-alpha.23) - 2026-10-02

### Added

- *(cargo-tangle)* add --init to blueprint create for in-place scaffolding (#1564)

## [0.5.0-alpha.22](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.5.0-alpha.21...cargo-tangle-v0.5.0-alpha.22) - 2026-10-02

### Other

- update Cargo.lock dependencies

## [0.5.0-alpha.21](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.5.0-alpha.20...cargo-tangle-v0.5.0-alpha.21) - 2026-10-01

### Fixed

- *(harness)* make operator shutdown actually graceful (SIGTERM first) (#1557)

## [0.5.0-alpha.20](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.5.0-alpha.19...cargo-tangle-v0.5.0-alpha.20) - 2026-10-01

### Fixed

- *(harness)* kill operator subprocesses when the harness test run ends (#1554)
- *(cargo-tangle)* fail closed when the keystore holds multiple ECDSA keys (#1549)

## [0.5.0-alpha.19](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.5.0-alpha.18...cargo-tangle-v0.5.0-alpha.19) - 2026-09-29

### Other

- update Cargo.lock dependencies

## [0.5.0-alpha.15](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.5.0-alpha.14...cargo-tangle-v0.5.0-alpha.15) - 2026-09-29

### Other

- update Cargo.toml dependencies

## [0.5.0-alpha.14](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.5.0-alpha.13...cargo-tangle-v0.5.0-alpha.14) - 2026-08-02

### Fixed

- *(testing)* regenerate the LocalTestnet Anvil fixture at tnt-core 0.19 (#1483)

### Other

- remove EigenLayer support and fix two pre-existing build breakages (#1479)

## [0.5.0-alpha.13](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.5.0-alpha.12...cargo-tangle-v0.5.0-alpha.13) - 2026-07-24

### Other

- update Cargo.lock dependencies

## [0.5.0-alpha.12](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.5.0-alpha.11...cargo-tangle-v0.5.0-alpha.12) - 2026-07-08

### Added

- adopt tnt-core 0.19 bindings (draft — blocked on publishing tnt-core-bindings 0.19.0) (#1467)

## [0.5.0-alpha.11](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.5.0-alpha.10...cargo-tangle-v0.5.0-alpha.11) - 2026-07-03

### Added

- *(tangle)* event-source blueprint display data (createBlueprint gas cut companion) (#1463)

## [0.5.0-alpha.10](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.5.0-alpha.9...cargo-tangle-v0.5.0-alpha.10) - 2026-06-13

### Other

- update Cargo.toml dependencies

## [0.5.0-alpha.8](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.5.0-alpha.7...cargo-tangle-v0.5.0-alpha.8) - 2026-05-22

### Added

- blueprint binary upgrade flow — CLI + manager watcher + ship wizard + local-ack + GH Action (#1434)

### Added

- *(cargo-tangle)* blueprint binary upgrade flow: `blueprint publish-version`,
  `set-active-version`, `deprecate-version`, `list-versions`, `show-version`,
  `trust-score`; `blueprint service set-policy`, `ack-version`,
  `effective-version`, `upgrade-status`; `attest submit`/`list`/`revoke`. Wraps
  the new tnt-core `BlueprintsBinaryVersions` + `BlueprintsBinaryAttestations`
  facets with local artifact hashing, optional IPFS pinning, and a CI-ready
  weighted trust score against `BlueprintAuditors`.
- *(cargo-tangle)* `cargo tangle blueprint ship` — one-command interactive release:
  detects the workspace, builds `cargo build --release`, hashes the binary,
  optionally pins to IPFS, optionally derives `attestationHash` from a
  sigstore/SLSA bundle, publishes via `publishBinaryVersion`, promotes via
  `setActiveBinaryVersion`, and (optionally) bulk-flips a list of services
  into AUTO policy. CI mode: `--yes --pin-ipfs --promote`.
- *(cargo-tangle)* MANUAL-with-assist local-authorization layer: new
  `service upgrades` / `upgrade-local` / `upgrade-whitelist` / `upgrade-skip`
  / `upgrade-authz` subcommands that drive the blueprint-manager's
  `/upgrades/*` RPC. Pre-authorizes the manager to swap into specific
  versions without writing an on-chain ack tx; on-chain policy stays MANUAL.
- *(blueprint-manager)* `LocalAuthzStore` persistence layer for the upgrade
  watcher. New `<data-dir>/upgrade-authz/<serviceId>.json` records carry
  whitelisted / pinned / skipped versions; the watcher honors them under
  the MANUAL policy branch (after the same sha256+attestation gate AUTO
  uses) and persists across manager restarts.
- *(ci)* `.github/actions/ship-release` composite action wraps
  `cargo tangle blueprint ship` for release-driven publishing flows plus an example
  workflow at `.github/workflows/example-release.yml`.

## [0.5.0-alpha.7](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.5.0-alpha.6...cargo-tangle-v0.5.0-alpha.7) - 2026-05-21

### Other

- update Cargo.lock dependencies

## [0.5.0-alpha.6](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.5.0-alpha.5...cargo-tangle-v0.5.0-alpha.6) - 2026-05-20

### Added

- *(cargo-tangle)* per-network built-in defaults for testnet/mainnet deploy (#1429)

## [0.5.0-alpha.5](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.5.0-alpha.4...cargo-tangle-v0.5.0-alpha.5) - 2026-05-17

### Other

- updated the following local packages: blueprint-crypto, blueprint-keystore, blueprint-client-tangle, blueprint-remote-providers, blueprint-runner, blueprint-clients, blueprint-chain-setup, blueprint-contexts, blueprint-eigenlayer-extra, blueprint-testing-utils, blueprint-manager

## [0.5.0-alpha.3](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.22...cargo-tangle-v0.5.0-alpha.3) - 2026-04-22

### Other

- Rename staking contract CLI surface from restaking (#1381)

## [0.4.0-alpha.23](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.22...cargo-tangle-v0.4.0-alpha.23) - 2025-10-02

### Other

- bump eigensdk & alloy version (#1164)

## [0.4.0-alpha.22](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.21...cargo-tangle-v0.4.0-alpha.22) - 2025-08-20

### Added

- *(manager)* [**breaking**] add optional container source support (#1085)

## [0.4.0-alpha.21](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.20...cargo-tangle-v0.4.0-alpha.21) - 2025-08-11

### Other

- update Cargo.toml dependencies

## [0.4.0-alpha.20](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.19...cargo-tangle-v0.4.0-alpha.20) - 2025-08-06

### Other

- update Cargo.toml dependencies

## [0.4.0-alpha.19](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.18...cargo-tangle-v0.4.0-alpha.19) - 2025-07-07

### Fixed

- typo (#1075)

## [0.4.0-alpha.18](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.17...cargo-tangle-v0.4.0-alpha.18) - 2025-07-03

### Added

- add rustls as default crypto provider (#1070)
- *(manager)* feature-gate vm sandbox (#1053)
- automate nftables NAT setup for VMs (#1021)
- quality of service (#968)

### Fixed

- use blueprint-core logging macros (#1062)
- remove git dependencies (#1056)
- gracefully handle AlreadyOperator error in blueprint registration (#1055)

## [0.4.0-alpha.17](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.16...cargo-tangle-v0.4.0-alpha.17) - 2025-06-18

### Added

- improve testing source fetcher and error handling (#1052)

## [0.4.0-alpha.16](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.15...cargo-tangle-v0.4.0-alpha.16) - 2025-06-16

### Added

- *(tangle-testing-utils)* support manager bridge in test harness (#1024)
- *(manager)* [**breaking**] allow spawning services without sandboxing (#1022)
- *(manager)* [**breaking**] add manager <-> service bridge (#969)
- *(cargo-tangle)* blueprint create without interaction (#996)
- add support for request params (#984)

### Fixed

- *(cargo-tangle)* missing template defaults (#999)

## [0.4.0-alpha.15](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.14...cargo-tangle-v0.4.0-alpha.15) - 2025-05-14

### Other

- update Cargo.toml dependencies

## [0.4.0-alpha.14](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.13...cargo-tangle-v0.4.0-alpha.14) - 2025-05-13

### Other

- update Cargo.toml dependencies

## [0.4.0-alpha.13](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.12...cargo-tangle-v0.4.0-alpha.13) - 2025-05-09

### Other

- remove legacy name references (#967)

## [0.4.0-alpha.12](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.11...cargo-tangle-v0.4.0-alpha.12) - 2025-05-07

### Added

- *(manager)* [**breaking**] support verifying binaries with gh attestations (#938)

### Other

- *(ci)* fixes from attestation pr (#964)

## [0.4.0-alpha.11](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.10...cargo-tangle-v0.4.0-alpha.11) - 2025-05-06

### Other

- updated the following local packages: blueprint-runner, blueprint-clients, blueprint-contexts, blueprint-chain-setup, blueprint-manager, blueprint-testing-utils, blueprint-testing-utils

## [0.4.0-alpha.10](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.9...cargo-tangle-v0.4.0-alpha.10) - 2025-05-01

### Added

- *(pricing-engine)* finalized pricing engine implementation (#904)

### Fixed

- *(cargo-tangle)* properly encode optional fields (#921)

### Other

- remove unused dependencies (#915)

## [0.4.0-alpha.9](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.8...cargo-tangle-v0.4.0-alpha.9) - 2025-04-22

### Fixed

- *(cargo-tangle)* stop always choosing eigenlayer bls template (#909)

## [0.4.0-alpha.8](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.7...cargo-tangle-v0.4.0-alpha.8) - 2025-04-21

### Added

- impl pricing engine diff network updates (#890)
- *(manager)* functional container sources (#883)

## [0.4.0-alpha.7](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.6...cargo-tangle-v0.4.0-alpha.7) - 2025-04-18

### Added

- allow for blueprint manage to take a keystore. (#888)
- Blueprint Runner MVP 0 (#881)

## [0.4.0-alpha.6](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.5...cargo-tangle-v0.4.0-alpha.6) - 2025-04-16

### Other

- updated the following local packages: blueprint-clients, blueprint-contexts, blueprint-chain-setup, blueprint-manager, blueprint-testing-utils, blueprint-testing-utils

## [0.4.0-alpha.5](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.4...cargo-tangle-v0.4.0-alpha.5) - 2025-04-15

### Fixed

- *(cargo-tangle)* construct a keystore for blueprint run (#882)

## [0.4.0-alpha.4](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.3...cargo-tangle-v0.4.0-alpha.4) - 2025-04-15

### Other

- updated the following local packages: blueprint-keystore, blueprint-keystore, blueprint-runner, blueprint-clients, blueprint-contexts, blueprint-chain-setup, blueprint-manager, blueprint-testing-utils, blueprint-testing-utils

## [0.4.0-alpha.3](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.2...cargo-tangle-v0.4.0-alpha.3) - 2025-04-14

### Other

- update Cargo.lock dependencies

## [0.4.0-alpha.2](https://github.com/tangle-network/blueprint/compare/cargo-tangle-v0.4.0-alpha.1...cargo-tangle-v0.4.0-alpha.2) - 2025-04-11

### Added

- *(tangle-extra)* [**breaking**] support multiple blueprint source types (#864)

## [0.4.0-alpha.1](https://github.com/tangle-network/blueprint/releases/tag/blueprint-metrics-v0.1.0-alpha.1) - 2025-04-08

### Other

- Initial release
