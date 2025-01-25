# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tangle-network/gadget/releases/tag/gadget-event-listeners-v0.1.0) - 2025-01-25

### Added

- eigenlayer incredible squaring blueprint ([#43](https://github.com/tangle-network/gadget/pull/43))
- add CronJob
- migrate blueprint-manager
- integrate clients into macros
- complete TangleClient, generalize services, contexts
- use new contexts in macros
- implement runners ([#11](https://github.com/tangle-network/gadget/pull/11))
- add event listeners

### Fixed

- revert no-std TanglePairSigner
- get tests compiling
- get job macro building
- get crates building again
- use `gadget_std` more
- update type paths in `job` macro

### Other

- *(event-listeners)* add periodic test
- *(event-listeners)* flatten crate structures
- clippy
- cleanup job macro impl
- get incredible-squaring building
- add ci
