pub mod client;
pub mod data;

pub type Error = Box<dyn std::error::Error>;
pub type Result<T = ()> = core::result::Result<T, Error>;
