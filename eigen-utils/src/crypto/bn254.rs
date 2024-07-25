use crate::crypto::bls::{g1_point_to_g1_projective, G1Point};
use crate::types::AvsError;
use alloy_primitives::U256;
use ark_bn254::{Bn254, Fq2, Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::{BigInteger, BigInteger256};
use ark_ff::{Field, One, PrimeField};
use std::ops::Mul;
use std::str::FromStr;

use ark_bn254::Fq as F;

pub fn map_to_curve(digest: &[u8; 32]) -> G1Projective {
    let one = F::one();
    let three = F::from(3u64);
    let mut x = F::from_be_bytes_mod_order(&digest.as_slice());

    loop {
        let x_cubed = x.pow([3]);
        let mut y = x_cubed + three;

        if y.legendre().is_qr() {
            let y = y.sqrt().unwrap();
            let point = G1Affine::new(x, y);

            if point.is_on_curve() {
                return G1Projective::new(point.x, point.y, F::one());
            }
        }

        x += one;
    }
}

// Helper for converting a PrimeField to its U256 representation for Ethereum compatibility
pub fn u256_to_point<F: PrimeField>(point: U256) -> F {
    let le: [u8; 32] = point.to_le_bytes();
    F::from_le_bytes_mod_order(&le[..])
}

// Helper for converting a PrimeField to its U256 representation for Ethereum compatibility
// (U256 reads data as big endian)
pub fn point_to_u256<F: PrimeField>(point: F) -> U256 {
    let point = point.into_bigint();
    let point_bytes = point.to_bytes_be();
    U256::from_be_slice(&point_bytes[..])
}

/// Converts [U256] to [BigInteger256]
pub fn u256_to_bigint256(value: U256) -> BigInteger256 {
    // Convert U256 to a big-endian byte array
    let bytes: [u8; 32] = value.to_be_bytes();

    // BigInteger256 expects a 4-element array of 64-bit values in little-endian order
    let mut data = [0u64; 4];

    // Iterate over the bytes in chunks of 8 bytes and convert to u64
    for (i, chunk) in bytes.chunks(8).enumerate() {
        let mut chunk_array = [0u8; 8];
        chunk_array.copy_from_slice(chunk);
        data[3 - i] = u64::from_be_bytes(chunk_array);
    }

    BigInteger256::new(data)
}

pub fn biginteger256_to_u256(bi: BigInteger256) -> U256 {
    let s = bi.to_bytes_be();
    U256::from_be_slice(&s)
}

pub fn get_g1_generator() -> Result<G1Affine, AvsError> {
    let x_result = F::from_str("1");

    let y_result = F::from_str("2");

    match x_result {
        Ok(x) => match y_result {
            Ok(y) => Ok(G1Affine::new(x, y)),
            Err(_) => Err(AvsError::KeyError(
                "Invalid G1 Generator Y Result".to_string(),
            )),
        },
        Err(_) => Err(AvsError::KeyError(
            "Invalid G1 Generator X Result".to_string(),
        )),
    }
}

pub fn get_g2_generator() -> Result<G2Affine, AvsError> {
    let x_0_result = F::from_str(
        "10857046999023057135944570762232829481370756359578518086990519993285655852781",
    );

    let x_1result = F::from_str(
        "11559732032986387107991004021392285783925812861821192530917403151452391805634",
    );

    match x_0_result {
        Ok(x_0) => {
            match x_1result {
                Ok(x_1) => {
                    let x = Fq2::new(x_0, x_1);

                    let y_0_result = F::from_str("8495653923123431417604973247489272438418190587263600148770280649306958101930");

                    match y_0_result {
                        Ok(y_0) => {
                            let y_1_result = F::from_str("4082367875863433681332203403145435568316851327593401208105741076214120093531");

                            match y_1_result {
                                Ok(y_1) => {
                                    let y = Fq2::new(y_0, y_1);
                                    Ok(G2Affine::new(x, y))
                                }
                                Err(_) => {
                                    Err(AvsError::KeyError("Invalid G2 Generator Y1".to_string()))
                                }
                            }
                        }
                        Err(_) => Err(AvsError::KeyError("Invalid G2 Generator Y0".to_string())),
                    }
                }
                Err(_) => Err(AvsError::KeyError("Invalid G2 Generator X1".to_string())),
            }
        }
        Err(_) => Err(AvsError::KeyError("Invalid G2 Generator X0".to_string())),
    }
}

pub fn mul_by_generator_g1(pvt_key: Fr) -> Result<G1Projective, AvsError> {
    let g1_gen_result = get_g1_generator();

    match g1_gen_result {
        Ok(g1_gen) => {
            let s: G1Projective = g1_gen.into();
            Ok(s.mul(pvt_key))
        }
        Err(_) => Err(AvsError::KeyError(
            "Invalid G1 Generator Result".to_string(),
        )),
    }
}

pub fn mul_by_generator_g2(pvt_key: Fr) -> Result<G2Projective, AvsError> {
    let g2_gen_result = get_g2_generator();

    match g2_gen_result {
        Ok(g2_gen) => {
            let s: G2Projective = g2_gen.into();
            Ok(s.mul(pvt_key))
        }
        Err(_) => Err(AvsError::KeyError(
            "Invalid G2 Generator Result".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_u256_to_bigint256() {
        let u256 = U256::from(123456789);
        let result = u256_to_bigint256(u256);
        assert_eq!(result, BigInteger256::from(123456789u32));
    }

    #[tokio::test]
    async fn test_bigint256_to_u256() {
        let bi = BigInteger256::from(123456789u32);
        let result = biginteger256_to_u256(bi);
        assert_eq!(result, U256::from(123456789));
    }
}
