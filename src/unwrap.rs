//! Best-effort resolution of known email "safe link" wrappers (Mimecast,
//! Microsoft SafeLinks, Proofpoint, Barracuda...) to the real destination.
//!
//! Opt-in (see `crate::settings`) and scoped to a fixed allowlist: this never
//! makes a network request for an ordinary link, only for one that is already
//! hosted on a known wrapper domain. Resolving is itself a "click" as far as
//! the sender's tracker is concerned, which is why it isn't automatic.

use std::time::Duration;

/// Suffixes matched against the URL's host, case-insensitively. A host
/// matches if it equals the suffix or ends with "." + suffix.
const WRAPPER_SUFFIXES: &[&str] = &[
    "mimecastprotect.com",
    "safelinks.protection.outlook.com",
    "urldefense.com",
    "urldefense.proofpoint.com",
    "linkprotect.cudasvc.com",
    "clicktime.symantec.com",
];

const MAX_HOPS: u32 = 4;
const TIMEOUT: Duration = Duration::from_millis(1500);

fn host_matches_suffix(host: &str, suffix: &str) -> bool {
    host == suffix || host.ends_with(&format!(".{suffix}"))
}

/// Whether this host is a known safe-link wrapper worth resolving.
pub fn is_wrapped_host(host: &str) -> bool {
    let host = host.to_lowercase();
    WRAPPER_SUFFIXES
        .iter()
        .any(|suffix| host_matches_suffix(&host, suffix))
}

/// Follow redirects while the URL stays on a wrapper host, stopping as soon
/// as it leaves the allowlist (the real destination) or the hop/time budget
/// runs out. Returns `None` if nothing was learned beyond the original URL.
///
/// Only ever a GET with the response body left unread, so the destination
/// page itself is never downloaded or rendered - this looks like a link
/// preview, not a visit.
pub fn resolve(url: &str) -> Option<String> {
    // Use the OS trust store (schannel, via native-tls) rather than a
    // bundled CA list: browsers already trust whatever root a corporate
    // network installs for TLS inspection, and this should see the same
    // link a click in the browser would.
    let tls = native_tls::TlsConnector::new().ok()?;
    let agent = ureq::AgentBuilder::new()
        .tls_connector(std::sync::Arc::new(tls))
        .timeout_connect(TIMEOUT)
        .timeout(TIMEOUT)
        .redirects(0)
        .user_agent("Mozilla/5.0")
        .build();

    let mut current = url.to_string();
    for _ in 0..MAX_HOPS {
        let host = crate::rules::domain_of(&current);
        if !is_wrapped_host(&host) {
            break;
        }
        let resp = match agent.get(&current).call() {
            Ok(r) | Err(ureq::Error::Status(_, r)) => r,
            Err(_) => return None,
        };
        let status = resp.status();
        if !(300..400).contains(&status) {
            break;
        }
        let Some(location) = resp.header("Location") else {
            break;
        };
        if !location.starts_with("http://") && !location.starts_with("https://") {
            break;
        }
        current = location.to_string();
    }

    if current != url { Some(current) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_wrapper_hosts() {
        assert!(is_wrapped_host("url.uk.m.mimecastprotect.com"));
        assert!(is_wrapped_host("MIMECASTPROTECT.com"));
        assert!(is_wrapped_host("eur01.safelinks.protection.outlook.com"));
        assert!(!is_wrapped_host("mimecastprotect.com.evil.net"));
        assert!(!is_wrapped_host("click.mail.axahealth.co.uk"));
        assert!(!is_wrapped_host("github.com"));
    }

    #[test]
    #[ignore = "hits the real network"]
    fn resolves_a_live_mimecast_link() {
        let url = "https://url.uk.m.mimecastprotect.com/s/JLznCqly3cJLE7qU2cKKiEGLhN?domain=click.mail.axahealth.co.uk";
        let resolved = resolve(url).expect("should resolve to something");
        println!("resolved: {resolved}");
        assert!(resolved.contains("axahealth.co.uk"));
    }
}
