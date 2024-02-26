use {
    bytes::Bytes,
    serde::{Deserialize, Serialize},
    std::borrow::Cow,
};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DataPayload<'a> {
    Link(Cow<'a, str>),
    Text(Cow<'a, str>),
    Image(Bytes),
}
