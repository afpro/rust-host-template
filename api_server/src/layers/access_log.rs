use {
    axum::{
        extract::{ConnectInfo, Request},
        response::Response,
    },
    pin_project::{pin_project, pinned_drop},
    std::{
        borrow::Cow,
        fmt::Debug,
        future::Future,
        net::SocketAddr,
        pin::Pin,
        task::{ready, Context, Poll},
        time::Instant,
    },
    tower::Service,
    tower_layer::Layer,
    tracing::{error, info, span, warn, Level, Span},
    uuid::Uuid,
};

#[derive(Copy, Clone)]
pub struct AccessLog;

impl<S> Layer<S> for AccessLog {
    type Service = AccessLogService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AccessLogService { inner }
    }
}

#[derive(Clone)]
pub struct AccessLogService<S> {
    inner: S,
}

impl<S> AccessLogService<S> {
    fn extract_remote<B>(req: &Request<B>) -> Cow<'static, str> {
        match req.extensions().get::<ConnectInfo<SocketAddr>>() {
            Some(ConnectInfo(addr)) => addr.to_string().into(),
            None => "unknown".into(),
        }
    }
}

impl<S, Req, Resp> Service<Request<Req>> for AccessLogService<S>
where
    S: Service<Request<Req>, Response = Response<Resp>>,
    S::Future: Future<Output = Result<Response<Resp>, S::Error>>,
    S::Error: Debug,
{
    type Response = Response<Resp>;
    type Error = S::Error;
    type Future = AccessLogServiceFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Req>) -> Self::Future {
        let span = span!(Level::INFO, "request", id=%Uuid::new_v4().simple());
        {
            let _guard = span.enter();
            info!(
                target: "request",
                remote = %Self::extract_remote(&req),
                method = %req.method(),
                uri = %req.uri(),
                headers = ?req.headers(),
                "begin",
            );
        }

        AccessLogServiceFuture::new(span, self.inner.call(req))
    }
}

#[pin_project(PinnedDrop)]
pub struct AccessLogServiceFuture<F> {
    span: Span,
    done: bool,
    start: Instant,
    #[pin]
    inner: F,
}

impl<F> AccessLogServiceFuture<F> {
    fn new(span: Span, inner: F) -> Self {
        Self {
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
            let cost = Instant::now().duration_since(*this.start);
            match &result {
                Ok(response) => {
                    if (400..=599).contains(&response.status().as_u16()) {
                        error!(
                            target: "request",
                            status = response.status().as_u16(),
                            cost = cost.as_millis(),
                            "end with error status"
                        );
                    } else {
                        info!(
                            target: "request",
                            status = response.status().as_u16(),
                            cost = cost.as_millis(),
                            "end ok"
                        );
                    }
                }
                Err(err) => {
                    error!(
                        target: "request",
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
            let cost = Instant::now().duration_since(self.start);
            warn!(
                target: "request",
                cost = cost.as_millis(),
                "request connection dropped before finish",
            );
        }
    }
}
