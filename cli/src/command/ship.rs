//! `cargo tangle blueprint ship` — one-command blueprint release.
//!
//! Wraps the existing `publish-version` + `set-active-version` + `set-policy`
//! paths into a single interactive flow:
//!
//! 1. Detect the workspace, blueprint id, network, and signer.
//! 2. (Optional) `cargo build --release` for the chosen package.
//! 3. Hash the binary, optionally pin to IPFS, optionally derive
//!    `attestationHash` from a sigstore/SLSA bundle.
//! 4. `publishBinaryVersion`, optionally `setActiveBinaryVersion`,
//!    optionally bulk `setServiceUpgradePolicy(AUTO)` on a service list.
//! 5. Print the resulting trust score and a one-shot summary.
//!
//! CI mode (`--yes --pin-ipfs --promote`) skips every prompt and is the
//! glue the `tangle-network/blueprint/.github/actions/ship-release` action
//! consumes.
//!
//! Auto-detection convention:
//! - `--blueprint-id` flag wins.
//! - Otherwise read `<cwd>/settings.env` for `BLUEPRINT_ID=`.
//! - Otherwise read `<cwd>/metadata/blueprint-metadata.json` for a
//!   `blueprintId` top-level key.
//! - Otherwise fail fast with an actionable message.

use std::path::{Path, PathBuf};
use std::process::Command;

use alloy_primitives::B256;
use color_eyre::eyre::{Context, Result, bail, eyre};
use dialoguer::console::style;
use dialoguer::{Confirm, Input, theme::ColorfulTheme};
use serde_json::json;

use crate::command::tangle::TangleClientArgs;
use crate::command::upgrade::{
    PinnedArtifact, PublishVersionResult, TxContext, UpgradePolicyArg, hash_file, parse_b256,
    pin_file_to_ipfs, publish_version, set_active_version, set_service_policy,
};

/// Inputs to the ship wizard. Every field except `network` may be either
/// resolved interactively or passed via flag — interactive mode is opt-in
/// via `--yes` being absent.
#[derive(Debug, Clone, clap::Args)]
pub struct ShipArgs {
    #[command(flatten)]
    pub network: TangleClientArgs,
    /// Accept all prompts. Implies `--json` for CI-friendly output.
    #[arg(long)]
    pub yes: bool,
    /// Don't run `cargo build --release` — caller supplies `--binary`.
    #[arg(long)]
    pub no_build: bool,
    /// Cargo package to build (`-p <pkg>`).
    #[arg(long, short = 'p', value_name = "NAME")]
    pub package: Option<String>,
    /// Path to a pre-built release binary.
    #[arg(long, value_name = "PATH")]
    pub binary: Option<PathBuf>,
    /// Pre-existing artifact URI (`ipfs://...`, `https://...`). Skips pinning.
    #[arg(long, value_name = "URI", conflicts_with = "pin_ipfs")]
    pub binary_uri: Option<String>,
    /// Pin the binary to IPFS using `IPFS_API_URL` + `IPFS_API_TOKEN`, or `PINATA_JWT`.
    #[arg(long)]
    pub pin_ipfs: bool,
    /// sigstore/SLSA bundle whose sha256 lands on-chain as `attestationHash`.
    #[arg(long, value_name = "PATH")]
    pub attestation_bundle: Option<PathBuf>,
    /// Explicit `attestationHash` (32-byte hex). Conflicts with `--attestation-bundle`.
    #[arg(long, value_name = "HEX", conflicts_with = "attestation_bundle")]
    pub attestation_hash: Option<String>,
    /// Promote the new version to active (`setActiveBinaryVersion`).
    #[arg(long, conflicts_with = "no_promote")]
    pub promote: bool,
    /// Explicitly skip promotion even if interactive default is yes.
    #[arg(long)]
    pub no_promote: bool,
    /// Comma-separated service IDs to bulk-flip into AUTO policy.
    #[arg(long, value_name = "LIST")]
    pub policy_services: Option<String>,
    /// Validate everything end-to-end without submitting transactions.
    #[arg(long)]
    pub dry_run: bool,
    /// Override blueprint id (skips auto-detection from settings.env / metadata).
    #[arg(long)]
    pub blueprint_id: Option<u64>,
    /// JSON output for machine readers (also implied by `--yes`).
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug)]
struct DetectedWorkspace {
    cwd: PathBuf,
    blueprint_id: u64,
    package: Option<String>,
}

/// Single entrypoint. Returns Ok on success; any prompt cancellation /
/// validation failure is surfaced as a normal error.
pub async fn run(args: ShipArgs) -> Result<()> {
    let cwd = std::env::current_dir().context("reading cwd")?;
    let detected = detect_workspace(&cwd, &args)?;
    let ctx = build_tx_context(&args.network)?;
    let signer = signer_address(&ctx)?;

    print_header(&detected, &args.network, signer, &args)?;

    // --- 1. Build
    let binary_path = resolve_binary(&detected, &args)?;
    let (sha256_hash, size_bytes) = hash_file(&binary_path)?;
    println!(
        "  {}      {sha256_hash:#x}  ({:.2} MB)",
        style("sha256").bold(),
        size_bytes as f64 / 1_048_576.0
    );

    // --- 2. IPFS
    let binary_uri = resolve_binary_uri(&binary_path, &args).await?;
    println!("  {}    {binary_uri}", style("binaryUri").bold());

    // --- 3. Attestation
    let attestation_hash = resolve_attestation_hash(&args)?;
    println!("  {} {attestation_hash:#x}", style("attestation").bold(),);

    if args.dry_run {
        print_dry_run_summary(
            &detected,
            &sha256_hash,
            &binary_uri,
            &attestation_hash,
            args.promote,
            args.json,
        );
        return Ok(());
    }

    // --- 4. Publish
    let confirm_publish = if args.yes {
        true
    } else {
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!(
                "Publish v? on-chain (blueprint={})?",
                detected.blueprint_id
            ))
            .default(true)
            .interact()?
    };
    if !confirm_publish {
        bail!("publish aborted by user");
    }
    let publish_result = publish_version(
        &ctx,
        detected.blueprint_id,
        sha256_hash,
        binary_uri.clone(),
        attestation_hash,
    )
    .await?;
    println!(
        "  {} v{} ({})",
        style("✓ Published").green().bold(),
        publish_result.version_id,
        publish_result
            .block_number
            .map(|b| format!("block {b}"))
            .unwrap_or_else(|| "pending".into()),
    );

    // --- 5. Promote
    let promote = decide_promote(&args)?;
    if promote {
        let tx = set_active_version(&ctx, detected.blueprint_id, publish_result.version_id).await?;
        println!(
            "  {} v{} (tx {:#x})",
            style("✓ Promoted").green().bold(),
            publish_result.version_id,
            tx.tx_hash,
        );
    }

    // --- 6. Optional bulk policy flip
    if let Some(services_csv) = &args.policy_services {
        let ids = parse_service_ids(services_csv)?;
        for sid in ids {
            let tx = set_service_policy(&ctx, sid, UpgradePolicyArg::Auto).await?;
            println!(
                "  {} service {} -> AUTO (tx {:#x})",
                style("✓ Policy").green().bold(),
                sid,
                tx.tx_hash
            );
        }
    }

    print_shipped_summary(&detected, &publish_result, promote, args.json);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Detection / inputs
// ─────────────────────────────────────────────────────────────────────────────

fn detect_workspace(cwd: &Path, args: &ShipArgs) -> Result<DetectedWorkspace> {
    let blueprint_id = if let Some(id) = args.blueprint_id {
        id
    } else if let Some(id) = read_blueprint_id_from_env_file(cwd)? {
        id
    } else if let Some(id) = read_blueprint_id_from_metadata(cwd)? {
        id
    } else {
        bail!(
            "could not auto-detect blueprint id. Pass --blueprint-id, or define BLUEPRINT_ID \
             in ./settings.env, or add a top-level `blueprintId` field to \
             metadata/blueprint-metadata.json."
        );
    };
    Ok(DetectedWorkspace {
        cwd: cwd.to_path_buf(),
        blueprint_id,
        package: args.package.clone(),
    })
}

/// Read `BLUEPRINT_ID=` from a key=value `settings.env` file if present.
/// Accepts both quoted and unquoted values and trims surrounding whitespace.
fn read_blueprint_id_from_env_file(cwd: &Path) -> Result<Option<u64>> {
    let path = cwd.join("settings.env");
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).with_context(|| format!("reading {path:?}"))?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("BLUEPRINT_ID=") {
            let raw = rest.trim().trim_matches('"').trim_matches('\'');
            if raw == "0" || raw.is_empty() {
                // The example file ships `BLUEPRINT_ID=0` as a placeholder; treat
                // it as "not configured" so we fall through to the next source.
                return Ok(None);
            }
            let id: u64 = raw
                .parse()
                .with_context(|| format!("parsing BLUEPRINT_ID from {path:?}"))?;
            return Ok(Some(id));
        }
    }
    Ok(None)
}

/// Read a top-level `blueprintId` from a JSON metadata file.
fn read_blueprint_id_from_metadata(cwd: &Path) -> Result<Option<u64>> {
    let path = cwd.join("metadata").join("blueprint-metadata.json");
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).with_context(|| format!("reading {path:?}"))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {path:?}"))?;
    Ok(value.get("blueprintId").and_then(|v| v.as_u64()))
}

fn build_tx_context(network: &TangleClientArgs) -> Result<TxContext> {
    let cfg = network.client_config(0, None)?;
    Ok(TxContext {
        http_rpc_url: cfg.http_rpc_endpoint.clone(),
        tangle_contract: cfg.settings.tangle_contract,
        keystore_path: PathBuf::from(cfg.keystore_uri.clone()),
    })
}

fn signer_address(ctx: &TxContext) -> Result<alloy_primitives::Address> {
    use crate::command::signer::load_evm_signer;
    let signer = load_evm_signer(&ctx.keystore_path)?;
    Ok(signer.operator_address)
}

// ─────────────────────────────────────────────────────────────────────────────
// Build / hash / pin / attest
// ─────────────────────────────────────────────────────────────────────────────

fn resolve_binary(workspace: &DetectedWorkspace, args: &ShipArgs) -> Result<PathBuf> {
    if let Some(path) = &args.binary {
        if !path.exists() {
            bail!("--binary {} does not exist", path.display());
        }
        return Ok(path.clone());
    }
    if args.no_build {
        bail!(
            "--no-build was passed but no --binary supplied; pass a path to the pre-built artifact"
        );
    }
    let want_build = if args.yes {
        true
    } else {
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Build a release binary now?")
            .default(true)
            .interact()?
    };
    if !want_build {
        bail!("no binary available — pass --binary or accept the build prompt");
    }

    let mut cmd = Command::new("cargo");
    cmd.current_dir(&workspace.cwd)
        .arg("build")
        .arg("--release");
    if let Some(pkg) = &workspace.package {
        cmd.arg("-p").arg(pkg);
    }
    println!(
        "  {} cargo build --release{}",
        style("> Building:").cyan().bold(),
        workspace
            .package
            .as_ref()
            .map(|p| format!(" -p {p}"))
            .unwrap_or_default()
    );
    let status = cmd
        .status()
        .with_context(|| "spawning cargo build --release".to_string())?;
    if !status.success() {
        bail!("cargo build --release failed (exit {:?})", status.code());
    }

    let target_dir = workspace.cwd.join("target").join("release");
    let binary_name = workspace
        .package
        .clone()
        .or_else(|| guess_default_binary_name(&workspace.cwd))
        .ok_or_else(|| {
            eyre!(
                "could not determine release binary name; pass --binary <path-to-target/release/foo>"
            )
        })?;
    let path = target_dir.join(&binary_name);
    if !path.exists() {
        bail!(
            "expected built binary at {} but the file is missing — pass --binary <path>",
            path.display()
        );
    }
    Ok(path)
}

/// Cheap heuristic: read `Cargo.toml` and pick the first `[[bin]] name = "..."`
/// or fall back to the workspace's `package.name`. Anything more elaborate
/// is `--binary <path>` territory.
fn guess_default_binary_name(cwd: &Path) -> Option<String> {
    let manifest = cwd.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).ok()?;
    // Naive scan — avoid pulling in a full toml dependency on the hot path.
    for (i, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("[[bin]]") {
            // Look ahead a handful of lines for a `name = "..."` entry.
            for follow in text.lines().skip(i + 1).take(6) {
                let t = follow.trim();
                if let Some(rest) = t.strip_prefix("name") {
                    let raw = rest.trim_start_matches([' ', '=', '"']).trim();
                    if let Some(end) = raw.find('"') {
                        return Some(raw[..end].to_string());
                    }
                }
                if t.starts_with('[') {
                    break;
                }
            }
        }
    }
    // Fallback: `[package] name = "..."`
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("name") {
            let raw = rest.trim_start_matches([' ', '=', '"']).trim();
            if let Some(end) = raw.find('"') {
                return Some(raw[..end].to_string());
            }
        }
    }
    None
}

async fn resolve_binary_uri(binary: &Path, args: &ShipArgs) -> Result<String> {
    if let Some(uri) = &args.binary_uri {
        return Ok(uri.clone());
    }
    if args.pin_ipfs {
        let pinned: PinnedArtifact = pin_file_to_ipfs(binary).await?;
        return Ok(pinned.uri);
    }
    if args.yes {
        bail!("--yes was passed but no IPFS strategy: pass --binary-uri or --pin-ipfs");
    }
    let want_pin = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Pin binary to IPFS now? (uses IPFS_API_URL+IPFS_API_TOKEN or PINATA_JWT)")
        .default(true)
        .interact()?;
    if want_pin {
        let pinned = pin_file_to_ipfs(binary).await?;
        return Ok(pinned.uri);
    }
    let uri: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Existing binary URI (ipfs://... or https://...)")
        .interact_text()?;
    if uri.trim().is_empty() {
        bail!("a binaryUri is required");
    }
    Ok(uri.trim().to_string())
}

fn resolve_attestation_hash(args: &ShipArgs) -> Result<B256> {
    if let Some(hex) = &args.attestation_hash {
        return parse_b256(hex, "--attestation-hash");
    }
    if let Some(path) = &args.attestation_bundle {
        if !path.exists() {
            bail!("--attestation-bundle {} does not exist", path.display());
        }
        let (digest, _) = hash_file(path)?;
        return Ok(digest);
    }
    Ok(B256::ZERO)
}

fn decide_promote(args: &ShipArgs) -> Result<bool> {
    if args.no_promote {
        return Ok(false);
    }
    if args.promote || args.yes {
        return Ok(true);
    }
    Ok(Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Promote to active version (setActiveBinaryVersion)?")
        .default(true)
        .interact()?)
}

fn parse_service_ids(csv: &str) -> Result<Vec<u64>> {
    csv.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<u64>()
                .with_context(|| format!("parsing service id `{s}`"))
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Pretty-printers
// ─────────────────────────────────────────────────────────────────────────────

fn print_header(
    detected: &DetectedWorkspace,
    network: &TangleClientArgs,
    signer: alloy_primitives::Address,
    args: &ShipArgs,
) -> Result<()> {
    println!(
        "{}  blueprintId={}  ({})",
        style("🚀 Shipping blueprint:").cyan().bold(),
        detected.blueprint_id,
        detected.cwd.display(),
    );
    if let Ok(http) = network.http_rpc_url() {
        println!("  {}    {}", style("RPC").bold(), http);
    }
    println!("  {} {}", style("Wallet").bold(), signer);
    if args.dry_run {
        println!(
            "  {}",
            style("(dry-run — no transactions will be sent)").yellow()
        );
    }
    Ok(())
}

fn print_dry_run_summary(
    detected: &DetectedWorkspace,
    sha256: &B256,
    binary_uri: &str,
    attestation: &B256,
    promote: bool,
    json: bool,
) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "event": "ship_dry_run",
                "blueprint_id": detected.blueprint_id,
                "sha256": format!("{sha256:#x}"),
                "binary_uri": binary_uri,
                "attestation_hash": format!("{attestation:#x}"),
                "would_promote": promote,
            }))
            .unwrap()
        );
        return;
    }
    println!();
    println!(
        "{}",
        style("── Dry-run summary ──────────────────────────────").dim()
    );
    println!("  blueprint_id     {}", detected.blueprint_id);
    println!("  sha256           {sha256:#x}");
    println!("  binaryUri        {binary_uri}");
    println!("  attestationHash  {attestation:#x}");
    println!("  would_promote    {promote}");
    println!("  No transactions submitted.");
}

fn print_shipped_summary(
    detected: &DetectedWorkspace,
    result: &PublishVersionResult,
    promoted: bool,
    json: bool,
) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "event": "ship_complete",
                "blueprint_id": detected.blueprint_id,
                "version_id": result.version_id,
                "sha256": format!("{:#x}", result.sha256_hash),
                "binary_uri": result.binary_uri,
                "attestation_hash": format!("{:#x}", result.attestation_hash),
                "tx_hash": format!("{:#x}", result.tx_hash),
                "block_number": result.block_number,
                "promoted": promoted,
            }))
            .unwrap()
        );
        return;
    }
    println!();
    let banner = format!(
        "── Shipped v{} ──────────────────────────────",
        result.version_id
    );
    println!("{}", style(&banner).green().bold());
    println!("  sha256     {:#x}", result.sha256_hash);
    println!("  binaryUri  {}", result.binary_uri);
    println!("  promoted   {}", promoted);
    if let Some(block) = result.block_number {
        println!("  block      {block}");
    }
    println!("  tx_hash    {:#x}", result.tx_hash);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_settings(dir: &Path, contents: &str) {
        let mut f = std::fs::File::create(dir.join("settings.env")).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
    }

    #[test]
    fn settings_env_with_id_is_parsed() {
        let tmp = tempfile::tempdir().unwrap();
        write_settings(tmp.path(), "FOO=bar\nBLUEPRINT_ID=42\n");
        assert_eq!(
            read_blueprint_id_from_env_file(tmp.path()).unwrap(),
            Some(42)
        );
    }

    #[test]
    fn settings_env_with_quoted_id_works() {
        // Real settings files in the wild quote env values; the parser must
        // strip both single and double quotes before parsing.
        let tmp = tempfile::tempdir().unwrap();
        write_settings(tmp.path(), "BLUEPRINT_ID=\"99\"\n");
        assert_eq!(
            read_blueprint_id_from_env_file(tmp.path()).unwrap(),
            Some(99)
        );
    }

    #[test]
    fn settings_env_zero_treated_as_unset() {
        // The example file ships `BLUEPRINT_ID=0` as a placeholder — if we
        // accepted it literally we'd ship onto a nonexistent blueprint.
        let tmp = tempfile::tempdir().unwrap();
        write_settings(tmp.path(), "BLUEPRINT_ID=0\n");
        assert_eq!(read_blueprint_id_from_env_file(tmp.path()).unwrap(), None);
    }

    #[test]
    fn settings_env_missing_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(read_blueprint_id_from_env_file(tmp.path()).unwrap(), None);
    }

    #[test]
    fn metadata_with_blueprint_id_field_parses() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("metadata")).unwrap();
        std::fs::write(
            tmp.path().join("metadata").join("blueprint-metadata.json"),
            r#"{"name":"X","blueprintId":7}"#,
        )
        .unwrap();
        assert_eq!(
            read_blueprint_id_from_metadata(tmp.path()).unwrap(),
            Some(7)
        );
    }

    #[test]
    fn parse_service_ids_supports_csv_with_whitespace() {
        assert_eq!(parse_service_ids("42, 43,44 ").unwrap(), vec![42, 43, 44]);
        assert_eq!(parse_service_ids("").unwrap(), Vec::<u64>::new());
        assert!(parse_service_ids("not-a-number").is_err());
    }

    #[test]
    fn guess_binary_name_prefers_first_bin_section() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest = r#"
[package]
name = "wrapper"
version = "0.1.0"

[[bin]]
name = "wrapped-bin"
path = "src/main.rs"
"#;
        std::fs::write(tmp.path().join("Cargo.toml"), manifest).unwrap();
        assert_eq!(
            guess_default_binary_name(tmp.path()).as_deref(),
            Some("wrapped-bin")
        );
    }

    #[test]
    fn guess_binary_name_falls_back_to_package_name() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"lonely\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        assert_eq!(
            guess_default_binary_name(tmp.path()).as_deref(),
            Some("lonely")
        );
    }
}
