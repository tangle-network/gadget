# Blueprint SDK

This Rust workspace provides the Blueprint SDK and the `cargo-tangle` CLI.
The workspace manifest owns package membership and dependency selection.
Public SDK exports live in `crates/sdk/`; crate README files and source documentation own their interfaces.
Read the affected crate's instructions and README before changing a contract.
Inspect the directory and source exports instead of maintaining generated directory or API catalogs in instruction files.

## Build and test

Use [rust-toolchain.toml](rust-toolchain.toml), the workspace manifests, and [.github/workflows/ci.yml](.github/workflows/ci.yml) for current toolchain and check configuration.
Rustfmt uses the toolchain selected by CI, which can differ from the compiler toolchain.
Read features from the affected manifest; preserve supported `no_std` and optional-feature builds.
Use `cargo fmt --all` for formatting and `taplo fmt` for manifest formatting under the repository configuration.

Start with crate-scoped checks, then run the broader checks required by the change and CI.
Do not infer workspace coverage from an unqualified `cargo test` command or a feature-gated suite that skipped its cases.
CI owns the list of packages that require serial execution.
Use [.config/nextest.toml](.config/nextest.toml)'s serial profile or `--test-threads=1` for those packages and resource-sharing tests.
Keep network, environment, Docker, and process state isolated across tests.
Build prerequisites and external fixtures must be available before claiming their integration tests passed.

For nontrivial behavior changes, follow [the engineering playbook](docs/engineering/HARNESS_ENGINEERING_PLAYBOOK.md) and [specification](docs/engineering/HARNESS_ENGINEERING_SPEC.md).
Define the behavior and invariants, reproduce the affected user path, and check invalid input and failure recovery.
Update examples and operator documentation when their contracts change.

## Publishing dependencies

A versioned workspace dev-dependency can create a publish-time cycle when its target reaches back to the publishing crate.
For such test-only dependencies, use a path-only dev-dependency instead of `workspace = true`.
For example, a crate can test through the SDK umbrella locally without requiring that umbrella's new release to exist first.
Delete a dev-dependency that the tests do not use.

Before publishing, inspect the dependency graph and the manifests Cargo will package.
Confirm production dependencies remain declared and path-only test dependencies do not introduce a published cycle.
A missing registry version can also mean an unpublished dependency or registry delay; diagnose the actual graph and registry state.
Do not treat every resolution failure as a cycle or remove production version constraints to make it pass.

## Pull requests

Dependency PRs merge only through `beelink-gate --merge <pr> auto <repo-url> <head-sha> -- cargo check --locked --workspace --all-targets`; the gate must pass on that exact head before merging.

Use [.github/PULL_REQUEST_TEMPLATE.md](.github/PULL_REQUEST_TEMPLATE.md) and the current [quality policy](.github/pr-quality-gate.toml).
The [validation script](.github/scripts/validate_pr_body.py) owns required sections, change classes, and documentation-only classification.
Read the [workflow](.github/workflows/pr-quality-gate.yml) when diagnosing how the PR body reaches validation.
A rerun can reuse the original event payload, so verify that a corrected body reached the check before claiming it passed.
Include changed behavior, relevant test evidence, risk, and rollback in the PR.
