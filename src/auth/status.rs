#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthState {
    Missing,
    Valid,
    Expired,
}

#[derive(Debug, Clone)]
pub struct AuthCheck {
    pub host: String,
    pub state: AuthState,
}

impl AuthCheck {
    pub fn missing(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            state: AuthState::Missing,
        }
    }

    pub fn valid(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            state: AuthState::Valid,
        }
    }

    pub fn expired(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            state: AuthState::Expired,
        }
    }
}
