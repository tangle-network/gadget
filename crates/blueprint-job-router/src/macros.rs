/// Private API.
#[cfg(feature = "tracing")]
#[doc(hidden)]
#[macro_export]
macro_rules! __log_rejection {
    (
        rejection_type = $ty:ident,
        body_text = $body_text:expr
    ) => {
        {
            $crate::__private::tracing::event!(
                target: "gadget::rejection",
                $crate::__private::tracing::Level::TRACE,
                body = $body_text,
                rejection_type = ::core::any::type_name::<$ty>(),
                "rejecting job call",
            );
        }
    };
}

#[cfg(not(feature = "tracing"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __log_rejection {
    (
        rejection_type = $ty:ident,
        body_text = $body_text:expr
    ) => {};
}

#[rustfmt::skip]
macro_rules! all_the_tuples {
    ($name:ident) => {
        $name!([], T1);
        $name!([T1], T2);
        $name!([T1, T2], T3);
        $name!([T1, T2, T3], T4);
        $name!([T1, T2, T3, T4], T5);
        $name!([T1, T2, T3, T4, T5], T6);
        $name!([T1, T2, T3, T4, T5, T6], T7);
        $name!([T1, T2, T3, T4, T5, T6, T7], T8);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8], T9);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9], T10);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10], T11);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11], T12);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12], T13);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13], T14);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14], T15);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15], T16);
    };
}

macro_rules! all_the_tuples_no_last_special_case {
    ($name:ident) => {
        $name!(T1);
        $name!(T1, T2);
        $name!(T1, T2, T3);
        $name!(T1, T2, T3, T4);
        $name!(T1, T2, T3, T4, T5);
        $name!(T1, T2, T3, T4, T5, T6);
        $name!(T1, T2, T3, T4, T5, T6, T7);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);
    };
}

/// Private API.
#[doc(hidden)]
#[macro_export]
macro_rules! __impl_deref {
    ($ident:ident) => {
        impl<T> core::ops::Deref for $ident<T> {
            type Target = T;

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<T> core::ops::DerefMut for $ident<T> {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };

    ($ident:ident: $ty:ty) => {
        impl core::ops::Deref for $ident {
            type Target = $ty;

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl core::ops::DerefMut for $ident {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
}

/// Private API.
#[doc(hidden)]
#[macro_export]
macro_rules! __impl_from {
    ($from:ty, $to:ty) => {
        impl From<$from> for $to {
            fn from(value: $from) -> Self {
                Self(value)
            }
        }
    };

    ($from:ty, $to:ty, $method:ident) => {
        impl From<$from> for $to {
            fn from(value: $from) -> Self {
                Self::$method(value)
            }
        }
    };
}

#[doc(hidden)]
macro_rules! opaque_future {
    ($(#[$m:meta])* pub type $name:ident = $actual:ty;) => {
        opaque_future! {
            $(#[$m])*
            pub type $name<> = $actual;
        }
    };

    ($(#[$m:meta])* pub type $name:ident<$($param:ident),*> = $actual:ty;) => {
        pin_project_lite::pin_project! {
            $(#[$m])*
            pub struct $name<$($param),*> {
                #[pin] future: $actual,
            }
        }

        impl<$($param),*> $name<$($param),*> {
            pub(crate) fn new(future: $actual) -> Self {
                Self { future }
            }
        }

        impl<$($param),*> core::fmt::Debug for $name<$($param),*> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_struct(stringify!($name)).finish_non_exhaustive()
            }
        }

        impl<$($param),*> core::future::Future for $name<$($param),*>
        where
            $actual: core::future::Future,
        {
            type Output = <$actual as core::future::Future>::Output;

            #[inline]
            fn poll(
                self: core::pin::Pin<&mut Self>,
                cx: &mut core::task::Context<'_>,
            ) -> core::task::Poll<Self::Output> {
                self.project().future.poll(cx)
            }
        }
    };
}

/// Private API.
#[doc(hidden)]
#[macro_export]
macro_rules! __define_rejection {
    (
        #[body = $body:literal]
        $(#[$m:meta])*
        pub struct $name:ident;
    ) => {
        $(#[$m])*
        #[derive(Debug)]
        #[non_exhaustive]
        pub struct $name;

        impl $name {
            /// Get the response body text used for this rejection.
            pub fn body_text(&self) -> String {
                self.to_string()
            }
        }

        impl $crate::job_result::IntoJobResult for $name {
            fn into_job_result(self) -> $crate::JobResult {
                $crate::__log_rejection!(
                    rejection_type = $name,
                    body_text = $body
                );
                $body.into_job_result()
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}", $body)
            }
        }

        impl core::error::Error for $name {}

        impl Default for $name {
            fn default() -> Self {
                Self
            }
        }
    };

    (
        #[body = $body:literal]
        $(#[$m:meta])*
        pub struct $name:ident (Error);
    ) => {
        $(#[$m])*
        #[derive(Debug)]
        pub struct $name(pub(crate) $crate::Error);

        impl $name {
            pub(crate) fn from_err<E>(err: E) -> Self
            where
                E: Into<$crate::BoxError>,
            {
                Self($crate::Error::new(err))
            }

            /// Get the response body text used for this rejection.
            pub fn body_text(&self) -> ::alloc::string::String {
                use ::alloc::string::ToString;
                self.to_string()
            }
        }

        impl $crate::job_result::IntoJobResult for $name {
            fn into_job_result(self) -> $crate::JobResult {
                let body_text = self.body_text();

                $crate::__log_rejection!(
                    rejection_type = $name,
                    body_text = body_text
                );
                body_text.into_job_result()
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str($body)?;
                f.write_str(": ")?;
                self.0.fmt(f)
            }
        }

        impl core::error::Error for $name {
            fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
                Some(&self.0)
            }
        }
    };
}

/// Private API.
#[doc(hidden)]
#[macro_export]
macro_rules! __composite_rejection {
    (
        $(#[$m:meta])*
        pub enum $name:ident {
            $($variant:ident),+
            $(,)?
        }
    ) => {
        $(#[$m])*
        #[derive(Debug)]
        #[non_exhaustive]
        pub enum $name {
            $(
                #[allow(missing_docs)]
                $variant($variant)
            ),+
        }

        impl $crate::job_result::IntoJobResult for $name {
            fn into_job_result(self) -> $crate::JobResult {
                match self {
                    $(
                        Self::$variant(inner) => inner.into_job_result(),
                    )+
                }
            }
        }

        impl $name {
            /// Get the response body text used for this rejection.
            pub fn body_text(&self) -> String {
                match self {
                    $(
                        Self::$variant(inner) => inner.body_text(),
                    )+
                }
            }
        }

        $(
            impl From<$variant> for $name {
                fn from(inner: $variant) -> Self {
                    Self::$variant(inner)
                }
            }
        )+

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self {
                    $(
                        Self::$variant(inner) => write!(f, "{inner}"),
                    )+
                }
            }
        }

        impl core::error::Error for $name {
            fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
                match self {
                    $(
                        Self::$variant(inner) => inner.source(),
                    )+
                }
            }
        }
    };
}
