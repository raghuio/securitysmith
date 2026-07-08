use thiserror::Error;

/// Application-wide error type for SecuritySmith.
/// Replaces Result<T, String> with structured, matchable errors.
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Vault not unlocked")]
    VaultLocked,

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Client not found (id={0})")]
    ClientNotFound(u32),

    #[error("Engagement not found (id={0})")]
    EngagementNotFound(u32),

    #[error("Finding not found (id={0})")]
    FindingNotFound(u32),

    #[error("Credential not found (id={0})")]
    CredentialNotFound(u32),

    #[error("Contact not found (id={0})")]
    ContactNotFound(u32),

    #[error("Template not found (id={0})")]
    TemplateNotFound(u32),

    #[error("Report not found (id={0})")]
    ReportNotFound(u32),

    #[error("Document not found (id={0})")]
    DocumentNotFound(u32),

    #[error("Invoice not found (id={0})")]
    InvoiceNotFound(u32),

    #[error("Invalid input: {0}")]
    Validation(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Internal state error")]
    StatePoisoned,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Migration failed: {0}")]
    Migration(String),

    #[error("Vault not initialized")]
    VaultNotInitialized,

    #[error("Recovery not configured")]
    RecoveryNotConfigured,

    #[error("Recovery phrase invalid")]
    InvalidRecoveryPhrase,

    #[error("Rate limited: wait {0} seconds")]
    RateLimited(u64),

    #[error("External service error: {0}")]
    ExternalService(String),

    #[error("Password too weak: {0}")]
    WeakPassword(String),

    #[error("Export failed: {0}")]
    Export(String),

    #[error("Import failed: {0}")]
    Import(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Zip error: {0}")]
    Zip(String),

    #[error("CSV error: {0}")]
    Csv(String),

    #[error("URL parse error: {0}")]
    UrlParse(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Image error: {0}")]
    Image(String),

    #[error("Password hash error: {0}")]
    PasswordHash(String),

    #[error("Tauri error: {0}")]
    Tauri(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Notification error: {0}")]
    Notification(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Scope item not found (id={0})")]
    ScopeItemNotFound(u32),

    #[error("Generic error: {0}")]
    Generic(String),
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Generic(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Generic(s.to_string())
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(e: zip::result::ZipError) -> Self {
        AppError::Zip(e.to_string())
    }
}

impl From<csv::Error> for AppError {
    fn from(e: csv::Error) -> Self {
        AppError::Csv(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Http(e.to_string())
    }
}

impl From<image::error::ImageError> for AppError {
    fn from(e: image::error::ImageError) -> Self {
        AppError::Image(e.to_string())
    }
}

impl From<argon2::Error> for AppError {
    fn from(e: argon2::Error) -> Self {
        AppError::PasswordHash(e.to_string())
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::Tauri(e.to_string())
    }
}

impl From<aes_gcm::Error> for AppError {
    fn from(e: aes_gcm::Error) -> Self {
        AppError::Encryption(e.to_string())
    }
}

/// Convenience type alias for Result with AppError.
pub type Result<T> = std::result::Result<T, AppError>;

/// Convert AppError to String for Tauri command responses.
/// Commands should use .map_err(AppError::from) and return Result<T, String>
/// until the full migration to AppError is complete.
pub fn to_string(err: AppError) -> String {
    err.to_string()
}

impl From<AppError> for String {
    fn from(err: AppError) -> Self {
        err.to_string()
    }
}
