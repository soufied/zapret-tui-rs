
pub mod curl;
pub mod score;
pub mod targets;

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::common::config::ZapretEngine;
use crate::firewalls::FirewallBackend;

use crate::diagnostics::events;
use curl::{Curl, DomainResult};
use score::{Measure, Record, BASELINE};

pub static CANCEL: AtomicBool = AtomicBool::new(false);

const ENGINE_WAIT: Duration = Duration::from_secs(8);
const BASE_MAX: usize = 32;

pub struct Options {
    pub engine: ZapretEngine,
    pub interface: String,
    pub names: Vec<String>,
    pub limit: usize,
    pub per_list: usize,
    pub services: Vec<String>,
    pub domains: Vec<String>,
}

struct CheckListGuard;

impl Drop for CheckListGuard {
    fn drop(&mut self) {
        crate::engine::runner::set_check_lists(false);
    }
}

pub struct Report {
    pub records: Vec<Record>,
    pub results_file: PathBuf,
}

fn work_dir() -> PathBuf {
    crate::common::config::get_cache_dir().join("blockcheck")
}

fn log_path() -> PathBuf {
    work_dir().join("blockcheck.log")
}

pub fn log(level: &str, msg: &str) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let day = secs % 86_400;
    let line = format!(
        "{:02}:{:02}:{:02} UTC [{}] {}",
        day / 3600,
        (day % 3600) / 60,
        day % 60,
        level,
        msg
    );
    say!("{}", line);
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(log_path()) {
        let _ = writeln!(f, "{}", line);
    }
}

fn cancelled() -> bool {
    CANCEL.load(Ordering::SeqCst)
}

fn engine_running() -> bool {
    crate::engine::runner::nfqws_process_running() || crate::platform::is_nfqws_running()
}

fn wait_engine() -> bool {
    let deadline = Instant::now() + ENGINE_WAIT;
    while Instant::now() < deadline {
        if cancelled() {
            return false;
        }
        if engine_running() {
            std::thread::sleep(Duration::from_millis(500));
            return engine_running();
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    false
}

fn seed_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1)
}

fn select_presets(opts: &Options) -> Result<Vec<String>, String> {
    let all = crate::strategy::get_strategies_for(&opts.engine);
    let mut picked: Vec<String> = if opts.names.is_empty() {
        all.clone()
    } else {
        let mut v: Vec<String> = Vec::new();
        for name in &opts.names {
            if !all.contains(name) {
                return Err(format!("unknown strategy: {}", name));
            }
            v.push(name.clone());
        }
        v
    };
    if opts.limit > 0 {
        picked.truncate(opts.limit);
    }
    if picked.is_empty() {
        return Err(format!(
            "no {} strategies installed; update components first",
            opts.engine
        ));
    }
    Ok(picked)
}

struct Plan {
    name: String,
    domains: Vec<String>,
}

fn build_plans(opts: &Options, presets: &[String], defaults: &[String]) -> Result<Vec<Plan>, String> {
    match opts.engine {
        ZapretEngine::Zapret1 => {
            let domains = if !opts.domains.is_empty() {
                opts.domains.clone()
            } else if !opts.services.is_empty() {
                targets::service_domains(&opts.services)?
            } else {
                defaults.to_vec()
            };
            let path = targets::prepare_check_list(&domains)?;
            log(
                "INFO",
                &format!("test list {} ({} domains)", path.display(), domains.len()),
            );
            Ok(presets
                .iter()
                .map(|name| Plan {
                    name: name.clone(),
                    domains: domains.clone(),
                })
                .collect())
        }
        ZapretEngine::Zapret2 => {
            if !opts.services.is_empty() || !opts.domains.is_empty() {
                log(
                    "WARN",
                    "domain and service selection applies to zapret 1 only; zapret 2 presets test the hosts they name",
                );
            }
            let seed = seed_now();
            let mut plans = Vec::with_capacity(presets.len());
            for (i, name) in presets.iter().enumerate() {
                let t = targets::zapret2_targets(name, opts.per_list, seed.wrapping_add(i as u64));
                for m in &t.missing {
                    log(
                        "WARN",
                        &format!("{}: hostlist not found in zapret2/lists: {}", name, m),
                    );
                }
                let domains = if t.domains.is_empty() {
                    log("INFO", &format!("{}: no hosts named in the preset; testing the default list", name));
                    defaults.to_vec()
                } else {
                    log(
                        "INFO",
                        &format!(
                            "{}: {} random hosts from {} hostlists: {}",
                            name,
                            t.domains.len(),
                            t.lists,
                            t.domains.join(" ")
                        ),
                    );
                    t.domains
                };
                plans.push(Plan {
                    name: name.clone(),
                    domains,
                });
            }
            Ok(plans)
        }
    }
}

fn baseline_domains(plans: &[Plan], defaults: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for plan in plans {
        for d in &plan.domains {
            if !out.contains(d) {
                out.push(d.clone());
            }
        }
    }
    out.truncate(BASE_MAX);
    if out.is_empty() {
        defaults.to_vec()
    } else {
        out
    }
}

fn measure(curl: &Curl, domains: &[String]) -> Measure {
    let results: Vec<DomainResult> = std::thread::scope(|s| {
        let handles: Vec<_> = domains
            .iter()
            .map(|d| s.spawn(move || curl::check_domain(curl, d)))
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    score::aggregate(&results)
}

fn make_record(
    name: &str,
    m: &Measure,
    curl: &Curl,
    engine: &ZapretEngine,
    domains: &[String],
    note: &str,
) -> Record {
    let zapret2 = *engine == ZapretEngine::Zapret2;
    Record {
        name: name.to_string(),
        score: m.score,
        max: m.max,
        http: m.http,
        tls12: m.tls12,
        tls13: if curl.tls13 { Some(m.tls13) } else { None },
        quic: if curl.http3 { Some(m.quic) } else { None },
        avg_ms: m.avg_ms,
        note: note.to_string(),
        dom_ok: m.dom_ok,
        dom_total: m.dom_total,
        domains: domains.to_vec(),
        rating: score::rating(zapret2, m),
    }
}

fn report_errors(name: &str, m: &Measure) {
    if !m.fail_codes.is_empty() {
        log(
            "INFO",
            &format!("{}: failed probes, curl exit codes: {}", name, score::code_summary(&m.fail_codes)),
        );
    }
    for (count, msg) in score::top_errors(&m.errors, 3) {
        log("INFO", &format!("{}: curl said: {} x{}", name, msg, count));
    }
}

pub fn run(
    opts: &Options,
    backend: &dyn FirewallBackend,
    on_progress: &dyn Fn(usize, usize) -> bool,
) -> Result<Report, String> {
    fs::create_dir_all(work_dir())
        .map_err(|e| format!("cannot create {}: {}", work_dir().display(), e))?;
    let _ = fs::write(log_path(), "");

    if engine_running() {
        return Err(
            "zapret is running; stop it before Blockcheck, which needs the engine off for the baseline"
                .to_string(),
        );
    }
    crate::engine::runner::set_check_lists(opts.engine == ZapretEngine::Zapret1);
    let _check_lists = CheckListGuard;

    let curl = curl::detect()?;
    log("INFO", &format!("http client: {}", curl.label()));
    crate::engine::runner::set_engine(opts.engine.clone());

    let presets = select_presets(opts)?;
    let defaults = targets::load_domains(&work_dir().join("domains.txt"));

    log(
        "INFO",
        &format!("run started, engine {}, {} strategies", opts.engine, presets.len()),
    );
    log("INFO", &format!("curl: {}", curl.version));
    log(
        "INFO",
        &format!(
            "curl {}: --tls-max={} TLS1.3-pin={} HTTP/3={}",
            curl.bin, curl.tls_max, curl.tls13, curl.http3
        ),
    );
    if !curl.http3 {
        log(
            "WARN",
            "QUIC is not tested: the selected curl has no HTTP/3. Use a curl built with HTTP/3, or curl-impersonate in Settings.",
        );
    }
    log(
        "INFO",
        "probe codes: h=HTTP, t2=TLS1.2, t3=TLS1.3, q=QUIC; a domain is available when any probe succeeds",
    );
    log(
        "INFO",
        "curl exit codes: 6 DNS, 7 refused, 28 timeout, 35 TLS handshake, 56 connection reset",
    );

    let plans = build_plans(opts, &presets, &defaults)?;
    let base_domains = baseline_domains(&plans, &defaults);
    log("INFO", &format!("baseline hosts: {}", base_domains.join(" ")));

    events::set_header(TABLE_COLUMNS);

    crate::engine::runner::stop_zapret(backend);
    log("INFO", &curl::control(&curl));
    let base_measure = measure(&curl, &base_domains);
    let baseline = make_record(BASELINE, &base_measure, &curl, &opts.engine, &base_domains, "no bypass");
    events::upsert_row(BASELINE, table_cells(&baseline));
    events::set_progress(1, plans.len() + 1);
    report_errors("baseline", &base_measure);
    log(
        "INFO",
        &format!(
            "baseline: {}/{} domains available, score {}/{}",
            baseline.dom_ok, baseline.dom_total, baseline.score, baseline.max
        ),
    );

    let mut records: Vec<Record> = Vec::new();
    let total = plans.len() + 1;
    for (i, plan) in plans.iter().enumerate() {
        if !on_progress(i + 2, total) {
            log("WARN", "cancelled by user");
            break;
        }
        if cancelled() {
            log("WARN", "cancelled; stopping after the last finished strategy");
            break;
        }
        log(
            "INFO",
            &format!("--- {} ({}/{}) hosts: {}", plan.name, i + 1, total, plan.domains.join(" ")),
        );

        let record = match crate::engine::runner::run_zapret_silent(&plan.name, &opts.interface, false, false, backend) {
            Err(e) => {
                log("WARN", &format!("{}: engine did not start: {}", plan.name, e));
                Record::failed(&plan.name, &plan.domains, "engine did not start")
            }
            Ok(()) => {
                if !wait_engine() {
                    log("WARN", &format!("{}: engine exited early", plan.name));
                    Record::failed(&plan.name, &plan.domains, "engine exited early")
                } else {
                    let m = measure(&curl, &plan.domains);
                    report_errors(&plan.name, &m);
                    let r = make_record(&plan.name, &m, &curl, &opts.engine, &plan.domains, "");
                    log("INFO", &r.summary());
                    r
                }
            }
        };
        events::upsert_row(&record.name, table_cells(&record));
        records.push(record);
        crate::engine::runner::stop_zapret(backend);
    }

    crate::engine::runner::stop_zapret(backend);
    events::set_progress(records.len() + 1, total);

    score::sort_ranked(&mut records);
    let best = records
        .iter()
        .find(|r| r.note.is_empty())
        .map(|r| r.name.clone())
        .unwrap_or_else(|| "none".to_string());

    let mut all = Vec::with_capacity(records.len() + 1);
    all.push(baseline);
    all.extend(records);

    let results_file = work_dir().join("results.txt");
    let mut body = String::new();
    for r in &all {
        body.push_str(&r.to_line());
        body.push('\n');
    }
    fs::write(&results_file, body)
        .map_err(|e| format!("cannot write {}: {}", results_file.display(), e))?;

    log("INFO", &format!("finished; best strategy: {}", best));
    Ok(Report {
        records: all,
        results_file,
    })
}

fn na(v: Option<usize>) -> String {
    v.map_or_else(|| "n/a".to_string(), |x| x.to_string())
}

pub const TABLE_COLUMNS: &[&str] = &["Strategy", "Rating", "Domains", "HTTP", "TLS1.2", "TLS1.3", "QUIC", "Avg ms", "Note"];

pub fn table_cells(r: &Record) -> Vec<String> {
    vec![
        r.name.clone(),
        r.rating.to_string(),
        format!("{}/{}", r.dom_ok, r.dom_total),
        r.http.to_string(),
        r.tls12.to_string(),
        na(r.tls13),
        na(r.quic),
        r.avg_ms.to_string(),
        r.note.clone(),
    ]
}

pub fn print_table(records: &[Record]) {
    println!(
        "{:<30} {:>7}  {:>9}  {:>5} {:>5} {:>5} {:>5}  {:>7}  {}",
        "strategy", "rating", "domains", "http", "tls12", "tls13", "quic", "avg ms", "note"
    );
    for r in records {
        println!(
            "{:<30} {:>7}  {:>9}  {:>5} {:>5} {:>5} {:>5}  {:>7}  {}",
            r.name,
            r.rating,
            format!("{}/{}", r.dom_ok, r.dom_total),
            r.http,
            r.tls12,
            na(r.tls13),
            na(r.quic),
            r.avg_ms,
            r.note
        );
    }
}

pub fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(String::from)
        .collect()
}

pub fn run_cli(opts: Options) -> i32 {
    let _ = ctrlc::set_handler(|| CANCEL.store(true, Ordering::SeqCst));

    #[cfg(target_os = "linux")]
    let backend = crate::firewalls::LinuxBackend::Nftables;
    #[cfg(target_os = "windows")]
    let backend = crate::firewalls::windivert::WinDivertBackend;

    match run(&opts, &backend, &|_, _| !CANCEL.load(Ordering::SeqCst)) {
        Ok(report) => {
            println!();
            print_table(&report.records);
            println!();
            log("INFO", &format!("results: {}", report.results_file.display()));
            0
        }
        Err(e) => {
            log("ERROR", &e);
            1
        }
    }
}
