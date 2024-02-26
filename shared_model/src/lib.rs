use std::borrow::Cow;

pub mod auth_api;
pub mod data_payload;

pub(crate) const fn empty_str() -> Cow<'static, str> {
    Cow::Borrowed("")
}
