//! Rejection response types.

use crate::__define_rejection as define_rejection;

define_rejection! {
    #[body = "Request body didn't contain valid UTF-8"]
    /// Rejection type used when buffering the request into a [`String`] if the
    /// body doesn't contain valid UTF-8.
    pub struct InvalidUtf8(Error);
}
