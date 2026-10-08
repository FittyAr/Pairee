use super::poller::DirSignature;
use super::strategy::{StrategyKind, strategy_chain};
use super::*;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

fn timing() -> CoalesceTiming {
    CoalesceTiming {
        quiet: Duration::from_millis(250),
        max_wait: Duration::from_secs(2),
    }
}

fn change(dir: &str, entries: &[&str]) -> DirChange {
    DirChange {
        dir: PathBuf::from(dir),
        entries: entries.iter().map(PathBuf::from).collect(),
        origin: WatchOrigin::Local,
    }
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn coalescer_waits_for_the_quiet_period() {
    let t0 = Instant::now();
    let mut c = Coalescer::new(timing());
    c.record(change("/a", &["/a/x"]), t0);
    assert!(c.take_due(t0 + ms(100)).is_empty());
    c.record(change("/a", &["/a/y"]), t0 + ms(200));
    assert!(c.take_due(t0 + ms(400)).is_empty(), "quiet restarts");
    let due = c.take_due(t0 + ms(450));
    assert_eq!(due, vec![change("/a", &["/a/x", "/a/y"])]);
    assert!(c.take_due(t0 + ms(10_000)).is_empty());
}

#[test]
fn coalescer_merges_a_burst_into_one_change() {
    let t0 = Instant::now();
    let mut c = Coalescer::new(timing());
    for i in 0..1000 {
        c.record(change("/a", &["/a/same"]), t0 + ms(i / 10));
    }
    let due = c.take_due(t0 + ms(500));
    assert_eq!(due, vec![change("/a", &["/a/same"])]);
}

#[test]
fn coalescer_flushes_an_endless_burst_after_max_wait() {
    let t0 = Instant::now();
    let mut c = Coalescer::new(timing());
    let mut released = 0;
    // An event every 100 ms for 5 s never goes quiet.
    for step in 0..50 {
        let now = t0 + ms(step * 100);
        c.record(change("/copy", &[]), now);
        released += c.take_due(now).len();
    }
    assert_eq!(released, 2, "one refresh per max_wait window");
}

#[test]
fn coalescer_keeps_folders_apart_and_retains() {
    let t0 = Instant::now();
    let mut c = Coalescer::new(timing());
    c.record(change("/a", &[]), t0);
    c.record(change("/b", &[]), t0);
    c.retain(|_, dir| dir == std::path::Path::new("/b"));
    assert_eq!(c.take_due(t0 + ms(300)), vec![change("/b", &[])]);
}

#[test]
fn strategy_chain_selection() {
    use StrategyKind::{Poll, Watch};
    assert_eq!(strategy_chain(false, false), vec![Watch, Poll]);
    assert_eq!(strategy_chain(true, false), vec![Poll]);
    assert_eq!(strategy_chain(false, true), vec![Poll]);
}

#[test]
fn local_temp_dir_is_watchable() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!needs_polling(dir.path()));
}

#[cfg(target_os = "windows")]
#[test]
fn unc_and_wsl_paths_are_polled() {
    assert!(needs_polling(std::path::Path::new(r"\\server\share\dir")));
    assert!(needs_polling(std::path::Path::new(r"\\wsl$\Ubuntu\home")));
    assert!(needs_polling(std::path::Path::new(r"\\?\UNC\server\share")));
}

#[test]
fn signature_changes_with_entries() {
    let dir = tempfile::tempdir().unwrap();
    let before = DirSignature::read(dir.path()).unwrap();
    assert_eq!(before.entries, 0);
    std::fs::write(dir.path().join("f"), b"x").unwrap();
    let after = DirSignature::read(dir.path()).unwrap();
    assert_eq!(after.entries, 1);
    assert_ne!(before, after);
    assert!(DirSignature::read(&dir.path().join("missing")).is_none());
}

fn sink() -> (ChangeSink, Receiver<DirChange>) {
    let (tx, rx) = channel();
    (ChangeSink::new(tx, || {}), rx)
}

/// Waits (tolerantly) for a change of `dir` while `touch` keeps creating
/// files, as watchers may need a moment to arm.
fn expect_change(rx: &Receiver<DirChange>, dir: &std::path::Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut n = 0;
    while Instant::now() < deadline {
        std::fs::write(dir.join(format!("file{n}")), b"x").unwrap();
        n += 1;
        if let Ok(change) = rx.recv_timeout(Duration::from_millis(200)) {
            assert_eq!(change.dir, dir);
            return;
        }
    }
    panic!("no change reported for {dir:?}");
}

#[test]
fn poller_reports_new_files() {
    let dir = tempfile::tempdir().unwrap();
    let (sink, rx) = sink();
    let plan = MonitorPlan {
        prefer_poll: true,
        poll_interval: Duration::from_millis(20),
    };
    let monitor = DirMonitor::start(dir.path().to_path_buf(), plan, sink);
    expect_change(&rx, dir.path());
    drop(monitor);
}

#[test]
fn watcher_reports_new_files() {
    let dir = tempfile::tempdir().unwrap();
    let (sink, rx) = sink();
    let plan = MonitorPlan {
        prefer_poll: false,
        poll_interval: Duration::from_secs(3600),
    };
    let _monitor = DirMonitor::start(dir.path().to_path_buf(), plan, sink);
    expect_change(&rx, dir.path());
}

/// A sustained burst (like a large copy into the folder) yields about one
/// refresh per `max_wait`, not one per event.
#[test]
fn burst_of_real_events_is_coalesced() {
    let dir = tempfile::tempdir().unwrap();
    let (sink, rx) = sink();
    let plan = MonitorPlan {
        prefer_poll: false,
        poll_interval: Duration::from_secs(3600),
    };
    let _monitor = DirMonitor::start(dir.path().to_path_buf(), plan, sink);
    expect_change(&rx, dir.path());
    let mut coalescer = Coalescer::new(CoalesceTiming::default());
    let start = Instant::now();
    let (mut raw, mut refreshes, mut files) = (0usize, 0usize, 0usize);
    while start.elapsed() < Duration::from_secs(4) {
        if start.elapsed() < Duration::from_secs(3) {
            for _ in 0..20 {
                std::fs::write(dir.path().join(format!("burst{files}")), [0u8; 4096]).unwrap();
                files += 1;
            }
        }
        while let Ok(change) = rx.try_recv() {
            raw += 1;
            coalescer.record(change, Instant::now());
        }
        refreshes += coalescer.take_due(Instant::now()).len();
        std::thread::sleep(Duration::from_millis(5));
    }
    println!("burst: files={files} raw_events={raw} refreshes={refreshes}");
    assert!(raw > refreshes);
    assert!(refreshes <= 10, "{refreshes} refreshes for a 3 s burst");
}

#[test]
fn reads_are_not_changes() {
    use notify::event::{AccessKind, CreateKind, EventKind};
    assert!(!watcher::is_change(&EventKind::Access(AccessKind::Any)));
    assert!(watcher::is_change(&EventKind::Create(CreateKind::File)));
}

/// A file created just before the monitor is armed (after the panel read
/// the folder) is not lost: the folder is reported once when armed.
#[test]
fn change_right_before_arming_is_reported() {
    for prefer_poll in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("early"), b"x").unwrap();
        let (sink, rx) = sink();
        let plan = MonitorPlan {
            prefer_poll,
            poll_interval: Duration::from_secs(3600),
        };
        let _monitor = DirMonitor::start(dir.path().to_path_buf(), plan, sink);
        let change = rx.recv_timeout(Duration::from_secs(10)).expect("reported");
        assert_eq!(change.dir, dir.path());
    }
}

/// An untouched folder is not reread when its monitor starts.
#[test]
fn quiet_folder_is_not_reported_when_armed() {
    let dir = tempfile::tempdir().unwrap();
    let old = filetime::FileTime::from_unix_time(1_600_000_000, 0);
    filetime::set_file_mtime(dir.path(), old).unwrap();
    let (sink, rx) = sink();
    let plan = MonitorPlan {
        prefer_poll: true,
        poll_interval: Duration::from_secs(3600),
    };
    let _monitor = DirMonitor::start(dir.path().to_path_buf(), plan, sink);
    assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
}

#[test]
fn changes_of_two_filesystems_are_kept_apart() {
    let t0 = Instant::now();
    let mut c = Coalescer::new(timing());
    let mut remote = change("/a", &["/a/r"]);
    remote.origin = WatchOrigin::Remote(7);
    c.record(change("/a", &["/a/l"]), t0);
    c.record(remote.clone(), t0);
    let mut due = c.take_due(t0 + ms(300));
    due.sort_by_key(|d| d.origin);
    assert_eq!(due, vec![change("/a", &["/a/l"]), remote]);
}

#[test]
fn a_vfs_folder_is_polled_through_the_port() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, rx) = channel();
    let sink = ChangeSink::new(tx, || {}).with_origin(WatchOrigin::Remote(1));
    let vfs = std::sync::Arc::new(crate::fs::vfs::LocalVfs);
    let _monitor = DirMonitor::start_polling(dir.path().to_path_buf(), vfs, ms(50), sink);
    std::thread::sleep(ms(150));
    std::fs::write(dir.path().join("new"), b"x").unwrap();
    let got = rx.recv_timeout(Duration::from_secs(5)).expect("a change");
    assert_eq!(got.dir, dir.path());
    assert_eq!(got.origin, WatchOrigin::Remote(1));
}
