mod error;
pub use error::{Error, ErrorKind};
pub type Result<T> = std::result::Result<T, Error>;

mod path_info;
pub use path_info::PathInfo;
