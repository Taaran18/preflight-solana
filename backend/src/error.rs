pub enum AppError {
    User(String),
    Internal(anyhow::Error),
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        AppError::Internal(err.into())
    }
}

impl AppError {
    pub fn message(&self) -> String {
        match self {
            AppError::User(message) => redact(message),
            AppError::Internal(err) => redact(&format!("Unexpected error: {err:#}")),
        }
    }

    pub fn is_user(&self) -> bool {
        matches!(self, AppError::User(_))
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub fn user<T>(message: impl Into<String>) -> AppResult<T> {
    Err(AppError::User(message.into()))
}

pub fn user_err(message: impl Into<String>) -> AppError {
    AppError::User(message.into())
}

pub fn redact(text: &str) -> String {
    const MARKERS: [&str; 4] = ["api-key=", "api_key=", "apikey=", "key="];
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    'scan: while !rest.is_empty() {
        let lower = rest.to_ascii_lowercase();
        let mut best: Option<(usize, usize)> = None;
        for marker in MARKERS {
            if let Some(at) = lower.find(marker) {
                if best.is_none_or(|(start, _)| at < start) {
                    best = Some((at, marker.len()));
                }
            }
        }
        let Some((at, len)) = best else {
            out.push_str(rest);
            break 'scan;
        };
        out.push_str(&rest[..at + len]);
        let after = &rest[at + len..];
        let secret_len = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(after.len());
        if secret_len > 0 {
            out.push_str("***");
        }
        rest = &after[secret_len..];
    }
    out
}
