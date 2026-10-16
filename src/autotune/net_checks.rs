use std::collections::HashMap;
use std::io::{self, ErrorKind, Read};
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::Mutex;
use std::time::Duration;

use super::quic;
use super::types::{BlockChecks, CheckResult};

const TIMEOUT: Duration = Duration::from_secs(4);

pub const TEST_DOMAINS: &[&str] = &["discord.com", "youtube.com", "cdn.discordapp.com"];
pub const CLEAN_DOMAIN: &str = "google.com";

pub const KNOWN_IPS: &[(&str, &[&str])] = &[
    (
        "discord.com",
        &["162.159.128.233", "162.159.135.232", "162.159.136.232"],
    ),
    ("youtube.com", &["142.250.150.46", "216.58.209.46", "142.250.185.78"]),
    ("google.com", &["142.250.185.78", "216.58.215.14"]),
];

static DNS_CACHE: Mutex<Option<HashMap<String, Vec<IpAddr>>>> = Mutex::new(None);

pub fn resolve_domain(domain: &str) -> Vec<IpAddr> {
    if let Ok(mut guard) = DNS_CACHE.lock() {
        let cache = guard.get_or_insert_with(HashMap::new);
        if let Some(ips) = cache.get(domain) {
            return ips.clone();
        }
        let addrs: Vec<IpAddr> = (domain, 0)
            .to_socket_addrs()
            .map(|addrs| addrs.map(|a| a.ip()).collect())
            .unwrap_or_default();
        if !addrs.is_empty() {
            cache.insert(domain.to_string(), addrs.clone());
        }
        addrs
    } else {
        (domain, 0)
            .to_socket_addrs()
            .map(|addrs| addrs.map(|a| a.ip()).collect())
            .unwrap_or_default()
    }
}

fn is_sinkhole(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_unspecified() || v4.is_loopback() || v4.is_private(),
        IpAddr::V6(v6) => v6.is_unspecified() || v6.is_loopback(),
    }
}

pub fn try_tcp_connect(addr: &str, port: u16) -> Result<TcpStream, io::Error> {
    let socket_addr: SocketAddr = format!("{}:{}", addr, port)
        .parse()
        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "invalid address"))?;
    TcpStream::connect_timeout(&socket_addr, TIMEOUT)
}

pub fn try_tcp_connect_domain(domain: &str, port: u16) -> Result<TcpStream, io::Error> {
    let addrs = (domain, port).to_socket_addrs()?;
    let mut last_err = io::Error::other("no addresses");
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, TIMEOUT) {
            Ok(stream) => return Ok(stream),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

pub fn check_dns_spoof() -> CheckResult {
    let mut problems = 0;

    for &domain in TEST_DOMAINS {
        let sys_ips = resolve_domain(domain);
        if sys_ips.is_empty() {
            problems += 1;
            continue;
        }

        let suspect: Vec<IpAddr> = sys_ips.iter().copied().filter(|&ip| is_sinkhole(ip)).collect();
        if !suspect.is_empty() {
            return CheckResult::fail();
        }

        if let Some(&(_, known_ips)) = KNOWN_IPS.iter().find(|(d, _)| *d == domain) {
            let known_addrs: Vec<IpAddr> = known_ips.iter().filter_map(|s| s.parse().ok()).collect();
            let any_match = sys_ips.iter().any(|ip| known_addrs.contains(ip));
            if !any_match {
                problems += 1;
            }
        }
    }

    if resolve_domain(CLEAN_DOMAIN).is_empty() {
        return CheckResult::skip();
    }

    if problems == 0 {
        CheckResult::pass()
    } else {
        CheckResult::fail()
    }
}

pub fn check_tcp_rst() -> CheckResult {
    let mut domain_success = 0;
    let mut domain_fail_rst = 0;

    for &domain in TEST_DOMAINS {
        match try_tcp_connect_domain(domain, 443) {
            Ok(mut stream) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let mut buf = [0u8; 1];
                match stream.read_exact(&mut buf) {
                    Ok(_) => domain_success += 1,
                    Err(ref e) if e.kind() == ErrorKind::ConnectionReset => domain_fail_rst += 1,
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                        domain_success += 1
                    }
                    Err(_) => {}
                }
            }
            Err(ref e) if e.kind() == ErrorKind::ConnectionReset => domain_fail_rst += 1,
            Err(ref e) if e.kind() == ErrorKind::TimedOut => domain_fail_rst += 1,
            Err(_) => {}
        }
    }

    if try_tcp_connect_domain(CLEAN_DOMAIN, 443).is_err() {
        return CheckResult::skip();
    }

    if domain_success > 0 && domain_fail_rst == 0 {
        CheckResult::pass()
    } else if domain_fail_rst > 0 {
        CheckResult::fail()
    } else {
        CheckResult::skip()
    }
}

pub fn check_sni_block() -> CheckResult {
    let mut ip_ok = 0;
    let mut domain_fail = 0;
    let mut ip_fail = 0;

    for &(domain, ips) in KNOWN_IPS {
        if domain == CLEAN_DOMAIN {
            continue;
        }

        let domain_ok = try_tcp_connect_domain(domain, 443).is_ok();
        if !domain_ok {
            domain_fail += 1;
        }

        for &ip in ips {
            match try_tcp_connect(ip, 443) {
                Ok(mut stream) => {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                    let mut buf = [0u8; 1];
                    match stream.read(&mut buf) {
                        Ok(_) => ip_ok += 1,
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                            ip_ok += 1
                        }
                        Err(ref e) if e.kind() == ErrorKind::ConnectionReset => ip_fail += 1,
                        Err(_) => ip_ok += 1,
                    }
                }
                Err(ref e) if e.kind() == ErrorKind::ConnectionReset => ip_fail += 1,
                Err(ref e) if e.kind() == ErrorKind::TimedOut => ip_fail += 1,
                Err(_) => {}
            }
        }
    }

    if try_tcp_connect_domain(CLEAN_DOMAIN, 443).is_err() {
        return CheckResult::skip();
    }

    if domain_fail > 0 && ip_ok > ip_fail {
        CheckResult::fail()
    } else if domain_fail == 0 {
        CheckResult::pass()
    } else {
        CheckResult::skip()
    }
}

pub fn check_siberian_block() -> CheckResult {
    const MAX_CONCURRENT: usize = 15;
    const EXTRA_CONNECTIONS: usize = 10;

    let test_ips: Vec<&str> = KNOWN_IPS[0].1.to_vec();

    if try_tcp_connect_domain(CLEAN_DOMAIN, 443).is_err() {
        return CheckResult::skip();
    }

    let mut handles: Vec<std::thread::JoinHandle<Result<TcpStream, io::Error>>> = Vec::new();

    for _ in 0..MAX_CONCURRENT {
        for &ip in &test_ips {
            let handle = std::thread::spawn(move || try_tcp_connect(ip, 443));
            handles.push(handle);
        }
    }

    let mut alive = 0;
    let mut failed = 0;

    for handle in handles {
        match handle.join() {
            Ok(Ok(_)) => alive += 1,
            Ok(Err(_)) => failed += 1,
            Err(_) => failed += 1,
        }
    }

    let mut extra_failed = 0;

    for _ in 0..EXTRA_CONNECTIONS {
        let ok = test_ips.iter().any(|&ip| try_tcp_connect(ip, 443).is_ok());
        if ok {
            alive += 1;
        } else {
            extra_failed += 1;
            failed += 1;
        }
    }

    let total_attempted = alive + failed;
    let pass_ratio = if total_attempted > 0 {
        alive as f64 / total_attempted as f64
    } else {
        1.0
    };

    if extra_failed == 0 && pass_ratio > 0.95 {
        CheckResult::pass()
    } else if extra_failed > 0 || pass_ratio < 0.8 {
        CheckResult::fail()
    } else {
        CheckResult::skip()
    }
}

pub fn check_quic_block() -> CheckResult {
    match UdpSocket::bind("0.0.0.0:0") {
        Ok(sock) => {
            let clean_ip: IpAddr = "8.8.8.8".parse().unwrap();
            if sock.connect((clean_ip, 53)).is_err() {
                return CheckResult::skip();
            }
        }
        Err(_) => {
            return CheckResult::skip();
        }
    }

    let mut quic_ok = 0;

    for &(domain, ips) in KNOWN_IPS {
        if domain == CLEAN_DOMAIN {
            continue;
        }
        for &ip_str in ips {
            let ip: IpAddr = match ip_str.parse() {
                Ok(ip) => ip,
                Err(_) => continue,
            };
            let addr = SocketAddr::new(ip, 443);
            let Ok(sock) = UdpSocket::bind("0.0.0.0:0") else {
                continue;
            };
            if sock.connect(addr).is_err() {
                continue;
            }
            if sock.set_read_timeout(Some(Duration::from_secs(2))).is_err() {
                continue;
            }
            match quic::send_probe(&sock, domain) {
                quic::ProbeOutcome::Reply => quic_ok += 1,
                quic::ProbeOutcome::NoReply | quic::ProbeOutcome::Error => {}
            }
        }
    }

    if quic_ok > 0 {
        CheckResult::pass()
    } else {
        CheckResult::fail()
    }
}

pub fn check_cidr_whitelist() -> CheckResult {
    let test_ips = ["1.1.1.1", "8.8.8.8", "77.88.8.8", "185.178.208.97", "104.16.0.0"];

    let mut blocked = 0;

    for ip in test_ips {
        match try_tcp_connect(ip, 443) {
            Ok(_) => {}
            Err(ref e) if e.kind() == ErrorKind::ConnectionReset || e.kind() == ErrorKind::TimedOut => blocked += 1,
            Err(_) => {}
        }
    }

    if try_tcp_connect_domain(CLEAN_DOMAIN, 443).is_err() {
        return CheckResult::skip();
    }

    if blocked == 0 {
        CheckResult::pass()
    } else {
        CheckResult::fail()
    }
}

pub fn run_network_checks(block_checks: &BlockChecks) -> Vec<CheckResult> {
    let checks: [fn() -> CheckResult; 6] = [
        check_dns_spoof,
        check_tcp_rst,
        check_sni_block,
        check_siberian_block,
        check_quic_block,
        check_cidr_whitelist,
    ];
    let mut handles: Vec<(usize, std::thread::JoinHandle<CheckResult>)> = Vec::new();
    for (i, &check) in checks.iter().enumerate() {
        if block_checks.get(i) {
            handles.push((i, std::thread::spawn(check)));
        }
    }
    let mut results: Vec<Option<CheckResult>> = vec![None; checks.len()];
    for (i, handle) in handles {
        results[i] = Some(handle.join().unwrap_or_else(|_| CheckResult::skip()));
    }
    results
        .into_iter()
        .map(|r| r.unwrap_or_else(|| CheckResult::skip()))
        .collect()
}
