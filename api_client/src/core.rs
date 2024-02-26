use reqwest::Client;

pub struct ApiCore {
    _client: Client,
}

impl Default for ApiCore {
    fn default() -> Self {
        Self {
            _client: platform::create_client(),
        }
    }
}

#[cfg(not(target_family = "wasm"))]
mod platform {
    use {
        reqwest::{redirect::Policy, Client},
        std::time::Duration,
    };

    pub fn create_client() -> Client {
        Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .redirect(Policy::limited(5))
            .build()
            .expect("create api client internal core")
    }
}

#[cfg(target_family = "wasm")]
mod platform {
    use reqwest::Client;

    pub fn create_client() -> Client {
        Client::default()
    }
}
