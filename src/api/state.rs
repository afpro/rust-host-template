use std::sync::Arc;

#[derive(Clone)]
pub struct HostState {
    inner: Arc<HostStateInner>,
}

struct HostStateInner {
    remote_header: Option<String>,
}

impl HostState {
    pub fn new(remote_header: Option<String>) -> Self {
        Self {
            inner: Arc::new(HostStateInner { remote_header }),
        }
    }

    pub fn remote_header(&self) -> Option<&str> {
        self.inner.remote_header.as_deref()
    }
}
