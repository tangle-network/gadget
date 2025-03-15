use gadget_std::io;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    CouldNotExtractPort(String),
    CouldNotExtractP2pAddress(String),
    CouldNotExtractP2pPort(String),
}
