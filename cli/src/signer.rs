use std::str::FromStr;

use alloy_signer_local::PrivateKeySigner;
use color_eyre::{eyre::Context, Result, Section};
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use tangle_subxt::subxt::ext::sp_core;
use tangle_subxt::subxt::ext::sp_core::Pair;
use tangle_subxt::subxt_signer::bip39;
use tangle_subxt::subxt_signer::ExposeSecret;
use tangle_subxt::subxt_signer::SecretUri;

pub(crate) const SIGNER_ENV: &str = "SIGNER";
pub(crate) const EVM_SIGNER_ENV: &str = "EVM_SIGNER";

const SURI_HELP_MSG: &str = r#"
The `SURI` can be parsed from a string. The string takes this form:
```text
phrase/path0/path1///password
111111 22222 22222   33333333
```
///
Where:
- 1 denotes a phrase or hex string. If this is not provided, the [`DEV_PHRASE`] is used
  instead.
- 2's denote optional "derivation junctions" which are used to derive keys. Each of these is
  separated by "/". A derivation junction beginning with "/" (ie "//" in the original string)
  is a "hard" path.
- 3 denotes an optional password which is used in conjunction with the phrase provided in 1
  to generate an initial key. If hex is provided for 1, it's ignored.

# Notes:
- If 1 is a `0x` prefixed 64-digit hex string, then we'll interpret it as hex, and treat the hex bytes
  as a seed/MiniSecretKey directly, ignoring any password.
- Else if the phrase part is a valid BIP-39 phrase, we'll use the phrase (and password, if provided)
  to generate a seed/MiniSecretKey.
- Uris like "//Alice" correspond to keys derived from a DEV_PHRASE, since no phrase part is given.

There is no correspondence mapping between `SURI` strings and the keys they represent.
Two different non-identical strings can actually lead to the same secret being derived.
Notably, integer junction indices may be legally prefixed with arbitrary number of zeros.
Similarly an empty password (ending the `SURI` with `///`) is perfectly valid and will
generally be equivalent to no password at all.
"#;

/// Loads the Substrate Signer from the environment.
pub fn load_signer_from_env() -> Result<TanglePairSigner<sp_core::sr25519::Pair>> {
    let s = std::env::var(SIGNER_ENV)
        .with_suggestion(|| {
            format!(
                "Please set the signer SURI in the environment using the `{SIGNER_ENV}` variable.",
            )
        })
        .note(SURI_HELP_MSG)?;

    let sp_core_keypair = sp_core::sr25519::Pair::from_string(&s, None)?;
    Ok(TanglePairSigner::new(sp_core_keypair))
}

/// Loads the EVM Signer from the environment.
pub fn load_evm_signer_from_env() -> Result<PrivateKeySigner> {
    let secret = std::env::var(EVM_SIGNER_ENV).with_suggestion(|| {
        format!(
            "Please set the EVM signer SURI in the environment using the `{EVM_SIGNER_ENV}` variable.",
        )
    })
    .note(SURI_HELP_MSG)?;

    let uri = SecretUri::from_str(&secret)
        .with_context(|| "Parsing the SURI into a Secret Key")
        .note(SURI_HELP_MSG)?;
    let key = if let Some(hex_str) = uri.phrase.expose_secret().strip_prefix("0x") {
        PrivateKeySigner::from_str(hex_str)
            .context("Parsing the hex string into a PrivateKeySigner")?
    } else {
        let phrase = bip39::Mnemonic::from_str(uri.phrase.expose_secret().as_str())?;
        let secret_bytes = phrase.to_entropy();
        PrivateKeySigner::from_slice(secret_bytes.as_slice())
            .context("Creating a PrivateKeySigner from the mnemonic phrase")?
    };

    Ok(key)
}
