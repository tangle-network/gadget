#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Eigenlayer error: `{0}`")]
    Eigenlayer(String),
    #[error("EVM error: `{0}`")]
    Evm(String),
    #[error("Tangle error: `{0}`")]
    Tangle(String),
    #[error("Network error: `{0}`")]
    Network(String),
    #[error("Unable to fetch operators: `{0}`")]
    GetOperators(String),
    #[error("Unable to fetch operator id: `{0}`")]
    OperatorId(String),
    #[error("Unable to fetch unique id: `{0}`")]
    UniqueId(String),
    #[error("Unable to fetch operators and operator id: `{0}`")]
    GetOperatorsAndOperatorId(String),
    #[error("Unable to fetch operator index: `{0}`")]
    GetOperatorIndex(String),
    #[error("Client error: `{0}`")]
    Other(String),
}

impl Error {
    pub fn msg<T: gadget_std::fmt::Debug>(msg: T) -> Self {
        Error::Other(format!("{msg:?}"))
    }
}
