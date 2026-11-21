//! How the core reaches a sync server. The trait keeps the network out of the sync logic, so tests run
//! it against an in-memory server, and builds without the `http` feature carry no network code at all.

use serde_json::Value;

#[derive(Debug)]
pub enum TransportError {
    /// No answer: offline, a timeout, or a name that doesn't resolve. Changes wait for the next try.
    Unreachable(String),
    /// The server answered with an error, in its `{ "code", "message" }` form.
    Rejected { status: u16, code: String, message: String },
}

pub trait Transport: Send + Sync {
    /// Sends `body` as JSON and returns the JSON reply. `token` goes in the Authorization header.
    fn post(&self, url: &str, token: Option<&str>, body: &Value) -> Result<Value, TransportError>;
}

#[cfg(feature = "http")]
pub use http::HttpTransport;

#[cfg(feature = "http")]
mod http {
    use std::time::Duration;

    use serde_json::Value;

    use super::{Transport, TransportError};

    /// HTTPS with the Mozilla root certificates built in, so it works the same on Ubuntu and Android.
    pub struct HttpTransport {
        agent: ureq::Agent,
    }

    impl Default for HttpTransport {
        fn default() -> Self {
            let agent = ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(10))
                .timeout(Duration::from_secs(60))
                .user_agent(concat!("Dayweave/", env!("CARGO_PKG_VERSION")))
                .build();
            HttpTransport { agent }
        }
    }

    impl Transport for HttpTransport {
        fn post(&self, url: &str, token: Option<&str>, body: &Value) -> Result<Value, TransportError> {
            let mut request = self.agent.post(url).set("Content-Type", "application/json");
            if let Some(token) = token {
                request = request.set("Authorization", &format!("Bearer {token}"));
            }
            match request.send_string(&body.to_string()) {
                Ok(response) => {
                    let text = response
                        .into_string()
                        .map_err(|error| TransportError::Unreachable(error.to_string()))?;
                    serde_json::from_str(&text).map_err(|_| TransportError::Rejected {
                        status: 200,
                        code: "bad_response".into(),
                        message: "The server's reply couldn't be read. Is this a Dayweave server?".into(),
                    })
                }
                Err(ureq::Error::Status(status, response)) => {
                    let reply: Value = response
                        .into_string()
                        .ok()
                        .and_then(|text| serde_json::from_str(&text).ok())
                        .unwrap_or(Value::Null);
                    Err(TransportError::Rejected {
                        status,
                        code: reply["code"].as_str().unwrap_or("http_error").to_string(),
                        message: reply["message"]
                            .as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| format!("The server answered with error {status}")),
                    })
                }
                Err(ureq::Error::Transport(error)) => Err(TransportError::Unreachable(error.to_string())),
            }
        }
    }
}
