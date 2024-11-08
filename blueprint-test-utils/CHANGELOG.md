# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/tangle-network/gadget/releases/tag/blueprint-test-utils-v0.1.1) - 2024-11-08

### Added

- add Substrate Node Runner to blueprint-test-utils ([#460](https://github.com/tangle-network/gadget/pull/460))
- [**breaking**] Refactor EventFlows for EVM and Remove EventWatchers ([#423](https://github.com/tangle-network/gadget/pull/423))
- symbiotic initial integration ([#411](https://github.com/tangle-network/gadget/pull/411))
- add optional data dir to blueprint manager ([#342](https://github.com/tangle-network/gadget/pull/342))
- eigenlayer incredible squaring blueprint and test ([#312](https://github.com/tangle-network/gadget/pull/312))

### Fixed

- *(gadget-sdk)* [**breaking**] prevent duplicate and self-referential messages ([#458](https://github.com/tangle-network/gadget/pull/458))
- *(blueprint-test-utils)* improve efficiency in handling of keys and environment in tests ([#431](https://github.com/tangle-network/gadget/pull/431))
- *(gadget-sdk)* update sdk and utilities for tangle avs ([#355](https://github.com/tangle-network/gadget/pull/355))
- *(cargo-tangle)* CLI bugs ([#409](https://github.com/tangle-network/gadget/pull/409))
- *(gadget-sdk)* updated keystore support and fixes ([#368](https://github.com/tangle-network/gadget/pull/368))
- *(sdk)* [**breaking**] downgrade substrate dependencies for now
- add `data_dir` back to `GadgetConfiguration` ([#350](https://github.com/tangle-network/gadget/pull/350))

### Other

- set blueprint-manager publishable ([#462](https://github.com/tangle-network/gadget/pull/462))
- improve test-utils and lower networking log level ([#448](https://github.com/tangle-network/gadget/pull/448))
- add description to crates ([#444](https://github.com/tangle-network/gadget/pull/444))
- Continue Improving Event Flows ([#399](https://github.com/tangle-network/gadget/pull/399))
- improve blueprint-manager and blueprint-test-utils ([#421](https://github.com/tangle-network/gadget/pull/421))
- Leverage blueprint in incredible squaring aggregator ([#365](https://github.com/tangle-network/gadget/pull/365))
- Event Flows for Tangle ([#363](https://github.com/tangle-network/gadget/pull/363))
- release ([#379](https://github.com/tangle-network/gadget/pull/379))
- release ([#378](https://github.com/tangle-network/gadget/pull/378))
- release ([#362](https://github.com/tangle-network/gadget/pull/362))
- Fix eigenlayer example ([#375](https://github.com/tangle-network/gadget/pull/375))
- update to latest changes in tangle ([#367](https://github.com/tangle-network/gadget/pull/367))
- Multi job runner + SDK main macro ([#346](https://github.com/tangle-network/gadget/pull/346))
- Event Listener Upgrade + Wrapper Types + sdk::main macro ([#333](https://github.com/tangle-network/gadget/pull/333))
- update naming ([#343](https://github.com/tangle-network/gadget/pull/343))
- release ([#314](https://github.com/tangle-network/gadget/pull/314))
- release ([#313](https://github.com/tangle-network/gadget/pull/313))
- release ([#308](https://github.com/tangle-network/gadget/pull/308))
- Remove Logger ([#311](https://github.com/tangle-network/gadget/pull/311))
- Streamline keystore, cleanup testing, refactor blueprint manager, add tests, remove unnecessary code ([#285](https://github.com/tangle-network/gadget/pull/285))
- CI Improvements ([#301](https://github.com/tangle-network/gadget/pull/301))
- Expose executor from SDK ([#300](https://github.com/tangle-network/gadget/pull/300))
- [MEGA PR] Overhaul repo, add Eigenlayer AVS example, remove many crates, add testing, remove unused code ([#246](https://github.com/tangle-network/gadget/pull/246))
- Remove unused workspace dependencies ([#276](https://github.com/tangle-network/gadget/pull/276))
- Add mpc blueprint starting point, cleanup abstractions ([#252](https://github.com/tangle-network/gadget/pull/252))
- Add more checks to CI ([#244](https://github.com/tangle-network/gadget/pull/244))
- Fix `too_long_first_doc_paragraph` ([#243](https://github.com/tangle-network/gadget/pull/243))
- Promote all dependencies to workspace ([#233](https://github.com/tangle-network/gadget/pull/233))
- Make `{core, io, common}` no_std and WASM compatible ([#231](https://github.com/tangle-network/gadget/pull/231))
- Remove shell sdk and put inside blueprint manager ([#229](https://github.com/tangle-network/gadget/pull/229))
- Blueprint testing ([#206](https://github.com/tangle-network/gadget/pull/206))

## [0.1.1](https://github.com/tangle-network/gadget/releases/tag/blueprint-test-utils-v0.1.1) - 2024-10-23

### Added

- add optional data dir to blueprint manager ([#342](https://github.com/tangle-network/gadget/pull/342))
- eigenlayer incredible squaring blueprint and test ([#312](https://github.com/tangle-network/gadget/pull/312))

### Fixed

- *(sdk)* updated keystore support and fixes ([#368](https://github.com/tangle-network/gadget/pull/368))
- *(sdk)* [**breaking**] downgrade substrate dependencies for now
- add `data_dir` back to `GadgetConfiguration` ([#350](https://github.com/tangle-network/gadget/pull/350))

### Other

- release ([#378](https://github.com/tangle-network/gadget/pull/378))
- release ([#362](https://github.com/tangle-network/gadget/pull/362))
- Fix eigenlayer example ([#375](https://github.com/tangle-network/gadget/pull/375))
- update to latest changes in tangle ([#367](https://github.com/tangle-network/gadget/pull/367))
- Multi job runner + SDK main macro ([#346](https://github.com/tangle-network/gadget/pull/346))
- Event Listener Upgrade + Wrapper Types + sdk::main macro ([#333](https://github.com/tangle-network/gadget/pull/333))
- update naming ([#343](https://github.com/tangle-network/gadget/pull/343))
- release ([#314](https://github.com/tangle-network/gadget/pull/314))
- release ([#313](https://github.com/tangle-network/gadget/pull/313))
- release ([#308](https://github.com/tangle-network/gadget/pull/308))
- Remove Logger ([#311](https://github.com/tangle-network/gadget/pull/311))
- Streamline keystore, cleanup testing, refactor blueprint manager, add tests, remove unnecessary code ([#285](https://github.com/tangle-network/gadget/pull/285))
- CI Improvements ([#301](https://github.com/tangle-network/gadget/pull/301))
- Expose executor from SDK ([#300](https://github.com/tangle-network/gadget/pull/300))
- [MEGA PR] Overhaul repo, add Eigenlayer AVS example, remove many crates, add testing, remove unused code ([#246](https://github.com/tangle-network/gadget/pull/246))
- Remove unused workspace dependencies ([#276](https://github.com/tangle-network/gadget/pull/276))
- Add mpc blueprint starting point, cleanup abstractions ([#252](https://github.com/tangle-network/gadget/pull/252))
- Add more checks to CI ([#244](https://github.com/tangle-network/gadget/pull/244))
- Fix `too_long_first_doc_paragraph` ([#243](https://github.com/tangle-network/gadget/pull/243))
- Promote all dependencies to workspace ([#233](https://github.com/tangle-network/gadget/pull/233))
- Make `{core, io, common}` no_std and WASM compatible ([#231](https://github.com/tangle-network/gadget/pull/231))
- Remove shell sdk and put inside blueprint manager ([#229](https://github.com/tangle-network/gadget/pull/229))
- Blueprint testing ([#206](https://github.com/tangle-network/gadget/pull/206))

## [0.1.1](https://github.com/tangle-network/gadget/releases/tag/blueprint-test-utils-v0.1.1) - 2024-10-23

### Added

- add optional data dir to blueprint manager ([#342](https://github.com/tangle-network/gadget/pull/342))
- eigenlayer incredible squaring blueprint and test ([#312](https://github.com/tangle-network/gadget/pull/312))

### Fixed

- *(sdk)* updated keystore support and fixes ([#368](https://github.com/tangle-network/gadget/pull/368))
- *(sdk)* [**breaking**] downgrade substrate dependencies for now
- add `data_dir` back to `GadgetConfiguration` ([#350](https://github.com/tangle-network/gadget/pull/350))

### Other

- release ([#362](https://github.com/tangle-network/gadget/pull/362))
- Fix eigenlayer example ([#375](https://github.com/tangle-network/gadget/pull/375))
- update to latest changes in tangle ([#367](https://github.com/tangle-network/gadget/pull/367))
- Multi job runner + SDK main macro ([#346](https://github.com/tangle-network/gadget/pull/346))
- Event Listener Upgrade + Wrapper Types + sdk::main macro ([#333](https://github.com/tangle-network/gadget/pull/333))
- update naming ([#343](https://github.com/tangle-network/gadget/pull/343))
- release ([#314](https://github.com/tangle-network/gadget/pull/314))
- release ([#313](https://github.com/tangle-network/gadget/pull/313))
- release ([#308](https://github.com/tangle-network/gadget/pull/308))
- Remove Logger ([#311](https://github.com/tangle-network/gadget/pull/311))
- Streamline keystore, cleanup testing, refactor blueprint manager, add tests, remove unnecessary code ([#285](https://github.com/tangle-network/gadget/pull/285))
- CI Improvements ([#301](https://github.com/tangle-network/gadget/pull/301))
- Expose executor from SDK ([#300](https://github.com/tangle-network/gadget/pull/300))
- [MEGA PR] Overhaul repo, add Eigenlayer AVS example, remove many crates, add testing, remove unused code ([#246](https://github.com/tangle-network/gadget/pull/246))
- Remove unused workspace dependencies ([#276](https://github.com/tangle-network/gadget/pull/276))
- Add mpc blueprint starting point, cleanup abstractions ([#252](https://github.com/tangle-network/gadget/pull/252))
- Add more checks to CI ([#244](https://github.com/tangle-network/gadget/pull/244))
- Fix `too_long_first_doc_paragraph` ([#243](https://github.com/tangle-network/gadget/pull/243))
- Promote all dependencies to workspace ([#233](https://github.com/tangle-network/gadget/pull/233))
- Make `{core, io, common}` no_std and WASM compatible ([#231](https://github.com/tangle-network/gadget/pull/231))
- Remove shell sdk and put inside blueprint manager ([#229](https://github.com/tangle-network/gadget/pull/229))
- Blueprint testing ([#206](https://github.com/tangle-network/gadget/pull/206))

## [0.1.1](https://github.com/tangle-network/gadget/releases/tag/blueprint-test-utils-v0.1.1) - 2024-10-23

### Fixed

- *(sdk)* updated keystore support and fixes ([#368](https://github.com/tangle-network/gadget/pull/368))

## [0.1.1](https://github.com/tangle-network/gadget/releases/tag/blueprint-test-utils-v0.1.1) - 2024-09-30

### Other

- release ([#313](https://github.com/tangle-network/gadget/pull/313))
- release ([#308](https://github.com/tangle-network/gadget/pull/308))
- Remove Logger ([#311](https://github.com/tangle-network/gadget/pull/311))
- Streamline keystore, cleanup testing, refactor blueprint manager, add tests, remove unnecessary code ([#285](https://github.com/tangle-network/gadget/pull/285))
- CI Improvements ([#301](https://github.com/tangle-network/gadget/pull/301))
- Expose executor from SDK ([#300](https://github.com/tangle-network/gadget/pull/300))
- [MEGA PR] Overhaul repo, add Eigenlayer AVS example, remove many crates, add testing, remove unused code ([#246](https://github.com/tangle-network/gadget/pull/246))
- Remove unused workspace dependencies ([#276](https://github.com/tangle-network/gadget/pull/276))
- Add mpc blueprint starting point, cleanup abstractions ([#252](https://github.com/tangle-network/gadget/pull/252))
- Add more checks to CI ([#244](https://github.com/tangle-network/gadget/pull/244))
- Fix `too_long_first_doc_paragraph` ([#243](https://github.com/tangle-network/gadget/pull/243))
- Promote all dependencies to workspace ([#233](https://github.com/tangle-network/gadget/pull/233))
- Make `{core, io, common}` no_std and WASM compatible ([#231](https://github.com/tangle-network/gadget/pull/231))
- Remove shell sdk and put inside blueprint manager ([#229](https://github.com/tangle-network/gadget/pull/229))
- Blueprint testing ([#206](https://github.com/tangle-network/gadget/pull/206))

## [0.1.1](https://github.com/tangle-network/gadget/releases/tag/blueprint-test-utils-v0.1.1) - 2024-09-25

### Other

- release ([#308](https://github.com/tangle-network/gadget/pull/308))
- Remove Logger ([#311](https://github.com/tangle-network/gadget/pull/311))
- Streamline keystore, cleanup testing, refactor blueprint manager, add tests, remove unnecessary code ([#285](https://github.com/tangle-network/gadget/pull/285))
- CI Improvements ([#301](https://github.com/tangle-network/gadget/pull/301))
- Expose executor from SDK ([#300](https://github.com/tangle-network/gadget/pull/300))
- [MEGA PR] Overhaul repo, add Eigenlayer AVS example, remove many crates, add testing, remove unused code ([#246](https://github.com/tangle-network/gadget/pull/246))
- Remove unused workspace dependencies ([#276](https://github.com/tangle-network/gadget/pull/276))
- Add mpc blueprint starting point, cleanup abstractions ([#252](https://github.com/tangle-network/gadget/pull/252))
- Add more checks to CI ([#244](https://github.com/tangle-network/gadget/pull/244))
- Fix `too_long_first_doc_paragraph` ([#243](https://github.com/tangle-network/gadget/pull/243))
- Promote all dependencies to workspace ([#233](https://github.com/tangle-network/gadget/pull/233))
- Make `{core, io, common}` no_std and WASM compatible ([#231](https://github.com/tangle-network/gadget/pull/231))
- Remove shell sdk and put inside blueprint manager ([#229](https://github.com/tangle-network/gadget/pull/229))
- Blueprint testing ([#206](https://github.com/tangle-network/gadget/pull/206))

## [0.1.1](https://github.com/tangle-network/gadget/releases/tag/blueprint-test-utils-v0.1.1) - 2024-09-24

### Other

- Remove Logger ([#311](https://github.com/tangle-network/gadget/pull/311))
- Streamline keystore, cleanup testing, refactor blueprint manager, add tests, remove unnecessary code ([#285](https://github.com/tangle-network/gadget/pull/285))
- CI Improvements ([#301](https://github.com/tangle-network/gadget/pull/301))
- Expose executor from SDK ([#300](https://github.com/tangle-network/gadget/pull/300))
- [MEGA PR] Overhaul repo, add Eigenlayer AVS example, remove many crates, add testing, remove unused code ([#246](https://github.com/tangle-network/gadget/pull/246))
- Remove unused workspace dependencies ([#276](https://github.com/tangle-network/gadget/pull/276))
- Add mpc blueprint starting point, cleanup abstractions ([#252](https://github.com/tangle-network/gadget/pull/252))
- Add more checks to CI ([#244](https://github.com/tangle-network/gadget/pull/244))
- Fix `too_long_first_doc_paragraph` ([#243](https://github.com/tangle-network/gadget/pull/243))
- Promote all dependencies to workspace ([#233](https://github.com/tangle-network/gadget/pull/233))
- Make `{core, io, common}` no_std and WASM compatible ([#231](https://github.com/tangle-network/gadget/pull/231))
- Remove shell sdk and put inside blueprint manager ([#229](https://github.com/tangle-network/gadget/pull/229))
- Blueprint testing ([#206](https://github.com/tangle-network/gadget/pull/206))
