//! End-to-end tests against the real `test-game` process: everything a user
//! does in the app, minus the UI.

use addition_engine::ptrscan::{filter_paths, find_paths, PointerScanOptions};
use addition_engine::scan::{ScanControl, ScanFilter, Scanner};
use addition_engine::{open_process, PointerPath, Session, Trainer, Value, ValueType};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

/// `ADDITION_TEST_GAME` points the tests at another build of the game, e.g.
/// a 32-bit one, to check that a 64-bit engine handles 32-bit games.
fn game_exe() -> String {
    std::env::var("ADDITION_TEST_GAME").unwrap_or_else(|_| env!("CARGO_BIN_EXE_test-game").to_string())
}

struct Game {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Game {
    fn start() -> Self {
        let mut child = Command::new(game_exe())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("start test-game");
        let stdin = child.stdin.take().unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        assert!(line.starts_with("ready"), "unexpected banner: {line}");
        Game { child, stdin, stdout }
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }

    fn cmd(&mut self, c: &str) -> serde_json::Value {
        writeln!(self.stdin, "{c}").unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|_| panic!("bad reply to {c}: {line}"))
    }

    fn get(&mut self, field: &str) -> i64 {
        let s = self.cmd("state");
        s[field].as_i64().or_else(|| s[field].as_f64().map(|f| f as i64)).unwrap()
    }

    fn addr(&mut self, field: &str) -> u64 {
        self.cmd("addrs")[field].as_u64().unwrap()
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn trainer() -> Trainer {
    let out = Command::new(game_exe()).arg("--print-trainer").output().unwrap();
    Trainer::from_json(&String::from_utf8(out.stdout).unwrap()).unwrap()
}

fn settle() {
    std::thread::sleep(addition_engine::freezer::INTERVAL * 4);
}

#[test]
fn scan_narrow_and_write() {
    let mut game = Game::start();
    let mem = open_process(game.pid()).unwrap();
    let ctl = ScanControl::default();
    let mut s = Scanner::new(ValueType::I32);

    s.first_scan(&*mem, ScanFilter::Exact("100".into()), &ctl).unwrap();
    game.cmd("hit");
    s.next_scan(&*mem, ScanFilter::Exact("93".into()), &ctl).unwrap();
    game.cmd("hit");
    s.next_scan(&*mem, ScanFilter::Decreased, &ctl).unwrap();

    let health = game.addr("health");
    let hits = s.hits(&*mem, 100);
    assert!(hits.iter().any(|h| h.address == health), "health not among {} hits", hits.len());

    mem.write(health, &ValueType::I32.encode(Value::Int(999))).unwrap();
    assert_eq!(game.get("health"), 999);
}

#[test]
fn unknown_value_scan_finds_gold() {
    let mut game = Game::start();
    let mem = open_process(game.pid()).unwrap();
    let ctl = ScanControl::default();
    let mut s = Scanner::new(ValueType::I64);
    s.first_scan(&*mem, ScanFilter::Unknown, &ctl).unwrap();
    game.cmd("spend 25");
    s.next_scan(&*mem, ScanFilter::Decreased, &ctl).unwrap();
    s.next_scan(&*mem, ScanFilter::Unchanged, &ctl).unwrap();
    game.cmd("spend 5");
    s.next_scan(&*mem, ScanFilter::Exact("470".into()), &ctl).unwrap();
    let gold = game.addr("gold");
    assert!(s.hits(&*mem, 50).iter().any(|h| h.address == gold));
}

#[test]
fn trainer_freeze_set_and_patch() {
    let mut game = Game::start();
    let mut session = Session::attach(trainer(), game.pid()).unwrap();

    // Freeze: health snaps back after damage, and keeps working after the
    // player object moves to a new heap address.
    session.enable("health", None).unwrap();
    game.cmd("hit");
    settle();
    assert_eq!(game.get("health"), 100);
    game.cmd("respawn");
    game.cmd("hit");
    settle();
    assert_eq!(game.get("health"), 100);
    session.disable("health").unwrap();
    game.cmd("hit");
    settle();
    assert_eq!(game.get("health"), 93);

    // Set: one-shot write, plus the hotkey default.
    session.set_value("gold", Value::Int(4242)).unwrap();
    assert_eq!(game.get("gold"), 4242);
    session.hotkey("gold").unwrap();
    assert_eq!(game.get("gold"), 99999);
    assert!(session.set_value("gold", Value::Int(-1)).is_err(), "range check");
    session.set_value("speed", Value::Float(2.5)).unwrap();
    assert_eq!(game.cmd("state")["speed"].as_f64(), Some(2.5));

    // Patch: writes into read-only memory, then restores the original byte.
    assert!(session.toggle("nodamage").unwrap());
    game.cmd("hit");
    assert_eq!(game.get("health"), 93);
    assert!(!session.toggle("nodamage").unwrap());
    game.cmd("hit");
    assert_eq!(game.get("health"), 86);

    // Detaching restores patches.
    session.enable("nodamage", None).unwrap();
    let states = session.states();
    assert!(states.iter().find(|s| s.id == "nodamage").unwrap().active);
    assert_eq!(states.iter().find(|s| s.id == "gold").unwrap().value, Some(Value::Int(99999)));
    drop(session);
    game.cmd("hit");
    assert_eq!(game.get("health"), 79);
}

#[test]
fn pointer_finder_survives_restart() {
    let mut game = Game::start();
    let mem = open_process(game.pid()).unwrap();
    let health = game.addr("health");
    let opts = PointerScanOptions { max_depth: 3, ..Default::default() };
    let paths = find_paths(&*mem, health, &opts, &ScanControl::default()).unwrap();
    assert!(!paths.is_empty());

    let t = trainer();
    let addition_engine::Action::Freeze { target: expected, .. } = &t.cheat("health").unwrap().action else {
        panic!()
    };
    assert!(paths.contains(expected), "expected {expected} among {} paths", paths.len());

    // A new run puts the player somewhere else; the right chain still works.
    drop(mem);
    drop(game);
    let mut game = Game::start();
    let mem = open_process(game.pid()).unwrap();
    let new_health = game.addr("health");
    let kept = filter_paths(&*mem, &paths, new_health);
    assert!(kept.contains(expected));
    assert!(kept.len() <= paths.len());
}

#[test]
fn static_path_and_dead_process() {
    let game = Game::start();
    let mem = open_process(game.pid()).unwrap();
    assert!(mem.is_alive());
    let p = PointerPath { module: Some("no-such-module".into()), base: 0, offsets: vec![] };
    assert!(p.resolve(&*mem).is_err());
    drop(game);
    std::thread::sleep(Duration::from_millis(100));
    assert!(!mem.is_alive());
}
