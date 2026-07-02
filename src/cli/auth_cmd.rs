use anyhow::{anyhow, bail, Result};

use super::{AuthStatusFailed, Global};
use crate::auth::{self, AuthService};
use crate::formatting::{self, style};
use crate::sessioncache;

pub(super) fn auth_login(global: &Global) -> Result<()> {
    let session = AuthService::new(global.require_host()?)?.login()?;
    println!("logged in to {}", session.host);
    Ok(())
}

pub(super) fn auth_logout(global: &Global) -> Result<()> {
    let host = global.require_host()?.to_string();
    AuthService::new(&host)?.logout()?;
    sessioncache::clear(&global.host, "")?;
    anstream::println!("{} Logged out of {}", style::success_icon(), host);
    Ok(())
}

pub(super) fn auth_status(global: &Global) -> Result<()> {
    let hosts = if global.host.trim().is_empty() {
        auth::known_hosts()
    } else {
        vec![global.host.clone()]
    };
    if hosts.is_empty() {
        bail!("not logged in to any QueryPie host; run `querypie --host <host> auth login`");
    }

    let checks = hosts
        .iter()
        .map(|host| AuthService::new(host)?.check())
        .collect::<Result<Vec<_>>>()?;
    if formatting::auth_status(&checks)? {
        return Err(AuthStatusFailed.into());
    }
    Ok(())
}

pub(super) fn auth_read_cookie(global: &Global) -> Result<()> {
    let cookies = auth::read_cookies_in_process(global.require_host()?)?
        .ok_or_else(|| anyhow!("not logged in"))?;
    println!("{cookies}");
    Ok(())
}

pub(super) fn auth_refresh_cookie(global: &Global) -> Result<()> {
    let cookies = auth::refresh_cookies_in_process(global.require_host()?)?
        .ok_or_else(|| anyhow!("refresh failed"))?;
    println!("{cookies}");
    Ok(())
}
