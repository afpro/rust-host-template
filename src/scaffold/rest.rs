use std::{fmt::Debug, str::FromStr, time::Duration};

use axum::{
    Json,
    extract::{FromRequest, FromRequestParts, Path, Query, Request},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, IntoResponseParts, Response, ResponseParts},
};
use axum_extra::{TypedHeader, extract::CookieJar, headers::CacheControl};
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::json;
use tracing::info;
use uuid::Uuid;

use crate::scaffold::{access_log::AccessLogId, pretty::Pretty};

#[macro_export]
macro_rules! rest_must_success {
    ($e:expr, $access_id:expr $(, $context:literal)? $(,)? ) => {
        match $e {
            Ok(v) => v,
            Err(err) => {
                let err = $crate::misc::pretty::Pretty(err);
                tracing::info!(target: "guard", ?err, $(context=%$context,)? "caught unknown error");

                return $crate::http::rest::RestResponse::fail(
                    $crate::http::rest::RestStatus::Unknown,
                    $access_id,
                );
            }
        }
    };
}

#[macro_export]
macro_rules! permit_check {
    ($cache:ident, $key:expr, $access_id:expr, acquire=$acquire:expr, duration=$duration_in_secs:expr, permit_in_duration=$permit_in_duration:expr $(,)?) => {{
        use g_base::cache::permit_acquire::{PermitAcquire, PermitAcquireConfig};

        let permit_key = &$key;

        let permit_status = $crate::rest_must_success!(
            $cache.acquire_permit(
                PermitAcquireConfig {
                    name: permit_key,
                    duration_in_secs: $duration_in_secs,
                    permit_per_sec: $permit_in_duration / $duration_in_secs,
                },
                $acquire,
            )
            .await,
            $access_id,
            "check permit"
        );

        if permit_status.acquired < $acquire {
            tracing::info!(%permit_status, %permit_key, "permit not match");
            return $crate::http::rest::RestResponse::fail($crate::http::rest::RestStatus::RateLimit, $access_id);
        }
    }};
    ($cache:ident, $key:expr, $access_id:expr, duration=$duration_in_secs:expr, permit_in_duration=$permit_in_duration:expr $(,)?) => {
        $crate::permit_check!($cache, $key, $access_id, acquire=1.0, duration=$duration_in_secs, permit_in_duration=$permit_in_duration)
    };
}

#[derive(Serialize, Deserialize, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
#[repr(u16)]
#[serde(rename_all = "snake_case")]
pub enum RestStatus {
    #[default]
    Ok = 0,
    Unknown,
    BadRequest,
}

pub struct RestResponse<B = ()> {
    status: RestStatus,
    access_id: Uuid,
    body: Option<B>,
    cookie_jar: Option<CookieJar>,
    s_cache: Option<Duration>,
}

impl<B> RestResponse<B> {
    pub fn get_status(&self) -> RestStatus {
        self.status
    }

    pub fn get_access_id(&self) -> Uuid {
        self.access_id
    }

    pub fn is_ok(&self) -> bool {
        self.status == RestStatus::Ok
    }

    pub fn ok(access_id: Uuid, body: B) -> Self {
        Self {
            status: RestStatus::Ok,
            access_id,
            body: Some(body),
            cookie_jar: None,
            s_cache: None,
        }
    }

    pub fn ok_none(access_id: Uuid) -> Self {
        Self {
            status: RestStatus::Ok,
            access_id,
            body: None,
            cookie_jar: None,
            s_cache: None,
        }
    }

    pub fn fail(status: RestStatus, access_id: Uuid) -> Self {
        debug_assert_ne!(status, RestStatus::Ok);

        Self {
            status,
            access_id,
            body: None,
            cookie_jar: None,
            s_cache: None,
        }
    }

    pub fn modify_cookie<F>(&mut self, modify: F)
    where
        F: FnOnce(CookieJar) -> CookieJar,
    {
        let jar = self.cookie_jar.take().unwrap_or_default();
        let jar = modify(jar);
        self.cookie_jar = Some(jar);
    }

    pub fn set_s_cache(&mut self, cache: Duration) {
        self.s_cache = Some(cache);
    }

    pub fn with_s_cache(mut self, cache: Duration) -> Self {
        self.s_cache = Some(cache);
        self
    }

    pub fn with_s_cache_seconds(self, seconds: u64) -> Self {
        self.with_s_cache(Duration::new(seconds, 0))
    }
}

impl<B: Default> RestResponse<B> {
    pub fn ok_default(access_id: Uuid) -> Self {
        Self::ok(access_id, <B as Default>::default())
    }
}

impl<B> IntoResponse for RestResponse<B>
where
    B: Serialize,
{
    fn into_response(self) -> Response {
        let Self {
            status,
            access_id,
            body,
            cookie_jar,
            s_cache,
        } = self;

        let status_code = match status {
            RestStatus::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::OK,
        };

        let json_body = match body {
            Some(body) => json!({
                "status": status,
                "access_id": access_id,
                "body": body,
            }),
            None => json!({
                "status": status,
                "access_id": access_id,
            }),
        };

        (status_code, cookie_jar, CachePart(s_cache), Json(json_body)).into_response()
    }
}

#[derive(Copy, Clone)]
pub struct CachePart(pub Option<Duration>);

impl IntoResponseParts for CachePart {
    type Error = <TypedHeader<CacheControl> as IntoResponseParts>::Error;

    fn into_response_parts(self, res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        let cache = if let Some(age) = self.0
            && !cfg!(debug_assertions)
        {
            CacheControl::new()
                .with_public()
                .with_max_age(age)
                .with_s_max_age(age)
        } else {
            CacheControl::new().with_no_cache()
        };

        TypedHeader(cache).into_response_parts(res)
    }
}

#[derive(Serialize)]
pub struct PagedRequest<N> {
    pub offset: Option<N>,
    pub count: Option<N>,
}

impl<'de, N> Deserialize<'de> for PagedRequest<N>
where
    N: FromStr + DeserializeOwned,
    <N as FromStr>::Err: Debug,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum TypeCheck<'a, T> {
            Direct(T),
            Text(&'a str),
        }

        #[derive(Deserialize)]
        struct Data<'a, T> {
            #[serde(borrow)]
            offset: Option<TypeCheck<'a, T>>,
            #[serde(borrow)]
            count: Option<TypeCheck<'a, T>>,
        }

        fn extract_type_check<T: FromStr>(
            v: Option<TypeCheck<T>>,
        ) -> Result<Option<T>, <T as FromStr>::Err> {
            match v {
                Some(TypeCheck::Direct(v)) => Ok(Some(v)),
                Some(TypeCheck::Text(v)) => <T as FromStr>::from_str(v).map(Some),
                None => Ok(None),
            }
        }

        let data = Data::<'de, N>::deserialize(deserializer)?;
        Ok(Self {
            offset: extract_type_check(data.offset).map_err(|err| {
                serde::de::Error::custom(format!("parse offset from str error: {:#?}", err))
            })?,
            count: extract_type_check(data.count).map_err(|err| {
                serde::de::Error::custom(format!("parse count from str error: {:#?}", err))
            })?,
        })
    }
}

#[derive(Clone, Debug, derive_more::From, derive_more::Deref, derive_more::DerefMut)]
pub struct RestJson<T>(pub T);

impl<T, S> FromRequest<S> for RestJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = RestResponse;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let access_id = req
            .extensions()
            .get::<AccessLogId>()
            .map(|v| v.uuid())
            .unwrap_or_else(Uuid::nil);

        match Json::<T>::from_request(req, state).await {
            Ok(v) => Ok(v.0.into()),
            Err(err) => {
                info!(err=?Pretty(err), "parse json error");
                Err(RestResponse::fail(RestStatus::BadRequest, access_id))
            }
        }
    }
}

macro_rules! gen_wrapper {
    ($name:ident, $wrap:ident, $error:literal) => {
        #[derive(Clone, Debug, derive_more::From, derive_more::Deref, derive_more::DerefMut)]
        pub struct $name<T>(pub T);

        impl<T, S> FromRequestParts<S> for $name<T>
        where
            T: DeserializeOwned + Send,
            S: Send + Sync,
        {
            type Rejection = RestResponse;

            async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
                let access_id = parts
                    .extensions
                    .get::<AccessLogId>()
                    .map(|v| v.uuid())
                    .unwrap_or_else(Uuid::nil);

                match $wrap::<T>::from_request_parts(parts, state).await {
                    Ok(v) => Ok(v.0.into()),
                    Err(err) => {
                        info!(err=?Pretty(err), $error);
                        Err(RestResponse::fail(RestStatus::BadRequest, access_id))
                    }
                }
            }
        }
    };
}

gen_wrapper!(RestPath, Path, "extract path error");
gen_wrapper!(RestQuery, Query, "extract query error");
