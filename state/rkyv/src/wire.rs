//! Private wire owners. Core types deliberately have no serialization derives.
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize)]
pub(crate) enum Image {
    Manifest(Manifest),
    Extensions(Vec<Extension>),
}
#[derive(Archive, Serialize, Deserialize)]
pub(crate) struct Manifest {
    pub version: u32,
    pub generation: String,
    pub resources: Vec<String>,
    pub bundles: Vec<Bundle>,
    pub extensions: Vec<Extension>,
}
#[derive(Archive, Serialize, Deserialize)]
pub(crate) struct Bundle {
    pub id: String,
    pub members: Vec<Member>,
    pub extensions: Vec<Extension>,
}
#[derive(Archive, Serialize, Deserialize)]
pub(crate) struct Member {
    pub id: String,
    pub resource: String,
    pub extensions: Vec<Extension>,
}
#[derive(Archive, Serialize, Deserialize)]
pub(crate) struct Extension {
    pub key: String,
    pub format: String,
    pub version: u32,
    pub required: bool,
    pub payload: Payload,
}
#[derive(Archive, Serialize, Deserialize)]
pub(crate) enum Payload {
    Inline(Vec<u8>),
    ResourceRange {
        resource: String,
        offset: u64,
        length: u64,
    },
}
