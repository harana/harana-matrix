use crate::validation::Error;

pub fn validate(s: &str) -> Result<(), Error> {
    match s.as_bytes().first() {
        Some(b'#') => crate::validation::room_alias_id::validate(s),
        Some(b'!') => crate::validation::room_id::validate(s),
        _ => Err(Error::MissingLeadingSigil),
    }
}
