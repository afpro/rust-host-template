use std::net::{IpAddr, SocketAddr};

use anyhow::{Result, anyhow};
use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{Extensions, HeaderMap, HeaderValue, request::Parts},
};
use tracing::{debug, error, info, instrument};
use uuid::Uuid;

use crate::{
    api::state::HostState,
    scaffold::{
        access_log::AccessLogId,
        rest::{RestResponse, RestStatus},
    },
};

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct RemoteAddr {
    pub ip: IpAddr,
    pub port: Option<u16>,
}

impl RemoteAddr {
    #[instrument("parse-remote", skip_all)]
    pub fn parse(
        extensions: &mut Extensions,
        headers: &HeaderMap<HeaderValue>,
        header_name: Option<&str>,
    ) -> Result<Self> {
        fn parse_inner(
            extensions: &Extensions,
            headers: &HeaderMap<HeaderValue>,
            header_name: Option<&str>,
        ) -> Result<RemoteAddr> {
            // check header
            if let Some(header_name) = header_name {
                match headers.get(header_name) {
                    Some(header_value) => {
                        let Ok(header_str) = header_value.to_str() else {
                            return Err(anyhow!(
                                "can't extract remote from header value: {:#?}",
                                header_value
                            ));
                        };

                        if let Ok(addr) = header_str.parse::<SocketAddr>() {
                            debug!(%header_name, ip=%addr.ip(), port=addr.port(), "remote header parsed as socket addr");
                            return Ok(RemoteAddr {
                                ip: addr.ip(),
                                port: Some(addr.port()),
                            });
                        }

                        if let Ok(addr) = header_str.parse::<IpAddr>() {
                            debug!(%header_name, ip=%addr, "remote header parsed as ip addr");
                            return Ok(RemoteAddr {
                                ip: addr,
                                port: None,
                            });
                        }

                        return Err(anyhow!(
                            "can't extract addr from header value: {:#?}",
                            header_str
                        ));
                    }
                    None => {
                        info!("can't extract remote from header");
                    }
                };
            }

            // check direct
            match extensions.get::<ConnectInfo<SocketAddr>>() {
                Some(ConnectInfo(addr)) => {
                    debug!(%addr, "remote connect info found");
                    Ok(RemoteAddr {
                        ip: addr.ip(),
                        port: Some(addr.port()),
                    })
                }
                None => Err(anyhow!("no remote header or connect info can be used")),
            }
        }

        // check already present in extension
        if let Some(in_extension) = extensions.get::<RemoteAddr>() {
            return Ok(*in_extension);
        }

        // run parse
        let parsed = parse_inner(extensions, headers, header_name)?;
        extensions.insert(parsed);
        Ok(parsed)
    }

    pub fn save_extension(self, extensions: &mut Extensions) {
        extensions.insert(self);
    }
}

impl FromRequestParts<HostState> for RemoteAddr {
    type Rejection = RestResponse;

    #[instrument("extract-remote", skip_all)]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &HostState,
    ) -> Result<Self, Self::Rejection> {
        // get access id
        let access_id = match parts.extensions.get::<AccessLogId>() {
            Some(v) => v.0,
            None => {
                error!("access id not found!");
                return Err(RestResponse::fail(RestStatus::Unknown, Uuid::nil()));
            }
        };

        // parse
        let addr = Self::parse(&mut parts.extensions, &parts.headers, state.remote_header())
            .map_err(|err| {
                info!(err=%err, "parse remote addr error");
                RestResponse::fail(RestStatus::Unknown, access_id)
            })?;

        // save in extension
        addr.save_extension(&mut parts.extensions);

        // finish extraction
        Ok(addr)
    }
}
