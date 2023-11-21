use std::path::PathBuf;

use ark_bn254::{Bn254, Fr as Bn254Fr};
use ark_circom::{CircomBuilder, CircomConfig, CircomReduction};
use ark_crypto_primitives::snark::SNARK;
use ark_ec::pairing::Pairing;
use ark_ff::Field;
use ark_groth16::Groth16;
use ark_poly::Radix2EvaluationDomain;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
use ark_serialize::CanonicalSerialize;
use ark_std::{cfg_chunks, cfg_into_iter, end_timer, start_timer, Zero};

use clap::Parser;
use groth16::qap::qap;
use serde_bytes::ByteBuf;
use tracing_subscriber::fmt::SubscriberBuilder;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use rand::SeedableRng;
use secret_sharing::pss::PackedSharingParams;
use zk_gadget::client_ext::job_types::{CircuitProperties, JobProperties, PackedQAPShare};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

fn pack_from_witness<E: Pairing>(
    pp: &PackedSharingParams<E::ScalarField>,
    full_assignment: Vec<E::ScalarField>,
) -> Vec<Vec<E::ScalarField>> {
    let packed_assignments = cfg_chunks!(full_assignment, pp.l)
        .map(|chunk| {
            let secrets = if chunk.len() < pp.l {
                let mut secrets = chunk.to_vec();
                secrets.resize(pp.l, E::ScalarField::zero());
                secrets
            } else {
                chunk.to_vec()
            };
            pp.pack_from_public(secrets)
        })
        .collect::<Vec<_>>();

    cfg_into_iter!(0..pp.n)
        .map(|i| {
            cfg_into_iter!(0..packed_assignments.len())
                .map(|j| packed_assignments[j][i])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

/// Client
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Circuit Id
    #[arg(long)]
    circuit_id: u64,
    /// Job Id
    #[arg(long)]
    job_id: u64,
    /// Circom wasm file
    #[arg(long)]
    wasm: PathBuf,
    /// Circom r1cs file
    #[arg(long)]
    r1cs: PathBuf,
    /// Circuit input as json file
    #[arg(long)]
    input: PathBuf,
    /// Public inputs as json file
    #[arg(long)]
    public_inputs: PathBuf,
    /// Output directory where we write the proving key and the shares.
    #[arg(long)]
    output_dir: PathBuf,
    /// Shall we generate the proving key?
    #[arg(long)]
    generate_proving_key: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_log();
    let args = Args::parse();
    let setup_timer = start_timer!(|| "Setup");
    let cfg = CircomConfig::<Bn254>::new(&args.wasm, &args.r1cs).unwrap();
    let mut builder = CircomBuilder::new(cfg);
    let rng = &mut ark_std::rand::rngs::StdRng::from_seed([42u8; 32]);
    let input = tokio::fs::read_to_string(args.input).await?;
    // input is a json map of string to BigInt values; BigInt is serialized as a string
    // {"a": 1, "b": 2}
    let input = serde_json::from_str::<serde_json::Value>(&input)?;
    let obj = match input.as_object() {
        Some(obj) => obj,
        None => anyhow::bail!("Input must be a json map of string to BigInt values"),
    };
    for (k, v) in obj {
        match v {
            serde_json::Value::Number(u) => builder.push_input(k, u.as_u64().unwrap()),
            _ => anyhow::bail!("Input must be a json map of string to BigInt values"),
        }
    }
    // read the public inputs
    // public inputs is a json array of BigInt values; BigInt is serialized as a string
    let public_inputs = tokio::fs::read_to_string(args.public_inputs).await?;
    let public_inputs = serde_json::from_str::<serde_json::Value>(&public_inputs)?;
    let public_inputs = match public_inputs.as_array() {
        Some(arr) => arr
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect::<Vec<_>>(),
        None => anyhow::bail!("Public inputs must be a json array of BigInt values"),
    };
    if args.generate_proving_key {
        let circuit = builder.setup();
        let (pk, _vk) = Groth16::<Bn254, CircomReduction>::circuit_specific_setup(circuit, rng)?;
        // save the proving key to a file in the output directory
        tokio::fs::create_dir_all(&args.output_dir).await?;
        let proving_key_path = args.output_dir.join("proving_key.bin");
        let mut bytes = Vec::new();
        pk.serialize_compressed(&mut bytes)?;
        tokio::fs::write(proving_key_path, bytes).await?;
    };

    let circom = builder.build().unwrap();
    let full_assignment = circom.witness.clone().unwrap();
    let cs = ConstraintSystem::<Bn254Fr>::new_ref();
    circom.generate_constraints(cs.clone()).unwrap();
    assert!(cs.is_satisfied().unwrap());
    let matrices = cs.to_matrices().unwrap();

    let pp = PackedSharingParams::new(2);
    let num_inputs = matrices.num_instance_variables;
    let num_constraints = matrices.num_constraints;
    let qap_timer = start_timer!(|| "QAP");
    let qap = qap::<Bn254Fr, Radix2EvaluationDomain<_>>(&matrices, &full_assignment).unwrap();
    end_timer!(qap_timer);
    let qap_shares = qap.pss(&pp);
    let ax_shares = pack_from_witness::<Bn254>(&pp, full_assignment[num_inputs..].to_vec());
    let a_shares = pack_from_witness::<Bn254>(&pp, full_assignment[1..].to_vec());
    let encoding_timer = start_timer!(|| "Encoding Shares");
    // Prepare things to be written to disk
    let qap_shares = qap_shares
        .into_iter()
        .map(|s| PackedQAPShare {
            a: cfg_into_iter!(s.a)
                .map(f_to_bytes)
                .map(ByteBuf::from)
                .collect(),
            b: cfg_into_iter!(s.b)
                .map(f_to_bytes)
                .map(ByteBuf::from)
                .collect(),
            c: cfg_into_iter!(s.c)
                .map(f_to_bytes)
                .map(ByteBuf::from)
                .collect(),
        })
        .collect::<Vec<_>>();
    let ax_shares = cfg_into_iter!(ax_shares)
        .map(|s| {
            cfg_into_iter!(s)
                .map(f_to_bytes)
                .map(ByteBuf::from)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let a_shares = cfg_into_iter!(a_shares)
        .map(|s| {
            cfg_into_iter!(s)
                .map(f_to_bytes)
                .map(ByteBuf::from)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    end_timer!(encoding_timer);

    let circuit = CircuitProperties {
        circuit_id: args.circuit_id,
        pk_uri: tokio::fs::canonicalize(args.output_dir.join("proving_key.bin")).await?,
        wasm_uri: tokio::fs::canonicalize(args.wasm).await?,
        r1cs_uri: tokio::fs::canonicalize(args.r1cs).await?,
        num_inputs,
        num_constraints,
    };
    let job = JobProperties {
        job_id: args.job_id,
        circuit_id: args.circuit_id,
        public_inputs,
        pss_l: pp.l,
        a_shares,
        ax_shares,
        qap_shares,
    };

    // write the circuit and job properties to a file
    let job_path = args.output_dir.join(format!("job_{}.json", args.job_id));
    let circuit_path = args
        .output_dir
        .join(format!("circuit_{}.json", args.circuit_id));
    tokio::fs::write(job_path, serde_json::to_string(&job)?).await?;
    tokio::fs::write(circuit_path, serde_json::to_string(&circuit)?).await?;
    tracing::info!("Job and circuit properties written to disk");
    end_timer!(setup_timer);
    Ok(())
}

fn f_to_bytes<F: Field + CanonicalSerialize>(f: F) -> Vec<u8> {
    let mut bytes = Vec::new();
    f.serialize_compressed(&mut bytes).unwrap();
    bytes
}

pub fn setup_log() {
    let _ = SubscriberBuilder::default()
        .with_env_filter(EnvFilter::from_default_env())
        .finish()
        .try_init();
}
