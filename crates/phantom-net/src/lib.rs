use std::collections::HashMap;

pub struct NetworkClient {
    rquest_client: rquest::Client,
    fallback_client: reqwest::Client,
}

impl NetworkClient {
    #[tracing::instrument]
    pub fn new() -> Result<Self, NetworkError> {
        let rquest_client = rquest::Client::builder()
            .impersonate(rquest::Impersonate::Chrome130)
            .build()?;

        let fallback_client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| NetworkError::Fallback(e.to_string()))?;

        Ok(Self {
            rquest_client,
            fallback_client,
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn fetch(&self, url: &str) -> Result<FetchResponse, NetworkError> {
        self.fetch_with_headers(url, HashMap::new()).await
    }

    #[tracing::instrument(skip(self, headers))]
    pub async fn fetch_with_headers(
        &self,
        url: &str,
        headers: HashMap<String, String>,
    ) -> Result<FetchResponse, NetworkError> {
        tracing::debug!(url = %url, "fetch via reqwest rustls");
        self.fetch_reqwest(url, &headers).await
    }

    #[allow(dead_code)]
    async fn fetch_rquest(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> Result<FetchResponse, NetworkError> {
        let mut builder = self.rquest_client.get(url);
        for (k, v) in headers {
            builder = builder.header(k.as_str(), v.as_str());
        }
        let response = builder.send().await?;

        let status = response.status().as_u16();
        let final_url = response.url().to_string();
        let content_type = response
            .headers()
            .get(rquest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let mut resp_headers = HashMap::new();
        for (k, v) in response.headers().iter() {
            resp_headers.insert(k.as_str().to_string(), v.to_str().unwrap_or("").to_string());
        }

        let body = response.bytes().await?.to_vec();

        Ok(FetchResponse {
            status,
            body,
            final_url,
            content_type,
            headers: resp_headers,
            tls_tier: TlsTier::BoringSSL,
        })
    }

    async fn fetch_reqwest(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> Result<FetchResponse, NetworkError> {
        let mut builder = self.fallback_client.get(url);
        for (k, v) in headers {
            builder = builder.header(k.as_str(), v.as_str());
        }
        let response = builder
            .send()
            .await
            .map_err(|e| NetworkError::Fallback(e.to_string()))?;

        let status = response.status().as_u16();
        let final_url = response.url().to_string();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let mut resp_headers = HashMap::new();
        for (k, v) in response.headers().iter() {
            resp_headers.insert(k.as_str().to_string(), v.to_str().unwrap_or("").to_string());
        }

        let body = response
            .bytes()
            .await
            .map_err(|e| NetworkError::Fallback(e.to_string()))?
            .to_vec();

        Ok(FetchResponse {
            status,
            body,
            final_url,
            content_type,
            headers: resp_headers,
            tls_tier: TlsTier::NativeTls,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TlsTier {
    BoringSSL,
    NativeTls,
}

pub struct FetchResponse {
    pub status: u16,
    pub body: Vec<u8>,
    pub final_url: String,
    pub content_type: Option<String>,
    pub headers: HashMap<String, String>,
    pub tls_tier: TlsTier,
}

impl FetchResponse {
    pub fn body_as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.body)
    }

    pub fn is_html(&self) -> bool {
        self.content_type
            .as_ref()
            .map(|s| s.contains("text/html"))
            .unwrap_or(false)
    }

    pub fn is_spoofed(&self) -> bool {
        self.tls_tier == TlsTier::BoringSSL
    }
}

#[derive(thiserror::Error, Debug)]
pub enum NetworkError {
    #[error("request failed: {0}")]
    Request(#[from] rquest::Error),
    #[error("fallback request failed: {0}")]
    Fallback(String),
    #[error("request timeout after {ms}ms")]
    Timeout { ms: u64 },
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("HTTP error {status}: {url}")]
    Http { status: u16, url: String },
}
