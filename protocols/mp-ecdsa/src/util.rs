use crate::constants::worker::ENGINE_ID;
use dkg_runtime_primitives::crypto::AuthorityId;
use dkg_runtime_primitives::{AuthoritySet, ConsensusLog, MaxAuthorities};
use sp_runtime::generic::OpaqueDigestItemId;
use sp_runtime::traits::{Block, Header};
use std::fmt::Display;

#[derive(Clone)]
pub struct DebugLogger;

impl DebugLogger {
    pub fn trace<T: Display>(&self, msg: T) {
        log::trace!(target: "gadget", "{msg}");
    }

    pub fn debug<T: Display>(&self, msg: T) {
        log::debug!(target: "gadget", "{msg}");
    }

    pub fn info<T: Display>(&self, msg: T) {
        log::info!(target: "gadget", "{msg}");
    }

    pub fn warn<T: Display>(&self, msg: T) {
        log::warn!(target: "gadget", "{msg}");
    }

    pub fn error<T: Display>(&self, msg: T) {
        log::error!(target: "gadget", "{msg}");
    }
}

/// Scan the `header` digest log for a DKG validator set change. Return either the new
/// validator set or `None` in case no validator set change has been signaled.
pub fn find_authorities_change<B>(
    header: &B::Header,
) -> Option<(
    AuthoritySet<AuthorityId, MaxAuthorities>,
    AuthoritySet<AuthorityId, MaxAuthorities>,
)>
where
    B: Block,
{
    let id = OpaqueDigestItemId::Consensus(&ENGINE_ID);

    header
        .digest()
        .convert_first(|l| l.try_to(id).and_then(match_consensus_log))
}

/// Matches a `ConsensusLog` for a DKG validator set change.
fn match_consensus_log(
    log: ConsensusLog<AuthorityId, MaxAuthorities>,
) -> Option<(
    AuthoritySet<AuthorityId, MaxAuthorities>,
    AuthoritySet<AuthorityId, MaxAuthorities>,
)> {
    match log {
        ConsensusLog::AuthoritiesChange {
            active: authority_set,
            queued: queued_authority_set,
        } => Some((authority_set, queued_authority_set)),
        _ => None,
    }
}
