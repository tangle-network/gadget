# `tangle-network/blueprint/.github/actions/ship-release`

Composite action that drives `cargo tangle blueprint ship` from a GitHub Actions
workflow. The same wizard that an operator runs locally is the one CI calls
— there is no parallel codepath to drift.

## What it does

1. Installs `cargo-tangle` (pinnable via `cargo-tangle-version:`).
2. Resolves the binary URI flag: either a pre-existing `--binary-uri` or
   `--pin-ipfs` (which uses the credentials below). Errors out if neither.
3. Resolves the attestation flag: either `--attestation-bundle <path>` or
   `--attestation-hash <hex>`. Errors out if both.
4. Runs `cargo tangle blueprint ship --yes --no-build --json` against the supplied
   binary and parses the `ship_complete` event for outputs.

The action **never** runs `cargo build` — it expects a previous step to
have produced the release binary. This makes it composable with any build
matrix (Linux/Mac, cross-rs, etc.) and keeps the action focused on the
publish/promote half of the release.

## Inputs

| Input                  | Required | Default | Notes |
|------------------------|----------|---------|-------|
| `blueprint-id`         | yes      | —       | On-chain blueprint id |
| `binary-path`          | yes      | —       | Path to the built release binary |
| `binary-uri`           | no       | —       | Pre-pinned `ipfs://` or `https://` URI. Mutually exclusive with `pin-ipfs`. |
| `pin-ipfs`             | no       | `false` | Pin via `IPFS_API_URL`+`IPFS_API_TOKEN`, `PINATA_JWT`, or `WEB3_STORAGE_TOKEN` |
| `attestation-bundle`   | no       | —       | sigstore/SLSA bundle; sha256 becomes `attestationHash` |
| `attestation-hash`     | no       | —       | Pre-computed 32-byte hex hash (alternative to `attestation-bundle`) |
| `promote`              | no       | `false` | Call `setActiveBinaryVersion` so AUTO services adopt the new version |
| `policy-services`      | no       | —       | Comma-separated service IDs to bulk-flip into AUTO |
| `rpc-url`              | yes      | —       | HTTP RPC for the target chain |
| `ws-rpc-url`           | no       | —       | WebSocket RPC (optional) |
| `tangle-contract`      | yes      | —       | Tangle diamond proxy address |
| `keystore-path`        | no       | `./keystore` | Path to the keystore that holds the deployer ECDSA key |
| `cargo-tangle-version` | no       | latest  | Pin a specific cargo-tangle version |

## Outputs

| Output       | Description |
|--------------|-------------|
| `version-id` | The newly-published `versionId` |
| `sha256`     | SHA-256 of the published binary (hex) |
| `binary-uri` | Resolved `binaryUri` (input or pin result) |
| `tx-hash`    | Publish transaction hash |

## Required secrets

Set these in your workflow's `env:` block, not as action inputs (so they
never appear in the action's input log):

| Secret                | Used for |
|-----------------------|----------|
| `DEPLOYER_KEY`        | Signing key for `publishBinaryVersion` etc. — must be the blueprint owner |
| `WEB3_STORAGE_TOKEN`  | Pinning via web3.storage (when `pin-ipfs: 'true'`) |
| `PINATA_JWT`          | Pinning via Pinata (fallback) |
| `IPFS_API_URL` + `IPFS_API_TOKEN` | Self-hosted Kubo / w3up endpoint |

## Example workflow

See [`.github/examples/ship-release.yml`](../../examples/ship-release.yml)
in this repository for a full end-to-end example wired to GitHub Releases.

```yaml
on:
  release: { types: [published] }
jobs:
  ship:
    runs-on: [self-hosted, ci-public]
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-rust@v1
      - run: cargo build --release -p my-blueprint
      - uses: tangle-network/blueprint/.github/actions/ship-release@main
        with:
          blueprint-id: ${{ vars.BLUEPRINT_ID }}
          binary-path: target/release/my-blueprint
          pin-ipfs: 'true'
          promote: 'true'
          rpc-url: ${{ secrets.RPC_URL }}
          tangle-contract: ${{ vars.TANGLE_CONTRACT }}
        env:
          DEPLOYER_KEY: ${{ secrets.DEPLOYER_KEY }}
          WEB3_STORAGE_TOKEN: ${{ secrets.WEB3_STORAGE_TOKEN }}
```

## Failure modes

- **Missing IPFS strategy**: the resolve step errors before any cargo-tangle
  invocation. Either set `binary-uri:` or `pin-ipfs: 'true'`.
- **Mutually exclusive attestation inputs**: pick one of
  `attestation-bundle` or `attestation-hash`, not both.
- **No `ship_complete` event in the JSON stream**: cargo-tangle errored out
  before reaching the publish step (build/network/keystore problem). The
  raw stdout is captured in the action log immediately above the failure.
