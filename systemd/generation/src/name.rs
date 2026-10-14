use std::{error::Error, fmt};
pub const MAX_NAME_BYTES: usize = 79;
pub const RESERVED_PREFIX: &str = "ho1.";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameError {
    TokenWidth,
    NonCanonicalHex,
    NameWidth,
    Delimiter,
    Member,
}
impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "generation name error: {self:?}")
    }
}
impl Error for NameError {}
fn hex(byte: u8) -> Result<u8, NameError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(NameError::NonCanonicalHex),
    }
}
fn token(bytes: &[u8]) -> Result<[u8; 16], NameError> {
    if bytes.len() != 32 {
        return Err(NameError::TokenWidth);
    }
    let mut result = [0; 16];
    for (slot, pair) in result.iter_mut().zip(bytes.as_chunks::<2>().0) {
        *slot = (hex(pair[0])? << 4) | hex(pair[1])?;
    }
    Ok(result)
}
macro_rules! token_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 16]);
        impl $name {
            pub const fn from_bytes(bytes: [u8; 16]) -> Self {
                Self(bytes)
            }
            pub const fn from_u128(value: u128) -> Self {
                Self(value.to_be_bytes())
            }
            pub const fn as_bytes(&self) -> &[u8; 16] {
                &self.0
            }
            pub fn parse(value: &str) -> Result<Self, NameError> {
                token(value.as_bytes()).map(Self)
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                for byte in self.0 {
                    write!(f, "{byte:02x}")?;
                }
                Ok(())
            }
        }
    };
}
token_type!(NamespaceToken);
token_type!(GenerationToken);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GenerationKey {
    pub namespace: NamespaceToken,
    pub generation: GenerationToken,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceSlot(u32);
impl ResourceSlot {
    pub const fn new(index: u32) -> Self {
        Self(index)
    }
    pub const fn index(self) -> u32 {
        self.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Member {
    Root,
    CommitMarker,
    Resource(ResourceSlot),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Address {
    pub key: GenerationKey,
    pub member: Member,
}
impl Address {
    /// Pure bounded parse, no string copies or allocation indexed by resource slot.
    /// None means unrelated; recognized malformed names return an explicit error.
    pub fn parse(value: &str) -> Result<Option<Self>, NameError> {
        let bytes = value.as_bytes();
        if value != "ho1" && !bytes.starts_with(RESERVED_PREFIX.as_bytes()) {
            return Ok(None);
        }
        if bytes.len() != 71 && bytes.len() != MAX_NAME_BYTES {
            return Err(NameError::NameWidth);
        }
        if bytes[36] != b'.' || bytes[69] != b'.' {
            return Err(NameError::Delimiter);
        }
        let key = GenerationKey {
            namespace: NamespaceToken(token(&bytes[4..36])?),
            generation: GenerationToken(token(&bytes[37..69])?),
        };
        let member = match &bytes[70..] {
            b"m" => Member::Root,
            b"c" => Member::CommitMarker,
            suffix if suffix.len() == 9 && suffix[0] == b'r' => {
                let mut index = 0u32;
                for byte in &suffix[1..] {
                    index = (index << 4) | hex(*byte)? as u32;
                }
                Member::Resource(ResourceSlot(index))
            }
            _ => return Err(NameError::Member),
        };
        Ok(Some(Self { key, member }))
    }
}
impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ho1.{}.{}.", self.key.namespace, self.key.generation)?;
        match self.member {
            Member::Root => f.write_str("m"),
            Member::CommitMarker => f.write_str("c"),
            Member::Resource(slot) => write!(f, "r{:08x}", slot.index()),
        }
    }
}
