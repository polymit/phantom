use std::collections::HashMap;

pub struct NetworkClient {
    client: rquest::Client,
}

impl NetworkClient {
    /// Create client with Chrome130 TLS impersonation
    #[tracing::instrument]
    pub fn new() -> Result<Self, NetworkError> {
        let client = rquest::Client::builder()
            .impersonate(rquest::Impersonate::Chrome130)
            .build()?;
        Ok(Self { client })
    }

    /// Fetch a URL, follow redirects, return response
    #[tracing::instrument(skip(self))]
    pub async fn fetch(&self, url: &str) -> Result<FetchResponse, NetworkError> {
        self.fetch_with_headers(url, HashMap::new()).await
    }

    /// Fetch with custom headers
    #[tracing::instrument(skip(self, headers))]
    pub async fn fetch_with_headers(
        &self,
        url: &str,
        headers: HashMap<String, String>,
    ) -> Result<FetchResponse, NetworkError> {
        let mut builder = self.client.get(url);
        for (k, v) in headers {
            builder = builder.header(&k, &v);
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
        })
    }
}

pub struct FetchResponse {
    pub status: u16,
    pub body: Vec<u8>,
    pub final_url: String,
    pub content_type: Option<String>,
    pub headers: HashMap<String, String>,
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
}

#[derive(thiserror::Error, Debug)]
pub enum NetworkError {
    #[error("request failed: {0}")]
    Request(#[from] rquest::Error),
    #[error("request timeout after {ms}ms")]
    Timeout { ms: u64 },
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("HTTP error {status}: {url}")]
    Http { status: u16, url: String },
}
