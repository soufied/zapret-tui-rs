
use std::sync::Mutex;

const MAX_LINES: usize = 400;
const MAX_ROWS: usize = 200;

#[derive(Default)]
struct State {
    live: bool,
    done: usize,
    total: usize,
    lines: Vec<String>,
    header: Vec<String>,
    rows: Vec<Row>,
}

static STATE: Mutex<State> = Mutex::new(State {
    live: false,
    done: 0,
    total: 0,
    lines: Vec::new(),
    header: Vec::new(),
    rows: Vec::new(),
});

#[derive(Debug, Clone, Default)]
pub struct Row {
    pub key: String,
    pub cells: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub done: usize,
    pub total: usize,
    pub lines: Vec<String>,
    pub header: Vec<String>,
    pub rows: Vec<Row>,
}

pub fn begin_live() {
    if let Ok(mut s) = STATE.lock() {
        *s = State {
            live: true,
            ..State::default()
        };
    }
}

pub fn end_live() {
    if let Ok(mut s) = STATE.lock() {
        s.live = false;
    }
}

pub fn set_progress(done: usize, total: usize) {
    if let Ok(mut s) = STATE.lock() {
        s.done = done;
        s.total = total;
    }
}

pub fn snapshot() -> Snapshot {
    match STATE.lock() {
        Ok(s) => Snapshot {
            done: s.done,
            total: s.total,
            lines: s.lines.clone(),
            header: s.header.clone(),
            rows: s.rows.clone(),
        },
        Err(_) => Snapshot::default(),
    }
}

pub fn set_header(columns: &[&str]) {
    if let Ok(mut s) = STATE.lock() {
        s.header = columns.iter().map(|c| c.to_string()).collect();
        s.rows.clear();
    }
}

pub fn upsert_row(key: &str, cells: Vec<String>) {
    if let Ok(mut s) = STATE.lock() {
        if let Some(row) = s.rows.iter_mut().find(|r| r.key == key) {
            row.cells = cells;
            return;
        }
        s.rows.push(Row {
            key: key.to_string(),
            cells,
        });
        let excess = s.rows.len().saturating_sub(MAX_ROWS);
        if excess > 0 {
            s.rows.drain(..excess);
        }
    }
}

pub fn say(message: String) {
    let live = STATE.lock().map(|s| s.live).unwrap_or(false);
    if !live {
        println!("{}", message);
        return;
    }
    if let Ok(mut s) = STATE.lock() {
        for line in message.split('\n') {
            s.lines.push(line.to_string());
        }
        let excess = s.lines.len().saturating_sub(MAX_LINES);
        if excess > 0 {
            s.lines.drain(..excess);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn live_buffer_keeps_the_tail() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        begin_live();
        for i in 0..(MAX_LINES + 10) {
            say(format!("line {}", i));
        }
        set_progress(3, 9);
        let snap = snapshot();
        assert_eq!(snap.lines.len(), MAX_LINES);
        assert_eq!(
            snap.lines.last().map(String::as_str),
            Some(format!("line {}", MAX_LINES + 9).as_str())
        );
        assert_eq!((snap.done, snap.total), (3, 9));
        end_live();
    }

    #[test]
    fn rows_are_replaced_by_key() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        begin_live();
        set_header(&["Target", "Result"]);
        upsert_row("a", vec!["x".into()]);
        upsert_row("b", vec!["y".into()]);
        upsert_row("a", vec!["z".into()]);
        let snap = snapshot();
        assert_eq!(snap.header, vec!["Target".to_string(), "Result".to_string()]);
        assert_eq!(snap.rows.len(), 2);
        assert_eq!(snap.rows[0].cells, vec!["z".to_string()]);
        end_live();
    }
}
