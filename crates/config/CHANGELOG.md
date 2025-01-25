# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tangle-network/gadget/releases/tag/gadget-config-v0.1.0) - 2025-01-25

### Added

- *(macros)* use `sdk::Error` instead of `Box<dyn Error>` ([#604](https://github.com/tangle-network/gadget/pull/604))
- add port number sanitizing
- Add more networking tests, add CI potential fix
- complete TangleClient, generalize services, contexts
- implement runners ([#11](https://github.com/tangle-network/gadget/pull/11))
- *(gadget-contexts)* contexts, clients, and features
- add tokio-std, bindings rename, config build

### Fixed

- get job macro building

### Other

- improve debug output for port parsing
- remove unused type param
- clean up refs to std
- fixes
- clippy
- test instrumented client and others ([#9](https://github.com/tangle-network/gadget/pull/9))
- workspace builds
