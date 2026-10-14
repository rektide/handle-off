//! Streaming typed dump decoding: never build a generic JSON tree. The row
//! seed refuses an excess element before deserializing any of its fields.
use crate::data::{Error, KernelIdentity, MAX_ENTRIES, MAX_OUTPUT_BYTES, StoredEntry};
use handle_off_systemd_fdstore::FdName;
use serde::{
    Deserialize, Deserializer,
    de::{self, DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor},
};
use std::{cell::Cell, fmt};

struct Budget {
    max_entries: usize,
    exceeded: Cell<bool>,
}
struct Envelope<'a>(&'a Budget);
struct Data<'a>(&'a Budget);
struct Rows<'a>(&'a Budget);
struct Row<'a> {
    budget: &'a Budget,
    allowed: bool,
}

impl<'de> DeserializeSeed<'de> for Envelope<'_> {
    type Value = Vec<StoredEntry>;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(self)
    }
}
impl<'de> Visitor<'de> for Envelope<'_> {
    type Value = Vec<StoredEntry>;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("typed D-Bus dump reply")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        let mut signature = false;
        let mut entries = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "type" => {
                    if signature {
                        return Err(de::Error::custom("duplicate D-Bus signature"));
                    }
                    if map.next_value::<String>()? != "a(suuutuusu)" {
                        return Err(de::Error::custom("unexpected D-Bus signature"));
                    }
                    signature = true;
                }
                "data" => {
                    if entries.is_some() {
                        return Err(de::Error::custom("duplicate typed data"));
                    }
                    entries = Some(map.next_value_seed(Data(self.0))?);
                }
                _ => {
                    // Unknown envelope fields cannot allocate a Value subtree.
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }
        if !signature {
            return Err(de::Error::missing_field("type"));
        }
        entries.ok_or_else(|| de::Error::missing_field("data"))
    }
}
impl<'de> DeserializeSeed<'de> for Data<'_> {
    type Value = Vec<StoredEntry>;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(self)
    }
}
impl<'de> Visitor<'de> for Data<'_> {
    type Value = Vec<StoredEntry>;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("exactly one call result")
    }
    fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<Self::Value, S::Error> {
        let entries = seq
            .next_element_seed(Rows(self.0))?
            .ok_or_else(|| de::Error::custom("missing call result"))?;
        // Reject extra call results without building or traversing their value.
        seq.next_element_seed(Reject("expected exactly one call result"))?;
        Ok(entries)
    }
}
impl<'de> DeserializeSeed<'de> for Rows<'_> {
    type Value = Vec<StoredEntry>;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(self)
    }
}
impl<'de> Visitor<'de> for Rows<'_> {
    type Value = Vec<StoredEntry>;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("fdstore row array")
    }
    fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<Self::Value, S::Error> {
        // Do not reserve from an untrusted sequence size hint.
        let mut entries = Vec::new();
        while let Some(entry) = seq.next_element_seed(Row {
            budget: self.0,
            allowed: entries.len() < self.0.max_entries,
        })? {
            entries.push(entry);
        }
        Ok(entries)
    }
}
impl<'de> DeserializeSeed<'de> for Row<'_> {
    type Value = StoredEntry;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        if !self.allowed {
            self.budget.exceeded.set(true);
            return Err(de::Error::custom("fdstore entry limit exceeded"));
        }
        // Tuple deserialization validates exact field count and scalar types;
        // malformed nested containers cannot become generic Value allocations.
        let (name, mode, device_major, device_minor, inode, rdev_major, rdev_minor, path, flags) =
            <(String, u32, u32, u32, u64, u32, u32, String, u32)>::deserialize(deserializer)?;
        let name = FdName::new(name).map_err(|_| de::Error::custom("invalid manager FDNAME"))?;
        Ok(StoredEntry {
            name,
            identity: KernelIdentity {
                mode,
                device_major,
                device_minor,
                inode,
                rdev_major,
                rdev_minor,
            },
            path,
            flags,
        })
    }
}
struct Reject(&'static str);
impl<'de> DeserializeSeed<'de> for Reject {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, _: D) -> Result<(), D::Error> {
        Err(de::Error::custom(self.0))
    }
}

/// Parse typed call output without inferring acceptance. At most `max_entries`
/// rows are materialized, and an excess row is rejected before reading its fields.
/// Input (including retained strings) is bounded by [`MAX_OUTPUT_BYTES`]; vector
/// capacity grows only from accepted rows. JSON errors in an unread excess tail
/// are deliberately not diagnosed: exceeding the limit fails closed first.
pub fn parse_dump(bytes: &[u8], max_entries: usize) -> Result<Vec<StoredEntry>, Error> {
    if max_entries > MAX_ENTRIES {
        return Err(Error::InvalidConfiguration("entry limit exceeds ceiling"));
    }
    if bytes.len() > MAX_OUTPUT_BYTES {
        return Err(Error::OutputLimit);
    }
    let budget = Budget {
        max_entries,
        exceeded: Cell::new(false),
    };
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let result = Envelope(&budget)
        .deserialize(&mut deserializer)
        .and_then(|entries| deserializer.end().map(|()| entries));
    if budget.exceeded.get() {
        Err(Error::OutputLimit)
    } else {
        result.map_err(Error::Json)
    }
}
