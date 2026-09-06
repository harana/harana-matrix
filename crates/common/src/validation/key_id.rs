use std::num::NonZeroU8;

use crate::validation::{Error, KeyName};

pub fn validate<K: KeyName + ?Sized>(s: &str) -> Result<NonZeroU8, Error> {
    let colon_idx =
        NonZeroU8::new(s.find(':').ok_or(Error::MissingColon)? as u8).ok_or(Error::MissingColon)?;

    K::validate(&s[colon_idx.get() as usize + 1..])?;

    Ok(colon_idx)
}
