use alloy_primitives::Address;
use blueprint_client_tangle::contracts::ITangleTypes;
use cargo_tangle::command::tangle::parse_address;
use color_eyre::eyre::{Result, ensure, eyre};
use std::str::FromStr;

pub(super) fn parse_address_list(values: &[String], label: &str) -> Result<Vec<Address>> {
    values
        .iter()
        .map(|value| parse_address(value, label))
        .collect()
}

pub(super) fn normalize_operator_exposures(
    exposures: &[u16],
    operator_len: usize,
) -> Result<Option<Vec<u16>>> {
    if exposures.is_empty() {
        return Ok(None);
    }
    ensure!(
        exposures.len() == operator_len,
        "Expected {operator_len} operator exposure values but received {}",
        exposures.len()
    );
    exposures
        .iter()
        .enumerate()
        .try_for_each(|(idx, value)| -> Result<()> {
            ensure!(
                *value <= MAX_BPS,
                "Operator exposure #{idx} exceeds {MAX_BPS} bps"
            );
            Ok(())
        })?;
    Ok(Some(exposures.to_vec()))
}

#[derive(Clone, Debug)]
pub(super) struct SecurityRequirementArg {
    kind: AssetKindArg,
    token: Address,
    min: u16,
    max: u16,
}

#[derive(Clone, Debug)]
pub(super) struct SecurityCommitmentArg {
    kind: AssetKindArg,
    token: Address,
    exposure: u16,
}

#[derive(Clone, Copy, Debug)]
enum AssetKindArg {
    Native,
    Erc20,
}

pub(super) fn parse_security_requirement(
    value: &str,
) -> std::result::Result<SecurityRequirementArg, String> {
    let parts: Vec<_> = value.split(':').collect();
    if parts.len() != 4 {
        return Err("Expected format KIND:TOKEN:MIN:MAX".to_string());
    }
    let kind = parse_asset_kind(parts[0])?;
    let token = parse_token(kind, parts[1])?;
    let min = parse_bps(parts[2], "min exposure")?;
    let max = parse_bps(parts[3], "max exposure")?;
    if min == 0 {
        return Err("minimum exposure must be greater than 0".to_string());
    }
    if min > max {
        return Err("minimum exposure cannot exceed maximum exposure".to_string());
    }
    Ok(SecurityRequirementArg {
        kind,
        token,
        min,
        max,
    })
}

pub(super) fn parse_security_commitment(
    value: &str,
) -> std::result::Result<SecurityCommitmentArg, String> {
    let parts: Vec<_> = value.split(':').collect();
    if parts.len() != 3 {
        return Err("Expected format KIND:TOKEN:EXPOSURE".to_string());
    }
    let kind = parse_asset_kind(parts[0])?;
    let token = parse_token(kind, parts[1])?;
    let exposure = parse_bps(parts[2], "exposure")?;
    Ok(SecurityCommitmentArg {
        kind,
        token,
        exposure,
    })
}

fn parse_asset_kind(value: &str) -> std::result::Result<AssetKindArg, String> {
    match value.to_lowercase().as_str() {
        "native" | "eth" => Ok(AssetKindArg::Native),
        "erc20" => Ok(AssetKindArg::Erc20),
        other => Err(format!("unsupported asset kind '{other}'")),
    }
}

fn parse_token(kind: AssetKindArg, raw: &str) -> std::result::Result<Address, String> {
    match kind {
        AssetKindArg::Native => {
            if raw.is_empty() || raw == "_" || raw == "0" {
                return Ok(Address::ZERO);
            }
            if raw.eq_ignore_ascii_case("native") {
                return Ok(Address::ZERO);
            }
            let addr = Address::from_str(raw)
                .map_err(|e| format!("invalid native token placeholder '{raw}': {e}"))?;
            if addr != Address::ZERO {
                return Err("native asset must use the zero address placeholder".to_string());
            }
            Ok(Address::ZERO)
        }
        AssetKindArg::Erc20 => {
            Address::from_str(raw).map_err(|e| format!("invalid ERC-20 address '{raw}': {e}"))
        }
    }
}

fn parse_bps(value: &str, label: &str) -> std::result::Result<u16, String> {
    let parsed = value
        .parse::<u16>()
        .map_err(|e| format!("invalid {label} '{value}': {e}"))?;
    if parsed > MAX_BPS {
        return Err(format!("{label} '{value}' exceeds {MAX_BPS} bps"));
    }
    Ok(parsed)
}

pub(super) fn requirement_to_abi(
    arg: SecurityRequirementArg,
) -> ITangleTypes::AssetSecurityRequirement {
    ITangleTypes::AssetSecurityRequirement {
        asset: asset_to_abi(arg.kind, arg.token),
        minExposureBps: arg.min,
        maxExposureBps: arg.max,
    }
}

fn commitment_to_abi(arg: SecurityCommitmentArg) -> ITangleTypes::AssetSecurityCommitment {
    ITangleTypes::AssetSecurityCommitment {
        asset: asset_to_abi(arg.kind, arg.token),
        exposureBps: arg.exposure,
    }
}

/// Parse a list of commitment strings into ABI-compatible commitment structures.
pub(super) fn parse_commitments(
    commitments: &[String],
) -> Result<Vec<ITangleTypes::AssetSecurityCommitment>> {
    commitments
        .iter()
        .map(|s| {
            parse_security_commitment(s)
                .map(commitment_to_abi)
                .map_err(|e| eyre!("Invalid commitment '{}': {}", s, e))
        })
        .collect()
}

fn asset_to_abi(kind: AssetKindArg, token: Address) -> ITangleTypes::Asset {
    let kind_value = match kind {
        AssetKindArg::Native => ITangleTypes::AssetKind::from_underlying(0).into_underlying(),
        AssetKindArg::Erc20 => ITangleTypes::AssetKind::from_underlying(1).into_underlying(),
    };
    ITangleTypes::Asset {
        kind: kind_value,
        token,
    }
}

pub(super) const MAX_BPS: u16 = 10_000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposures_are_optional() {
        assert!(normalize_operator_exposures(&[], 2).unwrap().is_none());

        let exposures = normalize_operator_exposures(&[5_000, 5_000], 2)
            .unwrap()
            .expect("exposures present");
        assert_eq!(exposures, vec![5_000, 5_000]);
    }

    #[test]
    fn exposures_length_enforced() {
        let err = normalize_operator_exposures(&[5_000], 2).unwrap_err();
        assert!(
            err.to_string()
                .contains("Expected 2 operator exposure values")
        );
    }

    #[test]
    fn exposures_bps_capped() {
        let err = normalize_operator_exposures(&[10_001], 1).unwrap_err();
        assert!(err.to_string().contains("exceeds"));
    }

    #[test]
    fn parse_security_requirement_native() {
        let arg = parse_security_requirement("native:_:100:200").expect("valid requirement");
        assert!(matches!(arg.kind, AssetKindArg::Native));
        assert_eq!(arg.token, Address::ZERO);
        assert_eq!(arg.min, 100);
        assert_eq!(arg.max, 200);
    }

    #[test]
    fn parse_security_requirement_checks_bounds() {
        let err = parse_security_requirement("native:_:0:0").unwrap_err();
        assert!(err.contains("minimum exposure"));

        let err = parse_security_requirement("native:_:200:100").unwrap_err();
        assert!(err.contains("cannot exceed"));
    }

    #[test]
    fn parse_security_commitment_erc20() {
        let token = "0x0000000000000000000000000000000000000001";
        let arg =
            parse_security_commitment(&format!("erc20:{token}:7500")).expect("valid commitment");
        assert!(matches!(arg.kind, AssetKindArg::Erc20));
        assert_eq!(arg.token, Address::from_str(token).unwrap());
        assert_eq!(arg.exposure, 7_500);
    }
}
