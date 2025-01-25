# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tangle-network/gadget/releases/tag/gadget-std-v0.1.0) - 2025-01-25

### Added

- implement runners ([#11](https://github.com/tangle-network/gadget/pull/11))
- separate key types entirely from keystore/put into crypto crates
- *(gadget-contexts)* contexts, clients, and features
- *(gadget-contexts)* wip implementation of contexts
- add tokio-std, bindings rename, config build
- improve ergonomics of `gadget_std::io::Error`
- get building, start on keystore traits
- add std and keystore initial impl

### Fixed

- *(testing-utils)* debugging blueprint tests
- cleanup a bunch of warnings

### Other

- merge `gadget-tokio-std` into `gadget-std`
- workspace builds
- working off donovan/contexts, adding networking, logging, keystore change
