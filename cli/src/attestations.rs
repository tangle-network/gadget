use crate::upgrade_context::{upgrade_tx_ctx, upgrade_view_ctx};
use cargo_tangle::command::tangle::TangleClientArgs;
use clap::Subcommand;
use color_eyre::eyre::Result;
use serde_json::json;

#[derive(Subcommand, Debug)]
pub(super) enum AttestCommands {
    /// Submit a new attestation against `(blueprint_id, version_id)`.
    ///
    /// Hashes the report file (or accepts an explicit `--report-hash`), maps
    /// the severity string to its uint8 ladder entry, and submits
    /// `attestBinaryVersion`. Auditor identity = `msg.sender`.
    Submit {
        #[command(flatten)]
        network: TangleClientArgs,
        #[arg(long)]
        blueprint_id: u64,
        #[arg(long)]
        version_id: u64,
        /// Path to report artifact (PDF, JSON, etc.) OR an off-chain URL.
        ///
        /// If the value resolves to an existing file, the sha256 is computed
        /// locally and the file may also be pinned to IPFS via `--pin-report-to-ipfs`.
        /// If it's a URL (e.g. `https://...`, `ipfs://...`), it's treated as
        /// `reportUri` with `reportHash = 0x0` unless `--report-hash` is provided.
        #[arg(long, value_name = "PATH_OR_URL")]
        report: String,
        /// Attestation kind.
        #[arg(long, value_enum)]
        kind: cargo_tangle::command::upgrade::AttestationKindArg,
        /// Severity discovered.
        #[arg(long, value_enum)]
        severity: cargo_tangle::command::upgrade::SeverityArg,
        /// Optional expiry as a duration from now (e.g. `6m`, `30d`, `1y`).
        #[arg(long, value_name = "DURATION")]
        expires_in: Option<String>,
        /// Pin a local report file to IPFS and use that as `reportUri`.
        #[arg(long)]
        pin_report_to_ipfs: bool,
        /// Override report hash (32-byte hex). Use when `--report` is a URL
        /// and you've computed the hash off-line.
        #[arg(long, value_name = "HEX")]
        report_hash: Option<String>,
        #[arg(long)]
        json: bool,
    },

    /// Revoke an attestation you previously submitted.
    Revoke {
        #[command(flatten)]
        network: TangleClientArgs,
        #[arg(long)]
        blueprint_id: u64,
        #[arg(long)]
        version_id: u64,
        #[arg(long)]
        attestation_id: u64,
        /// Off-chain pointer describing why the attestation was withdrawn.
        #[arg(long, value_name = "URI")]
        reason: String,
        #[arg(long)]
        json: bool,
    },

    /// List all attestations for a binary version.
    List {
        #[command(flatten)]
        network: TangleClientArgs,
        #[arg(long)]
        blueprint_id: u64,
        #[arg(long)]
        version_id: u64,
        #[arg(long)]
        json: bool,
    },
}

pub(super) async fn execute(command: AttestCommands) -> Result<()> {
    match command {
        AttestCommands::Submit {
            network,
            blueprint_id,
            version_id,
            report,
            kind,
            severity,
            expires_in,
            pin_report_to_ipfs,
            report_hash,
            json,
        } => {
            let ctx = upgrade_tx_ctx(&network)?;
            let (resolved_hash, resolved_uri) =
                resolve_report_inputs(&report, report_hash.as_deref(), pin_report_to_ipfs).await?;
            let expires_at = cargo_tangle::command::upgrade::duration_to_expiry_timestamp(
                expires_in.as_deref(),
            )?;
            let result = cargo_tangle::command::upgrade::attest_version(
                &ctx,
                blueprint_id,
                version_id,
                resolved_hash,
                resolved_uri,
                kind,
                severity,
                expires_at,
            )
            .await?;
            cargo_tangle::command::upgrade::print_attest_result(&result, json);
        }
        AttestCommands::Revoke {
            network,
            blueprint_id,
            version_id,
            attestation_id,
            reason,
            json,
        } => {
            let ctx = upgrade_tx_ctx(&network)?;
            let tx = cargo_tangle::command::upgrade::revoke_attestation(
                &ctx,
                blueprint_id,
                version_id,
                attestation_id,
                reason,
            )
            .await?;
            cargo_tangle::command::upgrade::print_simple_tx(
                &format!(
                    "revoke-attestation(bp={blueprint_id}, ver={version_id}, att={attestation_id})"
                ),
                &tx,
                json,
            );
        }
        AttestCommands::List {
            network,
            blueprint_id,
            version_id,
            json,
        } => {
            let view = upgrade_view_ctx(&network, None)?;
            let rows =
                cargo_tangle::command::upgrade::list_attestations(&view, blueprint_id, version_id)
                    .await?;
            cargo_tangle::command::upgrade::print_attestations(
                blueprint_id,
                version_id,
                &rows,
                json,
            );
        }
    }
    Ok(())
}

async fn resolve_report_inputs(
    report: &str,
    report_hash_override: Option<&str>,
    pin_report_to_ipfs: bool,
) -> Result<(alloy_primitives::B256, String)> {
    let path = std::path::Path::new(report);
    if path.exists() {
        let (digest, _) = cargo_tangle::command::upgrade::hash_file(path)?;
        let uri = if pin_report_to_ipfs {
            cargo_tangle::command::upgrade::pin_file_to_ipfs(path)
                .await?
                .uri
        } else {
            // Local-only artifact: emit a deterministic file:// URI as a fallback so the
            // on-chain `reportUri` invariant (non-empty) is preserved.
            format!("file://{}", path.display())
        };
        return Ok((digest, uri));
    }
    // Treat as URL/URI string.
    let hash = match report_hash_override {
        Some(hex) => cargo_tangle::command::upgrade::parse_b256(hex, "--report-hash")?,
        None => alloy_primitives::B256::ZERO,
    };
    Ok((hash, report.to_string()))
}
