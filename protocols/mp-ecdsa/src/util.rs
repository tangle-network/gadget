use curv::arithmetic::Converter;
use multi_party_ecdsa::gg_2020::party_i::SignatureRecid;
use sp_core::ecdsa::Signature;

pub fn convert_signature(sig_recid: &SignatureRecid) -> Signature {
    let r = sig_recid.r.to_bigint().to_bytes();
    let s = sig_recid.s.to_bigint().to_bytes();
    let v = sig_recid.recid + 27u8;
    let mut dkg_sig_arr: [u8; 65] = [0; 65];

    dkg_sig_arr[0..32].copy_from_slice(&r);
    dkg_sig_arr[32..64].copy_from_slice(&s);
    dkg_sig_arr[64] = v;

    Signature(dkg_sig_arr)
}
