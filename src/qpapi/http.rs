use anyhow::{bail, ensure, Context, Result};
use reqwest::blocking::Client;
use reqwest::Certificate;
use std::{env, ffi::OsStr, path::Path, time::Duration};

pub(super) fn client(timeout: Duration) -> Result<Client> {
    let file = env::var_os("SSL_CERT_FILE");
    let dirs = env::var_os("SSL_CERT_DIR");
    let certificates = ca_certificates(file.as_deref().map(Path::new), dirs.as_deref())?;

    Client::builder()
        .timeout(timeout)
        .tls_certs_merge(certificates)
        .build()
        .context("build HTTP client")
}

fn ca_certificates(file: Option<&Path>, dirs: Option<&OsStr>) -> Result<Vec<Certificate>> {
    let mut certificates = Vec::new();
    if let Some(file) = file {
        certificates.extend(
            load_certificates(Some(file), None)
                .with_context(|| format!("load SSL_CERT_FILE {}", file.display()))?,
        );
    }
    if let Some(dirs) = dirs {
        for dir in env::split_paths(dirs).filter(|path| !path.as_os_str().is_empty()) {
            certificates.extend(
                load_certificates(None, Some(&dir))
                    .with_context(|| format!("load SSL_CERT_DIR {}", dir.display()))?,
            );
        }
    }
    Ok(certificates)
}

fn load_certificates(file: Option<&Path>, dir: Option<&Path>) -> Result<Vec<Certificate>> {
    let result = rustls_native_certs::load_certs_from_paths(file, dir);
    if let Some(error) = result.errors.first() {
        bail!("{error}");
    }
    ensure!(
        file.is_none() || !result.certs.is_empty(),
        "CA bundle contains no certificates"
    );
    result
        .certs
        .into_iter()
        .map(|certificate| {
            Certificate::from_der(certificate.as_ref()).context("parse CA certificate")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    const CA_BUNDLE: &str = include_str!("../../tests/fixtures/ca-bundle.pem");

    #[test]
    fn no_overrides_leaves_platform_roots_unchanged() -> Result<()> {
        assert!(ca_certificates(None, None)?.is_empty());
        Ok(())
    }

    #[test]
    fn loads_every_certificate_in_a_bundle() -> Result<()> {
        let temp = tempdir()?;
        let file = temp.path().join("ca.pem");
        fs::write(&file, CA_BUNDLE)?;

        assert_eq!(ca_certificates(Some(&file), None)?.len(), 2);
        Ok(())
    }

    #[test]
    fn loads_certificates_from_multiple_directories() -> Result<()> {
        let temp = tempdir()?;
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        fs::create_dir(&first)?;
        fs::create_dir(&second)?;
        fs::write(first.join("ca.pem"), CA_BUNDLE)?;
        fs::write(second.join("ca.pem"), CA_BUNDLE)?;
        let dirs = env::join_paths([&first, &second])?;

        assert_eq!(ca_certificates(None, Some(&dirs))?.len(), 4);
        Ok(())
    }

    #[test]
    fn loads_both_file_and_directory_overrides() -> Result<()> {
        let temp = tempdir()?;
        let file = temp.path().join("ca.pem");
        let dir = temp.path().join("certs");
        fs::write(&file, CA_BUNDLE)?;
        fs::create_dir(&dir)?;
        fs::write(dir.join("ca.pem"), CA_BUNDLE)?;

        assert_eq!(
            ca_certificates(Some(&file), Some(dir.as_os_str()))?.len(),
            4
        );
        Ok(())
    }

    #[test]
    fn ignores_empty_directory_entries() -> Result<()> {
        let temp = tempdir()?;
        fs::write(temp.path().join("ca.pem"), CA_BUNDLE)?;
        let dirs = env::join_paths([Path::new(""), temp.path(), Path::new("")])?;

        assert_eq!(ca_certificates(None, Some(&dirs))?.len(), 2);
        assert!(ca_certificates(None, Some(OsStr::new("")))?.is_empty());
        Ok(())
    }

    #[test]
    fn rejects_missing_bundle_with_environment_variable_and_path() -> Result<()> {
        let temp = tempdir()?;
        let file = temp.path().join("missing.pem");
        let error = ca_certificates(Some(&file), None).unwrap_err();
        let message = format!("{error:#}");

        assert!(message.contains("SSL_CERT_FILE"));
        assert!(message.contains("missing.pem"));
        Ok(())
    }

    #[test]
    fn rejects_missing_directory_with_environment_variable_and_path() -> Result<()> {
        let temp = tempdir()?;
        let dir = temp.path().join("missing");
        let error = ca_certificates(None, Some(dir.as_os_str())).unwrap_err();
        let message = format!("{error:#}");

        assert!(message.contains("SSL_CERT_DIR"));
        assert!(message.contains("missing"));
        Ok(())
    }

    #[test]
    fn rejects_bundle_without_certificates() -> Result<()> {
        let temp = tempdir()?;
        let file = temp.path().join("empty.pem");
        fs::write(&file, "not a certificate")?;
        let error = ca_certificates(Some(&file), None).unwrap_err();
        let message = format!("{error:#}");

        assert!(message.contains("SSL_CERT_FILE"));
        assert!(message.contains("no certificates"));
        Ok(())
    }

    #[test]
    fn rejects_malformed_pem_in_bundle() -> Result<()> {
        let temp = tempdir()?;
        let file = temp.path().join("invalid.pem");
        fs::write(
            &file,
            "-----BEGIN CERTIFICATE-----\n!!!\n-----END CERTIFICATE-----\n",
        )?;
        let error = ca_certificates(Some(&file), None).unwrap_err();
        let message = format!("{error:#}");

        assert!(message.contains("SSL_CERT_FILE"));
        assert!(message.contains("invalid.pem"));
        Ok(())
    }
}
