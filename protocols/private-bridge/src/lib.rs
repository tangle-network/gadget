use crate::protocols::key_refresh::PrivateBridgeKeyRefreshExtraParams;
use crate::protocols::key_rotate::PrivateBridgeKeyRotateExtraParams;
use crate::protocols::keygen::PrivateBridgeKeygenExtraParams;
use crate::protocols::sign::PrivateBridgeSigningExtraParams;
use async_trait::async_trait;
use gadget_common::full_protocol::SharedOptional;
use gadget_common::prelude::*;
use gadget_common::tangle_runtime::*;
use gadget_common::{generate_protocol, generate_setup_and_run_command};
use protocol_macros::protocol;

pub mod constants;
pub mod error;
pub mod protocols;

generate_protocol!(
    "Private-Bridge-Keygen-Protocol",
    PrivateBridgeKeygenProtocol,
    PrivateBridgeKeygenExtraParams,
    protocols::keygen::generate_protocol_from,
    protocols::keygen::create_next_job,
    jobs::JobType::DKGTSSPhaseOne(_),
    roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::DfnsCGGMP21Secp256k1)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::DfnsCGGMP21Secp256r1)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::DfnsCGGMP21Stark)
);
generate_protocol!(
    "Private-Bridge-Signing-Protocol",
    PrivateBridgeSigningProtocol,
    PrivateBridgeSigningExtraParams,
    protocols::sign::generate_protocol_from,
    protocols::sign::create_next_job,
    jobs::JobType::DKGTSSPhaseTwo(_),
    roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::DfnsCGGMP21Secp256k1)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::DfnsCGGMP21Secp256r1)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::DfnsCGGMP21Stark)
);
generate_protocol!(
    "Private-Bridge-Refresh-Protocol",
    PrivateBirdgeKeyRefreshProtocol,
    PrivateBridgeKeyRefreshExtraParams,
    protocols::key_refresh::generate_protocol_from,
    protocols::key_refresh::create_next_job,
    jobs::JobType::DKGTSSPhaseThree(_),
    roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::DfnsCGGMP21Secp256k1)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::DfnsCGGMP21Secp256r1)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::DfnsCGGMP21Stark)
);
generate_protocol!(
    "Private-Bridge-Rotate-Protocol",
    PrivateBirdgeKeyRotateProtocol,
    PrivateBridgeKeyRotateExtraParams,
    protocols::key_rotate::generate_protocol_from,
    protocols::key_rotate::create_next_job,
    jobs::JobType::DKGTSSPhaseFour(_),
    roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::DfnsCGGMP21Secp256k1)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::DfnsCGGMP21Secp256r1)
        | roles::RoleType::Tss(roles::tss::ThresholdSignatureRoleType::DfnsCGGMP21Stark)
);

generate_setup_and_run_command!(
    PrivateBridgeKeygenProtocol,
    PrivateBridgeSigningProtocol,
    PrivateBirdgeKeyRefreshProtocol,
    PrivateBirdgeKeyRotateProtocol
);

#[cfg(test)]
mod secp256k1 {
    test_utils::generate_signing_and_keygen_tss_tests!(
        2,
        3,
        4,
        ThresholdSignatureRoleType::DfnsCGGMP21Secp256k1
    );
}

#[cfg(test)]
mod secp256r1 {
    test_utils::generate_signing_and_keygen_tss_tests!(
        2,
        3,
        4,
        ThresholdSignatureRoleType::DfnsCGGMP21Secp256r1
    );
}

#[cfg(test)]
mod stark {
    test_utils::generate_signing_and_keygen_tss_tests!(
        2,
        3,
        4,
        ThresholdSignatureRoleType::DfnsCGGMP21Stark
    );
}
