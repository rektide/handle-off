//! State meaning, encoding and carrier are separate concerns. No codec is required.
use handle_off_handles::ResourceId;
use std::{error::Error, fmt};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateFormat {
    pub name: String,
    pub version: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    Inline(Vec<u8>),
    /// Positional bytes; do not mutate the carrier's shared file offset.
    ResourceRange {
        resource: ResourceId,
        offset: u64,
        length: u64,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extension {
    pub key: String,
    pub format: StateFormat,
    /// An adopter must understand this extension before allowing resumed I/O.
    pub required: bool,
    pub payload: Payload,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateError {
    InvalidKey,
    InvalidFormat,
    RangeOverflow,
}
impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidKey => "invalid extension key",
            Self::InvalidFormat => "invalid state format name",
            Self::RangeOverflow => "state resource range overflows",
        })
    }
}
impl Error for StateError {}
impl Extension {
    /// Structural validation only. Applications validate compatibility and meaning;
    /// carrier adapters validate bounds against actual length and required seals.
    pub fn validate(&self) -> Result<(), StateError> {
        let valid = |s: &str| !s.is_empty() && !s.chars().any(char::is_control);
        if !valid(&self.key) {
            return Err(StateError::InvalidKey);
        }
        if !valid(&self.format.name) {
            return Err(StateError::InvalidFormat);
        }
        if let Payload::ResourceRange { offset, length, .. } = &self.payload {
            offset
                .checked_add(*length)
                .ok_or(StateError::RangeOverflow)?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges_are_checked_without_interpreting_opaque_bytes() {
        let mut extension = Extension {
            key: "decoder".into(),
            format: StateFormat {
                name: "app.decoder".into(),
                version: 99,
            },
            required: true,
            payload: Payload::Inline(vec![0xff]),
        };
        assert_eq!(extension.validate(), Ok(()));
        extension.payload = Payload::ResourceRange {
            resource: ResourceId::new("image").unwrap(),
            offset: u64::MAX,
            length: 1,
        };
        assert_eq!(extension.validate(), Err(StateError::RangeOverflow));
    }
}
