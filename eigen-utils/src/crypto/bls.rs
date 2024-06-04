use alloy_primitives::U256;

use ark_bn254::{Bn254, Fq, Fr, G1Affine, G2Affine};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::PrimeField;
use ark_ff::{BigInt, QuadExtField, Zero};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Valid};
use ark_std::One;
use ark_std::UniformRand;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::bn254::map_to_curve;
use super::pairing_products::{InnerProduct, PairingInnerProduct};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EncryptedBLSKeyJSONV3 {
    pub pub_key: String,
    pub crypto: serde_json::Value, // Adjust this type to match your specific encryption structure
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct G1Point {
    pub x: U256,
    pub y: U256,
}

impl CanonicalSerialize for G1Point {
    fn serialize_with_mode<W: std::io::prelude::Write>(
        &self,
        writer: W,
        compress: ark_serialize::Compress,
    ) -> Result<(), ark_serialize::SerializationError> {
        let affine = G1Affine::new(
            Fq::from(BigInt(self.x.into_limbs())),
            Fq::from(BigInt(self.y.into_limbs())),
        );

        affine.serialize_with_mode(writer, compress)
    }

    fn serialized_size(&self, compress: ark_serialize::Compress) -> usize {
        let affine = G1Affine::new(
            Fq::from(BigInt(self.x.into_limbs())),
            Fq::from(BigInt(self.y.into_limbs())),
        );

        affine.serialized_size(compress)
    }
}

impl Valid for G1Point {
    fn check(&self) -> Result<(), ark_serialize::SerializationError> {
        let affine = G1Affine::new(
            Fq::from(BigInt(self.x.into_limbs())),
            Fq::from(BigInt(self.y.into_limbs())),
        );

        affine.check()
    }
}

impl CanonicalDeserialize for G1Point {
    fn deserialize_with_mode<R: std::io::prelude::Read>(
        reader: R,
        compress: ark_serialize::Compress,
        validate: ark_serialize::Validate,
    ) -> Result<Self, ark_serialize::SerializationError> {
        let affine = G1Affine::deserialize_with_mode(reader, compress, validate)?;
        Ok(Self {
            x: U256::from_limbs(affine.x.0 .0),
            y: U256::from_limbs(affine.y.0 .0),
        })
    }
}

impl G1Point {
    pub fn new(x: Fq, y: Fq) -> Self {
        Self {
            x: U256::from_limbs(x.0 .0),
            y: U256::from_limbs(y.0 .0),
        }
    }

    pub fn zero() -> Self {
        Self::new(Fq::zero(), Fq::zero())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut ser_buf = vec![0; self.serialized_size(ark_serialize::Compress::Yes)];
        self.serialize_compressed(&mut ser_buf);
        ser_buf
    }

    pub fn neg(&self) -> Self {
        let affine = G1Affine::new(
            Fq::from(BigInt(self.x.into_limbs())),
            Fq::from(BigInt(self.y.into_limbs())),
        );

        let neg_affine = -affine;
        Self {
            x: U256::from_limbs(neg_affine.x.0 .0),
            y: U256::from_limbs(neg_affine.y.0 .0),
        }
    }

    pub fn generator() -> Self {
        let gen = G1Affine::generator();
        Self {
            x: U256::from_limbs(gen.x.0 .0),
            y: U256::from_limbs(gen.y.0 .0),
        }
    }

    pub fn add(&mut self, _other: &G1Point) {
        let affine_p1 = G1Affine::new(
            Fq::from(BigInt(self.x.into_limbs())),
            Fq::from(BigInt(self.y.into_limbs())),
        );

        let affine_p2 = G1Affine::new(
            Fq::from(BigInt(self.x.into_limbs())),
            Fq::from(BigInt(self.y.into_limbs())),
        );

        let pt = (affine_p1 + affine_p2).into_affine();
        self.x = U256::from_limbs(pt.x.0 .0);
        self.y = U256::from_limbs(pt.y.0 .0);
    }

    pub fn sub(&mut self, _other: &G1Point) {
        let affine_p1 = G1Affine::new(
            Fq::from(BigInt(self.x.into_limbs())),
            Fq::from(BigInt(self.y.into_limbs())),
        );

        let affine_p2 = G1Affine::new(
            Fq::from(BigInt(self.x.into_limbs())),
            Fq::from(BigInt(self.y.into_limbs())),
        );

        let pt = (affine_p1 - affine_p2).into_affine();
        self.x = U256::from_limbs(pt.x.0 .0);
        self.y = U256::from_limbs(pt.y.0 .0);
    }

    pub fn mul(&mut self, scalar: Fq) {
        let affine = G1Affine::new(
            Fq::from(BigInt(self.x.into_limbs())),
            Fq::from(BigInt(self.y.into_limbs())),
        );

        let pt = affine.mul_bigint(scalar.0).into_affine();
        self.x = U256::from_limbs(pt.x.0 .0);
        self.y = U256::from_limbs(pt.y.0 .0);
    }

    pub fn from_ark_g1(ark_g1: &G1Affine) -> Self {
        Self {
            x: U256::from_limbs(ark_g1.x.0 .0),
            y: U256::from_limbs(ark_g1.y.0 .0),
        }
    }

    pub fn to_ark_g1(&self) -> G1Affine {
        G1Affine::new(
            Fq::from(BigInt(self.x.into_limbs())),
            Fq::from(BigInt(self.y.into_limbs())),
        )
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct G2Point {
    pub x: [U256; 2],
    pub y: [U256; 2],
}

impl CanonicalSerialize for G2Point {
    fn serialize_with_mode<W: std::io::prelude::Write>(
        &self,
        writer: W,
        compress: ark_serialize::Compress,
    ) -> Result<(), ark_serialize::SerializationError> {
        let affine = G2Affine::new(
            QuadExtField {
                c0: Fq::from(BigInt(self.x[0].into_limbs())),
                c1: Fq::from(BigInt(self.x[1].into_limbs())),
            },
            QuadExtField {
                c0: Fq::from(BigInt(self.y[0].into_limbs())),
                c1: Fq::from(BigInt(self.y[1].into_limbs())),
            },
        );

        affine.serialize_with_mode(writer, compress)
    }

    fn serialized_size(&self, compress: ark_serialize::Compress) -> usize {
        let affine = G2Affine::new(
            QuadExtField {
                c0: Fq::from(BigInt(self.x[0].into_limbs())),
                c1: Fq::from(BigInt(self.x[1].into_limbs())),
            },
            QuadExtField {
                c0: Fq::from(BigInt(self.y[0].into_limbs())),
                c1: Fq::from(BigInt(self.y[1].into_limbs())),
            },
        );

        affine.serialized_size(compress)
    }
}

impl Valid for G2Point {
    fn check(&self) -> Result<(), ark_serialize::SerializationError> {
        let affine = G2Affine::new(
            QuadExtField {
                c0: Fq::from(BigInt(self.x[0].into_limbs())),
                c1: Fq::from(BigInt(self.x[1].into_limbs())),
            },
            QuadExtField {
                c0: Fq::from(BigInt(self.y[0].into_limbs())),
                c1: Fq::from(BigInt(self.y[1].into_limbs())),
            },
        );

        affine.check()
    }
}

impl CanonicalDeserialize for G2Point {
    fn deserialize_with_mode<R: std::io::prelude::Read>(
        reader: R,
        compress: ark_serialize::Compress,
        validate: ark_serialize::Validate,
    ) -> Result<Self, ark_serialize::SerializationError> {
        let affine = G2Affine::deserialize_with_mode(reader, compress, validate)?;
        Ok(Self {
            x: [
                U256::from_limbs(affine.x.c0.0 .0),
                U256::from_limbs(affine.x.c1.0 .0),
            ],
            y: [
                U256::from_limbs(affine.y.c0.0 .0),
                U256::from_limbs(affine.y.c1.0 .0),
            ],
        })
    }
}

impl G2Point {
    pub fn new(x: [Fr; 2], y: [Fr; 2]) -> Self {
        Self {
            x: [U256::from_limbs(x[0].0 .0), U256::from_limbs(x[1].0 .0)],
            y: [U256::from_limbs(y[0].0 .0), U256::from_limbs(y[1].0 .0)],
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut ser_buf = vec![0; self.serialized_size(ark_serialize::Compress::Yes)];
        self.serialize_compressed(&mut ser_buf);
        ser_buf
    }

    pub fn neg(&self) -> Self {
        let affine = G2Affine::new(
            QuadExtField {
                c0: Fq::from(BigInt(self.x[0].into_limbs())),
                c1: Fq::from(BigInt(self.x[1].into_limbs())),
            },
            QuadExtField {
                c0: Fq::from(BigInt(self.y[0].into_limbs())),
                c1: Fq::from(BigInt(self.y[1].into_limbs())),
            },
        );

        let neg_affine = -affine;
        Self {
            x: [
                U256::from_limbs(neg_affine.x.c0.0 .0),
                U256::from_limbs(neg_affine.x.c1.0 .0),
            ],
            y: [
                U256::from_limbs(neg_affine.y.c0.0 .0),
                U256::from_limbs(neg_affine.y.c1.0 .0),
            ],
        }
    }

    pub fn zero() -> Self {
        Self::new([Fr::zero(), Fr::zero()], [Fr::zero(), Fr::zero()])
    }

    pub fn generator() -> Self {
        let gen = G2Affine::generator();
        Self {
            x: [
                U256::from_limbs(gen.x.c0.0 .0),
                U256::from_limbs(gen.x.c1.0 .0),
            ],
            y: [
                U256::from_limbs(gen.y.c0.0 .0),
                U256::from_limbs(gen.y.c1.0 .0),
            ],
        }
    }

    pub fn add(&mut self, other: &G2Point) {
        let affine_p1 = G2Affine::new(
            QuadExtField {
                c0: Fq::from(BigInt(self.x[0].into_limbs())),
                c1: Fq::from(BigInt(self.x[1].into_limbs())),
            },
            QuadExtField {
                c0: Fq::from(BigInt(self.y[0].into_limbs())),
                c1: Fq::from(BigInt(self.y[1].into_limbs())),
            },
        );

        let affine_p2 = G2Affine::new(
            QuadExtField {
                c0: Fq::from(BigInt(other.x[0].into_limbs())),
                c1: Fq::from(BigInt(other.x[1].into_limbs())),
            },
            QuadExtField {
                c0: Fq::from(BigInt(other.y[0].into_limbs())),
                c1: Fq::from(BigInt(other.y[1].into_limbs())),
            },
        );

        let pt = (affine_p1 + affine_p2).into_affine();
        self.x = [
            U256::from_limbs(pt.x.c0.0 .0),
            U256::from_limbs(pt.x.c1.0 .0),
        ];
        self.y = [
            U256::from_limbs(pt.y.c0.0 .0),
            U256::from_limbs(pt.y.c1.0 .0),
        ];
    }

    pub fn sub(&mut self, other: &G2Point) {
        let affine_p1 = G2Affine::new(
            QuadExtField {
                c0: Fq::from(BigInt(self.x[0].into_limbs())),
                c1: Fq::from(BigInt(self.x[1].into_limbs())),
            },
            QuadExtField {
                c0: Fq::from(BigInt(self.y[0].into_limbs())),
                c1: Fq::from(BigInt(self.y[1].into_limbs())),
            },
        );

        let affine_p2 = G2Affine::new(
            QuadExtField {
                c0: Fq::from(BigInt(other.x[0].into_limbs())),
                c1: Fq::from(BigInt(other.x[1].into_limbs())),
            },
            QuadExtField {
                c0: Fq::from(BigInt(other.y[0].into_limbs())),
                c1: Fq::from(BigInt(other.y[1].into_limbs())),
            },
        );

        let pt = (affine_p1 - affine_p2).into_affine();
        self.x = [
            U256::from_limbs(pt.x.c0.0 .0),
            U256::from_limbs(pt.x.c1.0 .0),
        ];
        self.y = [
            U256::from_limbs(pt.y.c0.0 .0),
            U256::from_limbs(pt.y.c1.0 .0),
        ];
    }

    pub fn mul(&mut self, scalar: Fq) {
        let affine = G2Affine::new(
            QuadExtField {
                c0: Fq::from(BigInt(self.x[0].into_limbs())),
                c1: Fq::from(BigInt(self.x[1].into_limbs())),
            },
            QuadExtField {
                c0: Fq::from(BigInt(self.y[0].into_limbs())),
                c1: Fq::from(BigInt(self.y[1].into_limbs())),
            },
        );

        let pt = affine.mul_bigint(scalar.0).into_affine();
        self.x = [
            U256::from_limbs(pt.x.c0.0 .0),
            U256::from_limbs(pt.x.c1.0 .0),
        ];
        self.y = [
            U256::from_limbs(pt.y.c0.0 .0),
            U256::from_limbs(pt.y.c1.0 .0),
        ];
    }

    pub fn from_ark_g2(ark_g2: &G2Affine) -> Self {
        Self {
            x: [
                U256::from_limbs(ark_g2.x.c0.0 .0),
                U256::from_limbs(ark_g2.x.c1.0 .0),
            ],
            y: [
                U256::from_limbs(ark_g2.y.c0.0 .0),
                U256::from_limbs(ark_g2.y.c1.0 .0),
            ],
        }
    }

    pub fn to_ark_g2(&self) -> G2Affine {
        G2Affine::new(
            QuadExtField {
                c0: Fq::from(BigInt(self.x[0].into_limbs())),
                c1: Fq::from(BigInt(self.x[1].into_limbs())),
            },
            QuadExtField {
                c0: Fq::from(BigInt(self.y[0].into_limbs())),
                c1: Fq::from(BigInt(self.y[1].into_limbs())),
            },
        )
    }
}

#[derive(Clone, Debug, Default, CanonicalSerialize, CanonicalDeserialize)]
pub struct Signature {
    pub g1_point: G1Point,
}

impl Signature {
    pub fn new_zero() -> Self {
        Self {
            g1_point: G1Point::zero(),
        }
    }

    pub fn add(&mut self, other: &Signature) {
        self.g1_point.add(&other.g1_point);
    }

    pub fn verify(&self, pubkey: &G2Point, message: &[u8; 32]) -> Result<bool, String> {
        let g2_gen = G2Point::generator();
        let msg_point = map_to_curve(message);
        let neg_sig = self.g1_point.neg();
        let p: [G1Point; 2] = [msg_point, neg_sig];
        let q: [G2Point; 2] = [pubkey.clone(), g2_gen];

        let p_projective = [
            G1Affine::new(
                Fq::from(BigInt(p[0].x.into_limbs())),
                Fq::from(BigInt(p[0].y.into_limbs())),
            )
            .mul_bigint(Fr::one().0),
            G1Affine::new(
                Fq::from(BigInt(p[1].x.into_limbs())),
                Fq::from(BigInt(p[1].y.into_limbs())),
            )
            .mul_bigint(Fr::one().0),
        ];

        let q_projective = [
            G2Affine::new(
                QuadExtField {
                    c0: Fq::from(BigInt(q[0].x[0].into_limbs())),
                    c1: Fq::from(BigInt(q[0].x[1].into_limbs())),
                },
                QuadExtField {
                    c0: Fq::from(BigInt(q[0].y[0].into_limbs())),
                    c1: Fq::from(BigInt(q[0].y[1].into_limbs())),
                },
            )
            .mul_bigint(Fr::one().0),
            G2Affine::new(
                QuadExtField {
                    c0: Fq::from(BigInt(q[1].x[0].into_limbs())),
                    c1: Fq::from(BigInt(q[1].x[1].into_limbs())),
                },
                QuadExtField {
                    c0: Fq::from(BigInt(q[1].y[0].into_limbs())),
                    c1: Fq::from(BigInt(q[1].y[1].into_limbs())),
                },
            )
            .mul_bigint(Fr::one().0),
        ];

        let inner_product =
            PairingInnerProduct::<Bn254>::inner_product(&p_projective[..], &q_projective[..])
                .unwrap();
        Ok(inner_product.0 == QuadExtField::one())
    }
}

#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct PrivateKey {
    pub key: Fq,
}

pub struct KeyPair {
    pub priv_key: PrivateKey,
    pub pub_key: G1Point,
}

impl KeyPair {
    pub fn new(sk: &PrivateKey) -> Self {
        let pub_key_point = G1Affine::generator().mul_bigint(sk.key.0).into_affine();
        Self {
            priv_key: sk.clone(),
            pub_key: G1Point {
                x: U256::from_limbs(pub_key_point.x.0 .0),
                y: U256::from_limbs(pub_key_point.y.0 .0),
            },
        }
    }

    pub fn gen_random() -> Result<Self, String> {
        let mut rng = rand::thread_rng();
        let key = Fq::rand(&mut rng);
        let priv_key = PrivateKey { key };
        Ok(Self::new(&priv_key))
    }

    pub fn save_to_file(&self, path: &str, password: &str) -> Result<(), String> {
        let mut sk_bytes = vec![0; self.priv_key.compressed_size()];
        self.priv_key.serialize_compressed(&mut sk_bytes);

        let crypto_struct = serde_json::json!({
            "encrypted_data": base64::encode(sk_bytes), // Implement proper encryption logic here
            "password_hash": base64::encode(password),  // Implement proper password handling
        });

        let encrypted_bls_struct = EncryptedBLSKeyJSONV3 {
            pub_key: format!("{:?}", self.pub_key),
            crypto: crypto_struct,
        };

        let data = serde_json::to_string(&encrypted_bls_struct).map_err(|e| e.to_string())?;
        let dir = Path::new(path).parent().ok_or("Invalid path")?;
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        fs::write(path, data).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn read_from_file(path: &str, _password: &str) -> Result<Self, String> {
        let key_store_contents = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let encrypted_bls_struct: EncryptedBLSKeyJSONV3 =
            serde_json::from_str(&key_store_contents).map_err(|e| e.to_string())?;

        if encrypted_bls_struct.pub_key.is_empty() {
            return Err("Invalid BLS key file. PubKey field not found.".to_string());
        }

        let sk_bytes = base64::decode(
            encrypted_bls_struct.crypto["encrypted_data"]
                .as_str()
                .ok_or("Invalid data")?,
        )
        .map_err(|e| e.to_string())?;
        let priv_key = Fq::from_le_bytes_mod_order(&sk_bytes);
        Ok(Self::new(&PrivateKey { key: priv_key }))
    }

    pub fn sign_message(&self, message: &[u8; 32]) -> Signature {
        let mut sig_point = map_to_curve(message);
        sig_point.mul(self.priv_key.key);
        Signature {
            g1_point: sig_point,
        }
    }

    pub fn sign_hashed_to_curve_message(&self, g1_hashed_msg: &G1Point) -> Signature {
        let mut sig_point = g1_hashed_msg.clone();
        sig_point.mul(self.priv_key.key);
        Signature {
            g1_point: sig_point,
        }
    }

    pub fn get_pub_key_g2(&self) -> G2Point {
        let mut pub_key_g2_point = G2Point::generator();
        pub_key_g2_point.mul(self.priv_key.key);

        pub_key_g2_point
    }

    pub fn get_pub_key_g1(&self) -> &G1Point {
        &self.pub_key
    }
}
