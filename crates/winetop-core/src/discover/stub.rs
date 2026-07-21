use super::raw::RawProcess;
use crate::{Error, Result};

pub fn scan() -> Result<Vec<RawProcess>> {
    Err(Error::UnsupportedPlatform)
}
