# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-alpha.17](https://github.com/tangle-network/blueprint/compare/blueprint-tangle-extra-v0.2.0-alpha.16...blueprint-tangle-extra-v0.2.0-alpha.17) - 2026-10-07

### Fixed

- *(clients)* submit_job_from_quote parsed the wrong event — plus the live chain proof (#1568)

### Other

- *(tangle)* live quote generator for the cross-SDK interop proof (#1567)
- *(tangle)* pin the canonical marketplace quote digest (#1565)

## [0.2.0-alpha.16](https://github.com/tangle-network/blueprint/compare/blueprint-tangle-extra-v0.2.0-alpha.15...blueprint-tangle-extra-v0.2.0-alpha.16) - 2026-10-01

### Fixed

- *(keystore)* add sole_local and adopt it for operator identity selection (#1556)

## [0.2.0-alpha.15](https://github.com/tangle-network/blueprint/compare/blueprint-tangle-extra-v0.2.0-alpha.14...blueprint-tangle-extra-v0.2.0-alpha.15) - 2026-10-01

### Other

- updated the following local packages: blueprint-crypto, blueprint-keystore, blueprint-client-tangle, blueprint-networking, blueprint-networking-agg-sig-gossip-extension, blueprint-tangle-aggregation-svc

## [0.2.0-alpha.14](https://github.com/tangle-network/blueprint/compare/blueprint-tangle-extra-v0.2.0-alpha.13...blueprint-tangle-extra-v0.2.0-alpha.14) - 2026-09-29

### Other

- updated the following local packages: blueprint-client-tangle, blueprint-tangle-aggregation-svc

## [0.2.0-alpha.13](https://github.com/tangle-network/blueprint/compare/blueprint-tangle-extra-v0.2.0-alpha.12...blueprint-tangle-extra-v0.2.0-alpha.13) - 2026-09-29

### Fixed

- *(aggregation)* release waiting operators after submission (#1503)
- *(tangle)* propagate aggregate submission failures (#1501)
- *(aggregation)* match chain quorum rounding (#1499)
- *(tangle)* fail closed on aggregation policy errors (#1497)
- *(tangle)* reject degenerate aggregation contexts (#1496)
- *(tangle)* align BLS aggregation with TNT 0.19 (#1495)

### Other

- *(agents)* remove generated maps and preserve local contracts (#1513)

## [0.2.0-alpha.12](https://github.com/tangle-network/blueprint/compare/blueprint-tangle-extra-v0.2.0-alpha.11...blueprint-tangle-extra-v0.2.0-alpha.12) - 2026-08-02

### Other

- updated the following local packages: blueprint-std, blueprint-keystore, blueprint-tangle-aggregation-svc, blueprint-crypto-core, blueprint-crypto-bn254, blueprint-crypto, blueprint-client-tangle, blueprint-networking, blueprint-networking-agg-sig-gossip-extension

## [0.2.0-alpha.11](https://github.com/tangle-network/blueprint/compare/blueprint-tangle-extra-v0.2.0-alpha.10...blueprint-tangle-extra-v0.2.0-alpha.11) - 2026-07-08

### Other

- update Cargo.toml dependencies

## [0.2.0-alpha.10](https://github.com/tangle-network/blueprint/compare/blueprint-tangle-extra-v0.2.0-alpha.9...blueprint-tangle-extra-v0.2.0-alpha.10) - 2026-07-03

### Added

- *(tangle)* event-source blueprint display data (createBlueprint gas cut companion) (#1463)

## [0.2.0-alpha.9](https://github.com/tangle-network/blueprint/compare/blueprint-tangle-extra-v0.2.0-alpha.8...blueprint-tangle-extra-v0.2.0-alpha.9) - 2026-06-13

### Other

- updated the following local packages: blueprint-crypto-bn254, blueprint-keystore, blueprint-networking, blueprint-networking-agg-sig-gossip-extension, blueprint-tangle-aggregation-svc, blueprint-crypto, blueprint-client-tangle

## [0.2.0-alpha.8](https://github.com/tangle-network/blueprint/compare/blueprint-tangle-extra-v0.2.0-alpha.7...blueprint-tangle-extra-v0.2.0-alpha.8) - 2026-05-21

### Other

- updated the following local packages: blueprint-keystore, blueprint-crypto-bn254, blueprint-crypto, blueprint-client-tangle, blueprint-networking, blueprint-networking-agg-sig-gossip-extension, blueprint-tangle-aggregation-svc

## [0.2.0-alpha.7](https://github.com/tangle-network/blueprint/compare/blueprint-tangle-extra-v0.2.0-alpha.6...blueprint-tangle-extra-v0.2.0-alpha.7) - 2026-05-20

### Other

- updated the following local packages: blueprint-core, blueprint-std, blueprint-crypto-core, blueprint-crypto-bn254, blueprint-crypto, blueprint-keystore, blueprint-client-tangle, blueprint-networking, blueprint-networking-agg-sig-gossip-extension, blueprint-tangle-aggregation-svc

## [0.2.0-alpha.6](https://github.com/tangle-network/blueprint/compare/blueprint-tangle-extra-v0.2.0-alpha.5...blueprint-tangle-extra-v0.2.0-alpha.6) - 2026-05-17

### Other

- updated the following local packages: blueprint-crypto-bn254, blueprint-networking, blueprint-networking-agg-sig-gossip-extension, blueprint-crypto, blueprint-keystore, blueprint-client-tangle, blueprint-tangle-aggregation-svc

## [0.2.0-alpha.4](https://github.com/tangle-network/blueprint/compare/blueprint-tangle-extra-v0.2.0-alpha.3...blueprint-tangle-extra-v0.2.0-alpha.4) - 2026-04-22

### Fixed

- *(tangle)* cap log queries and add gas fallbacks (#1382)
