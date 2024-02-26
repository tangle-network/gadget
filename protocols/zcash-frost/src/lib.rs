use crate::protocol::keygen::ZcashFrostKeygenExtraParams;
use crate::protocol::sign::ZcashFrostSigningExtraParams;
use async_trait::async_trait;
use gadget_common::full_protocol::SharedOptional;
use gadget_common::prelude::*;
use gadget_common::{generate_protocol, generate_setup_and_run_command};
use protocol_macros::protocol;

pub mod constants;
pub mod protocol;
pub mod rounds;

generate_protocol!(
    "Zcash-FROST-Keygen-Protocol",
    ZcashFrostKeygenProtocol,
    ZcashFrostKeygenExtraParams,
    crate::protocol::keygen::generate_protocol_from,
    crate::protocol::keygen::create_next_job,
    GadgetJobType::DKGTSSPhaseOne(_),
    RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostEd25519)
        | RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostEd448)
        | RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostP256)
        | RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostP384)
        | RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostSecp256k1)
        | RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostRistretto255)
);
generate_protocol!(
    "Zcash-FROST-Signing-Protocol",
    ZcashFrostSigningProtocol,
    ZcashFrostSigningExtraParams,
    crate::protocol::sign::generate_protocol_from,
    crate::protocol::sign::create_next_job,
    GadgetJobType::DKGTSSPhaseTwo(_),
    RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostEd25519)
        | RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostEd448)
        | RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostP256)
        | RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostP384)
        | RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostSecp256k1)
        | RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostRistretto255)
);

generate_setup_and_run_command!(ZcashFrostKeygenProtocol, ZcashFrostSigningProtocol);

mod secp256k1 {
    test_utils::generate_signing_and_keygen_tss_tests!(
        2,
        3,
        2,
        ThresholdSignatureRoleType::ZcashFrostSecp256k1
    );
}

mod ristretto255 {
    test_utils::generate_signing_and_keygen_tss_tests!(
        2,
        3,
        2,
        ThresholdSignatureRoleType::ZcashFrostRistretto255
    );
}

mod p256 {
    test_utils::generate_signing_and_keygen_tss_tests!(
        2,
        3,
        2,
        ThresholdSignatureRoleType::ZcashFrostP256
    );
}

mod p384 {
    test_utils::generate_signing_and_keygen_tss_tests!(
        2,
        3,
        2,
        ThresholdSignatureRoleType::ZcashFrostP384
    );
}

mod ed25519 {
    test_utils::generate_signing_and_keygen_tss_tests!(
        2,
        3,
        2,
        ThresholdSignatureRoleType::ZcashFrostEd25519
    );
}

mod ed448 {
    test_utils::generate_signing_and_keygen_tss_tests!(
        2,
        3,
        2,
        ThresholdSignatureRoleType::ZcashFrostEd448
    );
}
