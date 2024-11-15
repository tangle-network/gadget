#[macro_export]
macro_rules! compute_sha256_hash {
    ($($data:expr),*) => {
        {
            use k256::sha2::{Digest, Sha256};
            let mut hasher = Sha256::default();
            $(hasher.update($data);)*
            let result = hasher.finalize();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(result.as_slice());
            hash
        }
    };
}
