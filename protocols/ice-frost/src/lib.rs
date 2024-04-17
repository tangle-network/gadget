use crate::protocol::keygen::IceFrostKeygenExtraParams;
use crate::protocol::sign::IceFrostSigningExtraParams;
use async_trait::async_trait;
use gadget_common::full_protocol::SharedOptional;
use gadget_common::prelude::*;
use gadget_common::tangle_runtime::*;
use gadget_common::{generate_protocol, generate_setup_and_run_command};
use protocol_macros::protocol;

pub mod constants;
pub mod curves;
pub mod protocol;
pub mod rounds;

generate_protocol!(
    "Ice-FROST-Keygen-Protocol",
    IceFrostKeygenProtocol,
    IceFrostKeygenExtraParams,
    crate::protocol::keygen::generate_protocol_from,
    crate::protocol::keygen::create_next_job,
    jobs::JobType::DKGTSSPhaseOne(_),
    roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::ZcashFrostEd25519)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::ZcashFrostEd448)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::ZcashFrostP256)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::ZcashFrostP384)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::ZcashFrostSecp256k1)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::ZcashFrostRistretto255)
);
generate_protocol!(
    "Ice-FROST-Signing-Protocol",
    IceFrostSigningProtocol,
    IceFrostSigningExtraParams,
    crate::protocol::sign::generate_protocol_from,
    crate::protocol::sign::create_next_job,
    jobs::JobType::DKGTSSPhaseTwo(_),
    roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::ZcashFrostEd25519)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::ZcashFrostEd448)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::ZcashFrostP256)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::ZcashFrostP384)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::ZcashFrostSecp256k1)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::ZcashFrostRistretto255)
);

generate_setup_and_run_command!(IceFrostKeygenProtocol, IceFrostSigningProtocol);
