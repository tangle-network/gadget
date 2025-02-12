use crate::JobCall;
use std::{
    sync::Arc,
    task::{Context, Poll},
};
use tower::layer::layer_fn;
use tower::{Layer, Service};

#[derive(Clone)]
pub(super) struct StripPrefix<S> {
    inner: S,
    prefix: Arc<str>,
}

impl<S> StripPrefix<S> {
    pub(super) fn layer(prefix: &str) -> impl Layer<S, Service = Self> + Clone {
        let prefix = Arc::from(prefix);
        layer_fn(move |inner| Self {
            inner,
            prefix: Arc::clone(&prefix),
        })
    }
}

impl<S, B> Service<JobCall<B>> for StripPrefix<S>
where
    S: Service<JobCall<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, call: JobCall<B>) -> Self::Future {
        self.inner.call(call)
    }
}

fn segments(s: &str) -> impl Iterator<Item = &str> {
    assert!(
        s.starts_with('/'),
        "path didn't start with '/'. axum should have caught this higher up."
    );

    s.split('/')
        // skip one because paths always start with `/` so `/a/b` would become ["", "a", "b"]
        // otherwise
        .skip(1)
}

fn zip_longest<I, I2>(a: I, b: I2) -> impl Iterator<Item = Item<I::Item>>
where
    I: Iterator,
    I2: Iterator<Item = I::Item>,
{
    let a = a.map(Some).chain(std::iter::repeat_with(|| None));
    let b = b.map(Some).chain(std::iter::repeat_with(|| None));
    a.zip(b).map_while(|(a, b)| match (a, b) {
        (Some(a), Some(b)) => Some(Item::Both(a, b)),
        (Some(a), None) => Some(Item::First(a)),
        (None, Some(b)) => Some(Item::Second(b)),
        (None, None) => None,
    })
}

fn is_capture(segment: &str) -> bool {
    segment.starts_with('{')
        && segment.ends_with('}')
        && !segment.starts_with("{{")
        && !segment.ends_with("}}")
        && !segment.starts_with("{*")
}

#[derive(Debug)]
enum Item<T> {
    Both(T, T),
    First(T),
    Second(T),
}
