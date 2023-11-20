#![allow(clippy::complexity, clippy::style, clippy::pedantic)]

use clap::Parser;
use rcgen::{date_time_ymd, Certificate, CertificateParams, DistinguishedName, DnType, SanType};
use std::fs;

/// ZK Node
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The Id of the node, also acts as the party index
    #[arg(long)]
    i: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut params: CertificateParams = Default::default();
    params.not_before = date_time_ymd(1975, 01, 01);
    params.not_after = date_time_ymd(4096, 01, 01);
    params.distinguished_name = DistinguishedName::new();
    params
        .distinguished_name
        .push(DnType::OrganizationName, "Webb Inc.");
    params
        .distinguished_name
        .push(DnType::CommonName, "Master Cert");
    params.subject_alt_names = vec![
        SanType::IpAddress("127.0.0.1".parse()?),
        SanType::DnsName("localhost".to_string()),
    ];

    let cert = Certificate::from_params(params)?;

    std::fs::create_dir_all(format!("certs/{}", args.i))?;
    fs::write(format!("certs/{}/cert.der", args.i), &cert.serialize_der()?)?;
    fs::write(
        format!("certs/{}/key.der", args.i),
        &cert.serialize_private_key_der(),
    )?;
    Ok(())
}
