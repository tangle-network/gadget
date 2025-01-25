# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tangle-network/gadget/releases/tag/gadget-contexts-v0.1.0) - 2025-01-25

### Added

- eigenlayer incredible squaring blueprint ([#43](https://github.com/tangle-network/gadget/pull/43))
- Improve test utils, fix tests ([#34](https://github.com/tangle-network/gadget/pull/34))
- integrate clients into macros
- complete TangleClient, generalize services, contexts
- use new contexts in macros
- add event listeners
- separate key types entirely from keystore/put into crypto crates

### Fixed

- *(incredible-squaring)* debugging blueprint test
- fix!(gadget-testing-utils): improved runner testing utilities for blueprints
- remove target addr from p2p client
- get tests compiling
- get job macro building
- get macro tests closer to building
- rename services client method

### Other

- *(clippy)* fmt
- add empty test to contexts
- clippy fixes, renaming
- workspace builds
- working off donovan/contexts, adding networking, logging, keystore change
