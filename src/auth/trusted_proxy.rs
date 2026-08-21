//! LINKS-31: gate forwarded IP / country headers on a trusted-proxy peer.
//!
//! Ported from menkent (MKT-33) by way of storefront (SF-109). `X-Forwarded-For`,
//! `X-Real-Ip`, and the LINKS-27 `X-IPCountry` header are only meaningful when
//! the socket peer is the reverse proxy that set them. A client reaching the app
//! directly can forge all three, so believing them unconditionally lets a direct
//! client spoof its IP (evading the per-IP tracing) or its country (suppressing
//! or faking the new-sign-in-location alert).
//!
//! The socket peer is the one non-forgeable input, so it gates everything:
//! [`normalize_forwarded_headers`] runs once per request as the outermost layer,
//! resolves the true client IP from the peer plus the forwarded headers, and
//! rewrites the request headers in place so every downstream reader
//! ([`crate::auth::middleware::client_ip_from_headers`],
//! [`crate::auth::middleware::client_country`]) observes only trusted values.
//! `X-IPCountry` from an untrusted peer is dropped, so no country resolves and
//! no alert fires.
//!
//! Rewriting the headers rather than threading the peer through each accessor is
//! what fits this crate: the readers all take a bare `&HeaderMap` (server
//! functions get theirs from `dioxus_fullstack::extract`, which carries no peer),
//! so a gated accessor would need a signature change at every call site and
//! would silently keep trusting any site that was missed.
//!
//! Trust is configured through `TRUSTED_PROXY_CIDRS` (comma-separated CIDRs or
//! bare IPs, see [`crate::config::Config::trusted_proxy_cidrs`]). Empty (the
//! default) means no peer is trusted and every forwarded header is ignored in
//! favor of the socket peer, which is the safe default for local dev. Deployed
//! stacks behind Traefik must set the private ingress ranges.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::extract::{ConnectInfo, Request};
use axum::http::header::HeaderName;
use axum::http::{HeaderMap, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use ipnetwork::IpNetwork;

/// Parse a comma-separated list of CIDRs or bare IPs. Unparseable entries are
/// warned about and dropped rather than failing boot.
pub fn parse_trusted_proxy_cidrs(raw: &str) -> Vec<IpNetwork> {
    raw.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| match entry.parse::<IpNetwork>() {
            Ok(net) => Some(net),
            Err(error) => {
                tracing::warn!(
                    entry = %entry,
                    error = %error,
                    "Ignoring invalid TRUSTED_PROXY_CIDRS entry"
                );
                None
            }
        })
        .collect()
}

fn is_trusted(ip: IpAddr, trusted: &[IpNetwork]) -> bool {
    trusted.iter().any(|net| net.contains(ip))
}

/// Resolve the client IP from the socket peer plus the forwarded headers.
///
/// The peer address gates everything: a forwarded header is read only when the
/// peer itself sits in `trusted`. With no trusted proxies configured a forged
/// `X-Forwarded-For` / `X-Real-Ip` is ignored entirely. Inside a proxy chain the
/// rightmost entry not belonging to a trusted proxy is the client, since
/// anything further left was supplied by the client and can be forged.
pub fn resolve_client_ip(
    peer: Option<IpAddr>,
    headers: &HeaderMap,
    trusted: &[IpNetwork],
) -> Option<IpAddr> {
    let peer = peer?;

    if !is_trusted(peer, trusted) {
        return Some(peer);
    }

    if let Some(forwarded) = headers.get("X-Forwarded-For").and_then(|v| v.to_str().ok()) {
        let client = forwarded
            .split(',')
            .filter_map(|entry| entry.trim().parse::<IpAddr>().ok())
            .rev()
            .find(|ip| !is_trusted(*ip, trusted));
        if let Some(ip) = client {
            return Some(ip);
        }
    }

    if let Some(ip) = headers
        .get("X-Real-Ip")
        .and_then(|v| v.to_str().ok())
        .and_then(|value| value.trim().parse().ok())
    {
        return Some(ip);
    }

    Some(peer)
}

/// Rewrite the forwarded headers in place so downstream readers see only
/// peer-gated values: `X-Forwarded-For` is collapsed to the single resolved
/// client IP (or removed when none resolves), `X-Real-Ip` is dropped, and
/// `X-IPCountry` is dropped unless the peer is a trusted proxy.
fn apply_normalization(headers: &mut HeaderMap, peer: Option<IpAddr>, trusted: &[IpNetwork]) {
    let resolved = resolve_client_ip(peer, headers, trusted);
    let country_trusted = peer.is_some_and(|ip| is_trusted(ip, trusted));

    headers.remove("x-forwarded-for");
    headers.remove("x-real-ip");
    if let Some(ip) = resolved {
        if let Ok(value) = HeaderValue::from_str(&ip.to_string()) {
            headers.insert(HeaderName::from_static("x-forwarded-for"), value);
        }
    }
    if !country_trusted {
        headers.remove("x-ipcountry");
    }
}

/// Middleware: gate the forwarded IP / country headers on the socket peer before
/// any handler reads them. The peer comes from axum `ConnectInfo`, injected by
/// `into_make_service_with_connect_info`; a request without it (no connect info)
/// is treated as having no peer, so nothing is trusted.
pub async fn normalize_forwarded_headers(
    trusted: Arc<Vec<IpNetwork>>,
    mut req: Request,
    next: Next,
) -> Response {
    let peer = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect_info| connect_info.0.ip());
    apply_normalization(req.headers_mut(), peer, &trusted);
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::middleware::{client_country, client_ip_from_headers};

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        map
    }

    fn ip(value: &str) -> IpAddr {
        value.parse().unwrap()
    }

    fn cidrs(list: &[&str]) -> Vec<IpNetwork> {
        list.iter().map(|entry| entry.parse().unwrap()).collect()
    }

    // With no trusted proxies, forged forwarded headers must not move the
    // resolved IP off the socket peer.
    #[test]
    fn resolve_ignores_forwarded_headers_without_trusted_proxies() {
        let map = headers(&[
            ("X-Forwarded-For", "9.9.9.9, 8.8.8.8"),
            ("X-Real-Ip", "7.7.7.7"),
        ]);
        assert_eq!(
            resolve_client_ip(Some(ip("203.0.113.5")), &map, &[]),
            Some(ip("203.0.113.5"))
        );
    }

    #[test]
    fn resolve_ignores_forwarded_headers_from_untrusted_peer() {
        let map = headers(&[("X-Forwarded-For", "9.9.9.9")]);
        assert_eq!(
            resolve_client_ip(Some(ip("203.0.113.5")), &map, &cidrs(&["10.0.0.0/8"])),
            Some(ip("203.0.113.5"))
        );
    }

    #[test]
    fn resolve_honors_forwarded_for_from_trusted_proxy() {
        let map = headers(&[("X-Forwarded-For", "203.0.113.5")]);
        assert_eq!(
            resolve_client_ip(Some(ip("10.1.2.3")), &map, &cidrs(&["10.0.0.0/8"])),
            Some(ip("203.0.113.5"))
        );
    }

    // A client that prepends its own XFF entries cannot hide behind them: the
    // rightmost untrusted entry is the one the trusted proxy observed.
    #[test]
    fn resolve_takes_rightmost_untrusted_entry() {
        let map = headers(&[("X-Forwarded-For", "1.2.3.4, 203.0.113.5, 10.9.9.9")]);
        assert_eq!(
            resolve_client_ip(Some(ip("10.1.2.3")), &map, &cidrs(&["10.0.0.0/8"])),
            Some(ip("203.0.113.5"))
        );
    }

    #[test]
    fn resolve_honors_real_ip_from_trusted_proxy() {
        let map = headers(&[("X-Real-Ip", "203.0.113.5")]);
        assert_eq!(
            resolve_client_ip(Some(ip("10.1.2.3")), &map, &cidrs(&["10.0.0.0/8"])),
            Some(ip("203.0.113.5"))
        );
    }

    // All XFF entries trusted and no X-Real-Ip: nothing untrusted was ever
    // observed, so fall back to the peer rather than inventing a client.
    #[test]
    fn resolve_falls_back_to_peer_when_all_entries_trusted() {
        let map = headers(&[("X-Forwarded-For", "10.9.9.9, 10.8.8.8")]);
        assert_eq!(
            resolve_client_ip(Some(ip("10.1.2.3")), &map, &cidrs(&["10.0.0.0/8"])),
            Some(ip("10.1.2.3"))
        );
    }

    #[test]
    fn resolve_is_none_without_a_peer() {
        let map = headers(&[("X-Forwarded-For", "203.0.113.5")]);
        assert_eq!(resolve_client_ip(None, &map, &cidrs(&["10.0.0.0/8"])), None);
    }

    #[test]
    fn normalization_collapses_forwarded_for_and_drops_real_ip() {
        let mut map = headers(&[
            ("X-Forwarded-For", "1.2.3.4, 203.0.113.5, 10.9.9.9"),
            ("X-Real-Ip", "7.7.7.7"),
        ]);
        apply_normalization(&mut map, Some(ip("10.1.2.3")), &cidrs(&["10.0.0.0/8"]));
        assert_eq!(
            client_ip_from_headers(&map),
            Some("203.0.113.5".to_string())
        );
        assert!(map.get("x-real-ip").is_none());
    }

    // Country is honored only from a trusted peer: an untrusted peer's
    // X-IPCountry is stripped, so client_country resolves to None and the
    // LINKS-27 alert can be neither faked nor silenced.
    #[test]
    fn normalization_strips_country_from_untrusted_peer() {
        let mut map = headers(&[("X-Forwarded-For", "9.9.9.9"), ("X-IPCountry", "DE")]);
        apply_normalization(&mut map, Some(ip("203.0.113.5")), &cidrs(&["10.0.0.0/8"]));
        assert_eq!(client_country(&map), None);
        // XFF collapsed to the socket peer, so the forged 9.9.9.9 is gone.
        assert_eq!(
            client_ip_from_headers(&map),
            Some("203.0.113.5".to_string())
        );
    }

    #[test]
    fn normalization_keeps_country_from_trusted_peer() {
        let mut map = headers(&[("X-Forwarded-For", "203.0.113.5"), ("X-IPCountry", "DE")]);
        apply_normalization(&mut map, Some(ip("10.1.2.3")), &cidrs(&["10.0.0.0/8"]));
        assert_eq!(client_country(&map), Some("DE".to_string()));
        assert_eq!(
            client_ip_from_headers(&map),
            Some("203.0.113.5".to_string())
        );
    }

    // No trusted CIDRs configured is the shipped default: the country header
    // must be ignored even when the peer is the reverse proxy.
    #[test]
    fn normalization_strips_country_without_trusted_proxies() {
        let mut map = headers(&[("X-IPCountry", "DE")]);
        apply_normalization(&mut map, Some(ip("10.1.2.3")), &[]);
        assert_eq!(client_country(&map), None);
    }

    // No connect info (peer None): nothing is trusted, so a present X-IPCountry
    // is stripped and no forwarded IP survives.
    #[test]
    fn normalization_without_a_peer_trusts_nothing() {
        let mut map = headers(&[("X-IPCountry", "DE"), ("X-Forwarded-For", "9.9.9.9")]);
        apply_normalization(&mut map, None, &cidrs(&["10.0.0.0/8"]));
        assert_eq!(client_country(&map), None);
        assert_eq!(client_ip_from_headers(&map), None);
    }

    #[test]
    fn parse_drops_invalid_entries_and_keeps_valid_ones() {
        assert!(parse_trusted_proxy_cidrs("").is_empty());
        assert!(parse_trusted_proxy_cidrs("not-an-ip, 10.0.0.0/99").is_empty());
        let parsed = parse_trusted_proxy_cidrs(" 10.0.0.0/8 , not-an-ip, 192.0.2.7 ");
        assert_eq!(parsed.len(), 2);
        assert!(parsed.iter().any(|net| net.contains(ip("10.1.2.3"))));
        assert!(parsed.iter().any(|net| net.contains(ip("192.0.2.7"))));
    }

    // The deployment default must cover Traefik on any private Docker network,
    // v4 and v6.
    #[test]
    fn parse_accepts_the_documented_private_ranges() {
        let parsed = parse_trusted_proxy_cidrs("10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,fd00::/8");
        assert_eq!(parsed.len(), 4);
        assert!(parsed.iter().any(|net| net.contains(ip("172.18.0.5"))));
        assert!(parsed.iter().any(|net| net.contains(ip("fd00::1"))));
    }
}
