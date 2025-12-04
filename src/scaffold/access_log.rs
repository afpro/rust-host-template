use std::{
    fmt::Debug,
    future::Future,
    pin::Pin,
    task::{Context, Poll, ready},
    time::Instant,
};

use axum::{
    extract::Request,
    response::{IntoResponse, Response},
};
use pin_project::{pin_project, pinned_drop};
use tower::Service;
use tower_layer::Layer;
use tracing::{Level, Span, debug, error, info, span, warn};
use uuid::Uuid;

use crate::{
    api::state::HostState,
    scaffold::{
        pretty::PrettyOpt,
        remote_addr::RemoteAddr,
        rest::{RestResponse, RestStatus},
    },
};

#[derive(Copy, Clone)]
pub struct AccessLogId(pub Uuid);

impl From<AccessLogId> for Uuid {
    fn from(value: AccessLogId) -> Self {
        value.0
    }
}

impl AccessLogId {
    pub fn uuid(self) -> Uuid {
        self.0
    }
}

#[derive(Clone)]
pub struct AccessLog {
    state: HostState,
}

impl AccessLog {
    pub fn new(state: HostState) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for AccessLog {
    type Service = AccessLogService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AccessLogService {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub struct AccessLogService<S> {
    inner: S,
    state: HostState,
}

impl<S, Req, Resp> Service<Request<Req>> for AccessLogService<S>
where
    S: Service<Request<Req>, Response = Response<Resp>>,
    S::Future: Future<Output = Result<Response<Resp>, S::Error>>,
    S::Error: Debug,
{
    type Response = AccessLogServiceBody<Resp>;
    type Error = S::Error;
    type Future = AccessLogServiceOptFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Req>) -> Self::Future {
        let id = Uuid::new_v4();
        req.extensions_mut().insert(AccessLogId(id));

        let span = span!(Level::INFO, "request", access_id=%id);
        let req = {
            let _guard = span.enter();
            let (mut parts, body) = req.into_parts();
            let remote = match RemoteAddr::parse(
                &mut parts.extensions,
                &parts.headers,
                self.state.remote_header(),
            ) {
                Ok(v) => v,
                Err(err) => {
                    info!(%err, "extract remote error");
                    return AccessLogServiceOptFuture::NoRemoteAddr { access_id: id };
                }
            };
            info!(
                target: "request",
                request_phase = "begin",
                remote_ip = %remote.ip,
                remote_port = %PrettyOpt(remote.port),
                method = %parts.method,
                uri = %parts.uri,
                "begin",
            );
            if cfg!(debug_assertions) {
                debug!(target: "request", headers=?parts.headers, "dump headers");
            }
            Request::from_parts(parts, body)
        };

        AccessLogServiceOptFuture::Next(AccessLogServiceFuture::new(
            req.uri().path().to_string(),
            span,
            self.inner.call(req),
        ))
    }
}

#[pin_project(project = AccessLogServiceOptFutureProj)]
pub enum AccessLogServiceOptFuture<F> {
    NoRemoteAddr { access_id: Uuid },
    Next(#[pin] AccessLogServiceFuture<F>),
}

pub enum AccessLogServiceBody<B> {
    NoRemoteAddr { access_id: Uuid },
    Inner(Response<B>),
}

impl<B> IntoResponse for AccessLogServiceBody<B>
where
    Response<B>: IntoResponse,
{
    fn into_response(self) -> Response {
        match self {
            AccessLogServiceBody::NoRemoteAddr { access_id } => {
                RestResponse::<()>::fail(RestStatus::Unknown, access_id).into_response()
            }
            AccessLogServiceBody::Inner(inner) => inner.into_response(),
        }
    }
}

impl<F, B, E> Future for AccessLogServiceOptFuture<F>
where
    F: Future<Output = Result<Response<B>, E>>,
    E: Debug,
{
    type Output = Result<AccessLogServiceBody<B>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            AccessLogServiceOptFutureProj::NoRemoteAddr { access_id } => {
                Poll::Ready(Ok(AccessLogServiceBody::NoRemoteAddr {
                    access_id: *access_id,
                }))
            }
            AccessLogServiceOptFutureProj::Next(fut) => {
                fut.poll(cx).map_ok(AccessLogServiceBody::Inner)
            }
        }
    }
}

#[pin_project(PinnedDrop)]
pub struct AccessLogServiceFuture<F> {
    pathname: String,
    span: Span,
    done: bool,
    start: Instant,
    #[pin]
    inner: F,
}

impl<F> AccessLogServiceFuture<F> {
    fn new(pathname: String, span: Span, inner: F) -> Self {
        Self {
            pathname,
            span,
            done: false,
            start: Instant::now(),
            inner,
        }
    }
}

impl<F, B, E> Future for AccessLogServiceFuture<F>
where
    F: Future<Output = Result<Response<B>, E>>,
    E: Debug,
{
    type Output = Result<Response<B>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let _guard = this.span.enter();
        let result = ready!(this.inner.poll(cx));
        if !*this.done {
            *this.done = true;
            let cost = Instant::now().saturating_duration_since(*this.start);
            match &result {
                Ok(response) => {
                    if (400..=599).contains(&response.status().as_u16()) {
                        error!(
                            target: "request",
                            request_phase = "end",
                            request_end_type = "error status",
                            pathname = %this.pathname,
                            status = response.status().as_u16(),
                            cost = cost.as_millis(),
                            "end with error status"
                        );
                    } else {
                        info!(
                            target: "request",
                            request_phase = "end",
                            request_end_type = "success",
                            pathname = %this.pathname,
                            status = response.status().as_u16(),
                            cost = cost.as_millis(),
                            "end ok"
                        );
                    }
                }
                Err(err) => {
                    error!(
                        target: "request",
                        request_phase = "end",
                        request_end_type = "server error",
                        pathname = %this.pathname,
                        cost = cost.as_millis(),
                        "end with uncached error {:?}", err
                    );
                }
            }
        }
        Poll::Ready(result)
    }
}

#[pinned_drop]
impl<F> PinnedDrop for AccessLogServiceFuture<F> {
    fn drop(self: Pin<&mut Self>) {
        if !self.done {
            let _guard = self.span.enter();
            let cost = Instant::now().saturating_duration_since(self.start);
            warn!(
                target: "request",
                request_phase = "end",
                request_end_type = "dropped",
                pathname = %self.pathname,
                cost = cost.as_millis(),
                "request connection dropped before finish",
            );
        }
    }
}
