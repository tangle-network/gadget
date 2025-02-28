## The Blueprint Framework crates

The Blueprint Framework is a collection of crates that makes it easy to build Tangle Blueprints.

### Blueprint SDK

This the Umbrella crate for the Blueprint Framework which exposes the core functionality of the framework to the user, and this crate is all what you need to get started with the framework.

Each sub-crate provides a specific functionality of the framework, and they are all hidden behind a feature flag(s) that you can enable/disable in your project.

### Project crates

-   [`blueprint-core`](./core): The core functionality of the framework, including the core data structures and the core traits that is used and shared across the framework.
-   [`blueprint-router`](./router): Includes the job routing functionality of the framework, this usually used hand in hand with the [`blueprint-runner`](./runner) crate.
-   [`blueprint-runner`](./runner): Includes the job execution functionality of the framework, which is your entry point to your blueprint.
-   [`blueprint-tangle-extra`](./tangle-extra): This crate provides extra functionality to use the Blueprint SDK with the Tangle Network.
-   [`blueprint-evm-extra`](./evm-extra): This crate provides extra functionality to use the Blueprint SDK with any EVM compatible network.
-   [`blueprint-sdk`](./sdk): This the most outer layer for the Blueprint Framework which acts as an umbrella crate for the framework.
-   [`blueprint-macros`](./macros): This crate provides the procedural macros that are used across the framework.

### Crate Naming Conventions

1. Each crate name starts with `blueprint-` followed by the name of the crate.
2. Any integration with any other system or crate, it should start with `blueprint-` followed by the name of the system or crate and ends with `-extra`.
3. The directory name should be in `crates/{name}` without the `blueprint-` prefix.
