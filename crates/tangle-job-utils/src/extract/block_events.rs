use blueprint_job_router::__composite_rejection as composite_rejection;
use blueprint_job_router::__define_rejection as define_rejection;
use blueprint_job_router::{job_call::Parts as JobCallParts, FromJobCallParts};
use tangle_subxt::subxt::events::Events;
use tangle_subxt::subxt::events::StaticEvent;

use crate::producer::TangleConfig;

/// Extracts all the events that happened in the current block.
#[derive(Debug, Clone)]
pub struct BlockEvents(pub Events<TangleConfig>);

blueprint_job_router::__impl_deref!(BlockEvents: Events<TangleConfig>);
blueprint_job_router::__impl_from!(Events<TangleConfig>, BlockEvents);

define_rejection! {
  #[body = "No events found in the extensions. Did you forget to add the `AddBlockEventsLayer`?"]
  /// A Rejection type for [`BlockEvents`] when it is missing in the Extensions.
  /// This usually happens when the `AddBlockEventsLayer` is not added to the Router.
  pub struct MissingBlockEvents;
}

impl TryFrom<&mut JobCallParts> for BlockEvents {
    type Error = MissingBlockEvents;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let events = parts
            .extensions
            .get::<Events<TangleConfig>>()
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

/// Extracts the first event of type `T` that happened in the current block.
pub struct FirstEvent<T>(pub T);

/// Extracts the last event of type `T` that happened in the current block.
pub struct LastEvent<T>(pub T);

/// Extracts all the events of type `T` that happened in the current block.
pub struct Event<T>(pub Vec<T>);

blueprint_job_router::__impl_deref!(FirstEvent);
blueprint_job_router::__impl_deref!(LastEvent);

define_rejection! {
  #[body = "No event that matches this event found in the extensions. Did you forget to add the `AddBlockEventsLayer`?"]
  /// A Rejection type for [`Event`] when it is missing in the Extensions.
  /// This usually happens when the `AddBlockEventsLayer` is not added to the Router.
  pub struct MissingEvent;
}

define_rejection! {
  #[body = "An error occurred while trying to extract the event from subxt"]
  /// A Rejection type for [`Event`] when an error occurs while trying to extract the event from subxt.
  pub struct ClientError(Error);
}

composite_rejection! {
    /// Rejection used for [`Event`].
    ///
    /// Contains one variant for each way the [`BlockHash`] extractor
    /// can fail.
    pub enum MissingEventRejection {
        MissingBlockEvents,
        MissingEvent,
        ClientError,
    }
}

macro_rules! impl_try_from {
    ($t:ident, $method:ident) => {
        impl<T> TryFrom<&mut JobCallParts> for $t<T>
        where
            T: StaticEvent,
        {
            type Error = MissingEventRejection;

            fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
                let event = parts
                    .extensions
                    .get::<Events<TangleConfig>>()
                    .ok_or(MissingBlockEvents)?
                    .$method::<T>()
                    .map_err(ClientError::from_err)?
                    .ok_or(MissingEvent)?;
                Ok($t(event))
            }
        }
    };
}
impl_try_from!(FirstEvent, find_first);
impl_try_from!(LastEvent, find_last);

impl<T> TryFrom<&mut JobCallParts> for Event<T>
where
    T: StaticEvent,
{
    type Error = MissingEventRejection;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let events = parts
            .extensions
            .get::<Events<TangleConfig>>()
            .ok_or(MissingBlockEvents)?
            .find::<T>()
            .collect::<Result<Vec<T>, _>>()
            .map_err(ClientError::from_err)?;
        Ok(Event(events))
    }
}

macro_rules! impl_from_job_call_parts {
    ($t:ident) => {
        impl<Ctx, T> FromJobCallParts<Ctx> for $t<T>
        where
            Ctx: Send + Sync,
            T: StaticEvent,
        {
            type Rejection = MissingEventRejection;

            async fn from_job_call_parts(
                parts: &mut JobCallParts,
                _: &Ctx,
            ) -> Result<Self, Self::Rejection> {
                Self::try_from(parts)
            }
        }
    };
}

impl_from_job_call_parts!(FirstEvent);
impl_from_job_call_parts!(LastEvent);
impl_from_job_call_parts!(Event);
