use curv::arithmetic::Converter;
use multi_party_ecdsa::gg_2020::party_i::SignatureRecid;
use sp_core::ecdsa::Signature;
use std::fmt::Display;

#[derive(Clone)]
pub struct DebugLogger {
    pub peer_id: String,
}

impl DebugLogger {
    pub fn trace<T: Display>(&self, msg: T) {
        log::trace!(target: "gadget", "[{}] {msg}", &self.peer_id);
    }

    pub fn debug<T: Display>(&self, msg: T) {
        log::debug!(target: "gadget", "[{}] {msg}", &self.peer_id);
    }

    pub fn info<T: Display>(&self, msg: T) {
        log::info!(target: "gadget", "[{}] {msg}", &self.peer_id);
    }

    pub fn warn<T: Display>(&self, msg: T) {
        log::warn!(target: "gadget", "[{}] {msg}", &self.peer_id);
    }

    pub fn error<T: Display>(&self, msg: T) {
        log::error!(target: "gadget", "[{}] {msg}", &self.peer_id);
    }
}

pub fn convert_signature(sig_recid: &SignatureRecid) -> Option<Signature> {
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
        log::warn!(target: "dkg_gadget", "🕸️  Invalid signature len: {}, expected 65", sig_vec.len());
        return None;
    }

    let mut dkg_sig_arr: [u8; 65] = [0; 65];
    dkg_sig_arr.copy_from_slice(&sig_vec[0..65]);

    Some(Signature(dkg_sig_arr))
}
