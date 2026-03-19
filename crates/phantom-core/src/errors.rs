use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("DOM error: {0}")]
    Dom(#[from] DomError),

    #[error("Network error from core: {0}")]
    Network(#[from] phantom_net::NetworkError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Utf8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum DomError {
    #[error("element not found: selector '{selector}'")]
    ElementNotFound { selector: String },

    #[error("stale element reference: '{selector}'")]
    StaleElement { selector: String },

    #[error("invalid selector: {0}")]
    InvalidSelector(String),

    #[error("element not interactable: {reason}")]
    NotInteractable { reason: String, selector: String },
}
