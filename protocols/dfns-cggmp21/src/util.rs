use curv::arithmetic::Converter;
use gadget_common::debug_logger::DebugLogger;
use sp_core::ecdsa::Signature;

pub fn convert_signature(
    debug_logger: &DebugLogger,
    sig_recid: &SignatureRecid,
) -> Option<Signature> {
    let r = sig_recid.r.to_bigint().to_bytes();
    let s = sig_recid.s.to_bigint().to_bytes();
    let v = sig_recid.recid + 27u8;

    let mut sig_vec: Vec<u8> = Vec::new();

    for _ in 0..(32 - r.len()) {
        sig_vec.extend([0]);
    }
    sig_vec.extend_from_slice(&r);

    for _ in 0..(32 - s.len()) {
        sig_vec.extend([0]);
    }
    sig_vec.extend_from_slice(&s);

    sig_vec.extend([v]);

    if 65 != sig_vec.len() {
        debug_logger.warn(format!(
            "Invalid signature len: {}, expected 65",
            sig_vec.len()
        ));
        return None;
    }

    let mut dkg_sig_arr: [u8; 65] = [0; 65];
    dkg_sig_arr.copy_from_slice(&sig_vec[0..65]);

    Some(Signature(dkg_sig_arr))
}
