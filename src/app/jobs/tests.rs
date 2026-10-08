use super::JobSlot;
use super::cancel::CancelToken;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Polls `slot` until it yields a result or the timeout expires.
async fn wait_result<T: Send + 'static, P: Send + Sync + 'static>(
    slot: &mut JobSlot<T, P>,
) -> Option<T> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Some(v) = slot.poll() {
            return Some(v);
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    None
}

#[test]
fn runs_inline_without_runtime() {
    let mut slot: JobSlot<u32> = JobSlot::new();
    slot.start(|_| 7);
    assert_eq!(slot.poll(), Some(7));
    assert!(!slot.is_running());
}

#[tokio::test]
async fn stale_generation_is_dropped() {
    let mut slot: JobSlot<&'static str> = JobSlot::new();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let (done_tx, done_rx) = mpsc::channel::<()>();
    let first = slot.start(move |_| {
        let _ = release_rx.recv();
        let _ = done_tx.send(());
        "stale"
    });
    let second = slot.start(|_| "fresh");
    assert!(second > first);

    assert_eq!(wait_result(&mut slot).await, Some("fresh"));
    // Let the superseded job finish now; its result must never surface.
    release_tx.send(()).unwrap();
    done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(slot.poll(), None);
}

#[tokio::test]
async fn superseded_job_sees_cancellation() {
    let mut slot: JobSlot<bool> = JobSlot::new();
    let (seen_tx, seen_rx) = mpsc::channel::<bool>();
    slot.start(move |ctx| {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !ctx.is_cancelled() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        let _ = seen_tx.send(ctx.is_cancelled());
        true
    });
    slot.cancel();
    assert!(!slot.is_running());
    assert!(seen_rx.recv_timeout(Duration::from_secs(5)).unwrap());
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(slot.poll(), None, "cancelled job result is ignored");
}

#[tokio::test]
async fn progress_reports_latest_value() {
    let mut slot: JobSlot<(), u32> = JobSlot::new();
    let (go_tx, go_rx) = mpsc::channel::<()>();
    let (reported_tx, reported_rx) = mpsc::channel::<()>();
    slot.start(move |ctx| {
        for i in 1..=3 {
            ctx.report(i);
        }
        let _ = reported_tx.send(());
        let _ = go_rx.recv();
    });
    reported_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(slot.progress(), Some(3));
    go_tx.send(()).unwrap();
    assert_eq!(wait_result(&mut slot).await, Some(()));
    assert_eq!(slot.progress(), None);
}

#[tokio::test]
async fn panicking_job_finishes_without_result() {
    let mut slot: JobSlot<u8> = JobSlot::new();
    slot.start(|_| panic!("boom"));
    let deadline = Instant::now() + Duration::from_secs(5);
    while slot.is_running() && Instant::now() < deadline {
        assert_eq!(slot.poll(), None);
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(!slot.is_running());
}

#[test]
fn cancel_token_is_shared() {
    let token = CancelToken::new();
    let clone = token.clone();
    assert!(!clone.is_cancelled());
    token.cancel();
    assert!(clone.is_cancelled());
}
