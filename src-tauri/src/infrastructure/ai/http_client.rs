use std::time::Duration;

/// HTTP client boundary for AI providers. Redirects are never followed automatically,
/// eliminating any chance of forwarding credentials to a different origin.
pub struct SecureHttpClient {
    client: reqwest::Client,
}

impl SecureHttpClient {
    /// Builds the shared provider client with bounded connection and request timeouts.
    ///
    /// # Errors
    ///
    /// Returns a `reqwest` error if TLS or client initialization fails.
    pub fn new() -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_mins(2))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self { client })
    }

    /// Starts a GET request with the client-wide secure redirect policy.
    pub fn get(&self, url: impl reqwest::IntoUrl) -> reqwest::RequestBuilder {
        self.client.get(url)
    }
}

#[cfg(test)]
mod tests {
    use reqwest::header::AUTHORIZATION;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers};

    #[tokio::test]
    async fn cross_origin_redirect_never_receives_authorization() {
        let source = MockServer::start().await;
        let target = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .respond_with(
                ResponseTemplate::new(302)
                    .insert_header("location", format!("{}/target", target.uri())),
            )
            .mount(&source)
            .await;
        Mock::given(matchers::method("GET"))
            .and(matchers::header(
                "authorization",
                "Bearer evolish-secret-canary",
            ))
            .respond_with(ResponseTemplate::new(500))
            .expect(0)
            .mount(&target)
            .await;

        let response = super::SecureHttpClient::new()
            .expect("secure client")
            .get(format!("{}/source", source.uri()))
            .header(AUTHORIZATION, "Bearer evolish-secret-canary")
            .send()
            .await
            .expect("redirect response");

        assert_eq!(response.status(), reqwest::StatusCode::FOUND);
        target.verify().await;
    }
}
