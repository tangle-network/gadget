use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct TestError {
    pub reason: String,
}

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl Error for TestError {}
