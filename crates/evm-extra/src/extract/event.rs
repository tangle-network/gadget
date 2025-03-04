//! Event extractors for EVM
//!
//! Simple extractors for EVM events and logs, following the same pattern as
//! the Substrate implementation.

use alloy_rpc_types::Log;
use alloy_sol_types::SolEvent;
use blueprint_core::{
    __composite_rejection as composite_rejection, __define_rejection as define_rejection,
    __impl_deref as impl_deref, __impl_deref_vec as impl_deref_vec, __impl_from as impl_from,
    FromJobCallParts, job_call::Parts as JobCallParts,
};

/// Extracts all events from the current block
#[derive(Debug, Clone)]
pub struct BlockEvents(pub Vec<Log>);

impl_deref!(BlockEvents: Vec<Log>);
impl_from!(Vec<Log>, BlockEvents);

define_rejection! {
    #[body = "No events found in the extensions"]
    /// This rejection is used to indicate that no events were found in the extensions.
    /// This should never happen, but it's here to be safe.
    pub struct MissingBlockEvents;
}

impl TryFrom<&mut JobCallParts> for BlockEvents {
    type Error = MissingBlockEvents;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let events = parts
            .extensions
            .get::<Vec<Log>>()
            .ok_or(MissingBlockEvents)?;
        Ok(BlockEvents(events.clone()))
    }
}

impl<Ctx> FromJobCallParts<Ctx> for BlockEvents
where
    Ctx: Send + Sync,
{
    type Rejection = MissingBlockEvents;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        Self::try_from(parts)
    }
}

/// Extracts events of type T from the current block
#[derive(Debug)]
pub struct Events<T>(pub Vec<T>);

impl_deref_vec!(Events);

define_rejection! {
    #[body = "Failed to decode events"]
    /// This rejection is used to indicate that the events could not be decoded.
    pub struct EventDecodingError;
}

composite_rejection! {
    /// Rejection for event extractor
    pub enum EventRejection {
        MissingBlockEvents,
        EventDecodingError,
    }
}

impl<T> TryFrom<&mut JobCallParts> for Events<T>
where
    T: SolEvent + Clone,
{
    type Error = EventRejection;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let logs = parts
            .extensions
            .get::<Vec<Log>>()
            .ok_or(MissingBlockEvents)?;

        let events = logs
            .iter()
            .filter(|log| T::SIGNATURE_HASH == log.topics()[0])
            .filter_map(|log| T::decode_log(&log.inner, true).ok())
            .map(|event| event.data)
            .collect();

        Ok(Events(events))
    }
}

impl<Ctx, T> FromJobCallParts<Ctx> for Events<T>
where
    Ctx: Send + Sync,
    T: SolEvent + Clone + Send + Sync,
{
    type Rejection = EventRejection;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        Self::try_from(parts)
    }
}

/// Extracts the first event of type T from the current block
#[derive(Debug, Clone)]
pub struct FirstEvent<T>(pub T);

impl_deref!(FirstEvent);

impl<T: SolEvent + Clone> TryFrom<&mut JobCallParts> for FirstEvent<T> {
    type Error = EventRejection;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let logs = parts
            .extensions
            .get::<Vec<Log>>()
            .ok_or(MissingBlockEvents)?;

        let first_t = logs
            .iter()
            .find_map(|log| {
                if Some(&T::SIGNATURE_HASH) == log.topic0() {
                    T::decode_log(&log.inner, true).ok()
                } else {
                    None
                }
            })
            .ok_or(EventDecodingError)?;

        Ok(FirstEvent(first_t.data))
    }
}

impl<Ctx, T> FromJobCallParts<Ctx> for FirstEvent<T>
where
    Ctx: Send + Sync,
    T: SolEvent + Clone + Send + Sync,
{
    type Rejection = EventRejection;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        Self::try_from(parts)
    }
}

/// Extracts the last event of type T from the current block
#[derive(Debug, Clone)]
pub struct LastEvent<T>(pub T);

impl_deref!(LastEvent);

impl<T: SolEvent + Clone> TryFrom<&mut JobCallParts> for LastEvent<T> {
    type Error = EventRejection;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let logs = parts
            .extensions
            .get::<Vec<Log>>()
            .ok_or(MissingBlockEvents)?;

        let last_t = logs
            .iter()
            .rev()
            .find_map(|log| {
                if Some(&T::SIGNATURE_HASH) == log.topic0() {
                    T::decode_log(&log.inner, true).ok()
                } else {
                    None
                }
            })
            .ok_or(EventDecodingError)?;

        Ok(LastEvent(last_t.data))
    }
}

impl<Ctx, T> FromJobCallParts<Ctx> for LastEvent<T>
where
    Ctx: Send + Sync,
    T: SolEvent + Clone + Send + Sync,
{
    type Rejection = EventRejection;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        Self::try_from(parts)
    }
}
