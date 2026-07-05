#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Login canceled before a QueryPie session was established")]
    LoginCanceled,
}

pub fn is_login_canceled(err: &anyhow::Error) -> bool {
    matches!(
        err.downcast_ref::<AuthError>(),
        Some(AuthError::LoginCanceled)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_login_canceled_error() {
        // given
        let err = anyhow::Error::new(AuthError::LoginCanceled);

        // when / then
        assert!(is_login_canceled(&err));
    }
}
