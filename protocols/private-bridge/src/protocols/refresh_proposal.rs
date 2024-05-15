use parity_scale_codec::{Decode, Encode, MaxEncodedLen};

#[derive(Clone)]
pub struct RefreshProposal {
    /// The merkle root of the voters (validators)
    pub voter_merkle_root: [u8; 32],
    /// The session length in milliseconds
    pub session_length: u64,
    /// The number of voters
    pub voter_count: u32,
    /// The refresh nonce for the rotation
    pub nonce: u32,
    /// The public key of the governor
    pub pub_key: Vec<u8>,
}

impl RefreshProposal {
    /// Length of the proposal in bytes.
    pub const LENGTH: usize = 32 + 8 + 4 + 4 + 64;

    /// Build a `RefreshProposal` from raw bytes
    pub fn from(bytes: &[u8]) -> Result<Self, &'static str> {
        Decode::decode(&mut &bytes[..]).map_err(|_| "Failed to decode RefreshProposal")
    }
}

impl Default for RefreshProposal {
    fn default() -> Self {
        Self {
            voter_merkle_root: [0u8; 32],
            session_length: 0,
            voter_count: 0,
            nonce: 0,
            pub_key: vec![],
        }
    }
}

impl MaxEncodedLen for RefreshProposal {
    fn max_encoded_len() -> usize {
        Self::LENGTH
    }
}

impl Decode for RefreshProposal {
    fn decode<I: parity_scale_codec::Input>(
        input: &mut I,
    ) -> Result<Self, parity_scale_codec::Error> {
        let mut data = [0u8; Self::LENGTH];
        input.read(&mut data).map_err(|_| {
            parity_scale_codec::Error::from(
                "input bytes are less than the expected size (112 bytes)",
            )
        })?;
        let mut voter_merkle_root_bytes = [0u8; 32];
        let mut session_length_bytes = [0u8; 8];
        let mut voter_count_bytes = [0u8; 4];
        let mut nonce_bytes = [0u8; 4];
        let mut pub_key_bytes = [0u8; 64];
        voter_merkle_root_bytes.copy_from_slice(&data[0..32]);
        session_length_bytes.copy_from_slice(&data[32..40]);
        voter_count_bytes.copy_from_slice(&data[40..44]);
        nonce_bytes.copy_from_slice(&data[44..(44 + 4)]);
        pub_key_bytes.copy_from_slice(&data[(44 + 4)..]);
        let voter_merkle_root = voter_merkle_root_bytes;
        let session_length = u64::from_be_bytes(session_length_bytes);
        let voter_count = u32::from_be_bytes(voter_count_bytes);
        let nonce = u32::from_be_bytes(nonce_bytes);
        let pub_key = pub_key_bytes.to_vec();
        Ok(Self {
            voter_merkle_root,
            session_length,
            voter_count,
            nonce,
            pub_key,
        })
    }
}

impl Encode for RefreshProposal {
    fn encode(&self) -> Vec<u8> {
        let mut ret = vec![0u8; Self::LENGTH];
        let voter_merkle_root = self.voter_merkle_root;
        let session_length = self.session_length.to_be_bytes();
        let voter_count = self.voter_count.to_be_bytes();
        let nonce = self.nonce.to_be_bytes();
        let pub_key = self.pub_key.as_slice();
        ret[0..32].copy_from_slice(&voter_merkle_root);
        ret[32..40].copy_from_slice(&session_length);
        ret[40..44].copy_from_slice(&voter_count);
        ret[44..(44 + 4)].copy_from_slice(&nonce);
        ret[(44 + 4)..].copy_from_slice(pub_key);
        ret
    }

    fn encoded_size(&self) -> usize {
        Self::LENGTH
    }
}
