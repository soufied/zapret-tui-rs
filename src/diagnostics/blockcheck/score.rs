
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};

use super::curl::{DomainResult, Kind, Tok};

pub const BASELINE: &str = "__baseline__";
pub const RATING_Z: f64 = 1.0;

#[derive(Debug, Clone, Default)]
pub struct Measure {
    pub score: usize,
    pub max: usize,
    pub http: usize,
    pub tls12: usize,
    pub tls13: usize,
    pub quic: usize,
    pub dom_ok: usize,
    pub dom_total: usize,
    pub avg_ms: u64,
    pub fail_codes: Vec<i32>,
    pub errors: Vec<String>,
}

pub fn aggregate(results: &[DomainResult]) -> Measure {
    let mut m = Measure::default();
    let mut lat_sum: u64 = 0;
    let mut lat_n: u64 = 0;

    for r in results {
        m.dom_total += 1;
        let mut any_ok = false;
        for (kind, tok) in &r.probes {
            m.max += 1;
            match tok {
                Tok::Ok { ms } => {
                    any_ok = true;
                    m.score += 1;
                    lat_sum += *ms;
                    lat_n += 1;
                    match kind {
                        Kind::Http => m.http += 1,
                        Kind::Tls12 => m.tls12 += 1,
                        Kind::Tls13 => m.tls13 += 1,
                        Kind::Quic => m.quic += 1,
                    }
                }
                Tok::Fail { rc, msg } => {
                    m.fail_codes.push(*rc);
                    if !msg.is_empty() {
                        m.errors.push(msg.clone());
                    }
                }
            }
        }
        if any_ok {
            m.dom_ok += 1;
        }
    }

    if lat_n > 0 {
        m.avg_ms = lat_sum / lat_n;
    }
    m
}

pub fn rating(zapret2: bool, m: &Measure) -> u32 {
    if !zapret2 {
        return if m.max == 0 {
            0
        } else {
            (m.score * 1000 / m.max) as u32
        };
    }
    if m.dom_total == 0 {
        return 0;
    }
    let n = m.dom_total as f64;
    let p = m.dom_ok as f64 / n;
    let z = RATING_Z;
    let z2 = z * z;
    let center = (p + z2 / (2.0 * n)) / (1.0 + z2 / n);
    let half = z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt() / (1.0 + z2 / n);
    let lower = (center - half).max(0.0);
    (lower * 1000.0) as u32
}

pub fn top_errors(errors: &[String], n: usize) -> Vec<(usize, String)> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for e in errors {
        *counts.entry(e.as_str()).or_insert(0) += 1;
    }
    let mut v: Vec<(usize, String)> = counts
        .into_iter()
        .map(|(msg, c)| (c, msg.to_string()))
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    v.truncate(n);
    v
}

pub fn code_summary(codes: &[i32]) -> String {
    let mut counts: BTreeMap<i32, usize> = BTreeMap::new();
    for &c in codes {
        *counts.entry(c).or_insert(0) += 1;
    }
    counts
        .iter()
        .map(|(code, n)| format!("{} x{}", code, n))
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Debug, Clone)]
pub struct Record {
    pub name: String,
    pub score: usize,
    pub max: usize,
    pub http: usize,
    pub tls12: usize,
    pub tls13: Option<usize>,
    pub quic: Option<usize>,
    pub avg_ms: u64,
    pub note: String,
    pub dom_ok: usize,
    pub dom_total: usize,
    pub domains: Vec<String>,
    pub rating: u32,
}

impl Record {
    pub fn failed(name: &str, domains: &[String], note: &str) -> Self {
        Record {
            name: name.to_string(),
            score: 0,
            max: 0,
            http: 0,
            tls12: 0,
            tls13: None,
            quic: None,
            avg_ms: 0,
            note: note.to_string(),
            dom_ok: 0,
            dom_total: domains.len(),
            domains: domains.to_vec(),
            rating: 0,
        }
    }

    pub fn to_line(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.name,
            self.score,
            self.max,
            self.http,
            self.tls12,
            opt(self.tls13),
            opt(self.quic),
            self.avg_ms,
            self.note,
            self.dom_ok,
            self.dom_total,
            self.domains.join(","),
            self.rating
        )
    }

    pub fn summary(&self) -> String {
        format!(
            "{}: {}/{} domains available, score {}/{}, rating {}",
            self.name, self.dom_ok, self.dom_total, self.score, self.max, self.rating
        )
    }
}

fn opt(v: Option<usize>) -> String {
    v.map_or_else(|| "n/a".to_string(), |x| x.to_string())
}

pub fn sort_ranked(records: &mut [Record]) {
    records.sort_by_key(|r| {
        (
            Reverse(r.rating),
            !r.note.is_empty(),
            Reverse(r.dom_ok),
            r.avg_ms,
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn domain(probes: Vec<(Kind, Tok)>) -> DomainResult {
        DomainResult { probes }
    }

    fn ok(ms: u64) -> Tok {
        Tok::Ok { ms }
    }

    #[test]
    fn zapret1_rating_is_share_of_probes() {
        let results = vec![domain(vec![(Kind::Http, ok(10)), (Kind::Tls12, ok(10))])];
        let m = aggregate(&results);
        assert_eq!(rating(false, &m), 1000);
    }

    #[test]
    fn zapret2_wilson_lower_bound_3_of_3() {
        let m = Measure {
            dom_ok: 3,
            dom_total: 3,
            ..Measure::default()
        };
        assert_eq!(rating(true, &m), 750);
    }

    #[test]
    fn zapret2_rating_zero_without_domains() {
        assert_eq!(rating(true, &Measure::default()), 0);
    }

    #[test]
    fn available_domain_needs_one_ok_probe() {
        let results = vec![
            domain(vec![(Kind::Http, Tok::Fail { rc: 28, msg: "timeout".into() })]),
            domain(vec![(Kind::Tls12, ok(5))]),
        ];
        let m = aggregate(&results);
        assert_eq!(m.dom_ok, 1);
        assert_eq!(m.dom_total, 2);
        assert_eq!(m.fail_codes, vec![28]);
    }

    #[test]
    fn sort_puts_higher_rating_first_and_notes_last() {
        let mut a = Record::failed("a", &[], "engine did not start");
        a.rating = 900;
        let mut b = Record::failed("b", &[], "");
        b.rating = 900;
        b.dom_ok = 2;
        let mut c = Record::failed("c", &[], "");
        c.rating = 950;
        let mut v = vec![a, b, c];
        sort_ranked(&mut v);
        let names: Vec<&str> = v.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["c", "b", "a"]);
    }

    #[test]
    fn line_uses_na_for_untested_probes() {
        let mut r = Record::failed("s", &["a.test".into()], "");
        r.tls13 = None;
        r.quic = Some(0);
        let line = r.to_line();
        assert!(line.starts_with("s|0|0|0|0|n/a|0|0||0|1|a.test|0"), "{}", line);
    }
}
