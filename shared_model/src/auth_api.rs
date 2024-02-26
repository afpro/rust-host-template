use {
    chrono::{DateTime, Utc},
    crate::empty_str,
    serde::{Deserialize, Serialize},
    std::borrow::Cow,
};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AuthRequest<'a> {
    Login {
        username: Cow<'a, str>,
        password: Cow<'a, str>,
    },
    RefreshToken {
        token: Cow<'a, str>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AuthResponse<'a> {
    Success {
        token: Cow<'a, str>,
        expire: DateTime<Utc>,
    },
    InvalidUsernameOrPassword,
    InvalidToken,
    InternalServerError {
        #[serde(default = "empty_str")]
        message: Cow<'a, str>,
    },
}
