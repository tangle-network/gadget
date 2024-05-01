use color_eyre::Report;

impl From<color_eyre::Report> for gadget_common::Error {
    fn from(value: Report) -> Self {
        gadget_common::Error::InitError {
            err: value.to_string(),
        }
    }
}
