
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};
use std::sync::Mutex;
use std::time::Duration;

use crate::common::logger;

const DOH_ENDPOINTS: &[&str] = &["https://1.1.1.1/dns-query", "https://dns.google/resolve"];

const DOH_TIMEOUT: Duration = Duration::from_secs(4);

const TYPE_A: u64 = 1;

static CLEAN_CACHE: Mutex<Option<HashMap<String, Vec<IpAddr>>>> = Mutex::new(None);

pub fn is_sinkhole(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_unspecified() || v4.is_loopback() || v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_unspecified() || v6.is_loopback(),
    }
}

pub fn pinned_v4(domain: &str) -> Option<Ipv4Addr> {
    resolve(domain).into_iter().find_map(|ip| match ip {
        IpAddr::V4(v4) => Some(v4),
        IpAddr::V6(_) => None,
    })
}

pub fn resolve(domain: &str) -> Vec<IpAddr> {
    let clean = lookup_clean(domain);
    if !clean.is_empty() {
        return clean;
    }
    system_answer(domain)
        .into_iter()
        .filter(|ip| !is_sinkhole(*ip))
        .collect()
}

pub fn system_answer(domain: &str) -> Vec<IpAddr> {
    let mut ips: Vec<IpAddr> = (domain, 0)
        .to_socket_addrs()
        .map(|addrs| addrs.map(|a| a.ip()).collect())
        .unwrap_or_default();
    ips.sort();
    ips.dedup();
    ips
}

pub fn lookup_clean(domain: &str) -> Vec<IpAddr> {
    if let Ok(guard) = CLEAN_CACHE.lock() {
        if let Some(hit) = guard.as_ref().and_then(|cache| cache.get(domain)) {
            return hit.clone();
        }
    }

    let mut answer = Vec::new();
    for endpoint in DOH_ENDPOINTS {
        match doh_query(endpoint, domain) {
            Ok(ips) if !ips.is_empty() => {
                answer = ips;
                break;
            }
            Ok(_) => {}
            Err(e) => logger::log_info(&format!("dns: {} via {} failed: {}", domain, endpoint, e)),
        }
    }

    if let Ok(mut guard) = CLEAN_CACHE.lock() {
        guard
            .get_or_insert_with(HashMap::new)
            .insert(domain.to_string(), answer.clone());
    }
    answer
}

fn doh_query(endpoint: &str, domain: &str) -> Result<Vec<IpAddr>, String> {
    let response = ureq::get(endpoint)
        .query("name", domain)
        .query("type", "A")
        .set("Accept", "application/dns-json")
        .timeout(DOH_TIMEOUT)
        .call()
        .map_err(|e| e.to_string())?;

    let text = response.into_string().map_err(|e| e.to_string())?;
    let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    match body["Status"].as_i64() {
        Some(0) => {}
        Some(code) => return Err(format!("DNS status {}", code)),
        None => return Err("no status in DoH answer".to_string()),
    }

    let ips = body["Answer"]
        .as_array()
        .map(|answers| {
            answers
                .iter()
                .filter(|a| a["type"].as_u64() == Some(TYPE_A))
                .filter_map(|a| a["data"].as_str())
                .filter_map(|s| s.parse::<Ipv4Addr>().ok())
                .map(IpAddr::V4)
                .collect()
        })
        .unwrap_or_default();
    Ok(ips)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sinkholes_are_detected() {
        assert!(is_sinkhole("0.0.0.0".parse().unwrap()));
        assert!(is_sinkhole("127.0.0.1".parse().unwrap()));
        assert!(is_sinkhole("192.168.1.1".parse().unwrap()));
        assert!(is_sinkhole("::".parse().unwrap()));
        assert!(!is_sinkhole("162.159.128.233".parse().unwrap()));
        assert!(!is_sinkhole("2606:4700::1111".parse().unwrap()));
    }
}
