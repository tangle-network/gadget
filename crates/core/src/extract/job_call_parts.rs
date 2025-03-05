use super::{FromJobCall, FromJobCallParts, rejection::*};
use crate::JobCall;
use crate::job_call::Parts;
use crate::metadata::{MetadataMap, MetadataValue};
use alloc::string::String;
use bytes::{Bytes, BytesMut};
use core::convert::Infallible;

impl<Ctx> FromJobCall<Ctx> for JobCall
where
    Ctx: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_job_call(call: JobCall, _: &Ctx) -> Result<Self, Self::Rejection> {
        Ok(call)
    }
}

impl<Ctx> FromJobCallParts<Ctx> for MetadataMap<MetadataValue>
where
    Ctx: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_job_call_parts(parts: &mut Parts, _: &Ctx) -> Result<Self, Self::Rejection> {
        let metadata = parts.metadata.clone();

        Ok(metadata)
    }
}

impl<Ctx> FromJobCall<Ctx> for BytesMut
where
    Ctx: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_job_call(call: JobCall, _: &Ctx) -> Result<Self, Self::Rejection> {
        let (_, body) = call.into_parts();
        Ok(body.into())
    }
}

impl<Ctx> FromJobCall<Ctx> for Bytes
where
    Ctx: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_job_call(call: JobCall, _: &Ctx) -> Result<Self, Self::Rejection> {
        let (_, body) = call.into_parts();

        Ok(body)
    }
}

impl<Ctx> FromJobCall<Ctx> for String
where
    Ctx: Send + Sync,
{
    type Rejection = InvalidUtf8;

    async fn from_job_call(call: JobCall, ctx: &Ctx) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_job_call(call, ctx)
            .await
            .map_err(InvalidUtf8::from_err)?;

        let string = String::from_utf8(bytes.into()).map_err(InvalidUtf8::from_err)?;

        Ok(string)
    }
}

impl<Ctx> FromJobCallParts<Ctx> for Parts
where
    Ctx: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_job_call_parts(
        parts: &mut Parts,
        _context: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        Ok(parts.clone())
    }
}
