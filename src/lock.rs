//! Poison-tolerant locking.
//!
//! A panic while a `std::sync::Mutex` is held marks it poisoned; calling
//! `lock().unwrap()` afterwards turns one panic into a cascade that takes the
//! whole UI down. The data guarded in Pairee (job lists, SFTP handles,
//! conflict slots) stays structurally valid after such a panic, so we log and
//! keep going with the inner value instead.

use std::sync::{Mutex, MutexGuard};

pub trait LockExt<T> {
    /// Locks the mutex, recovering the guard if a previous holder panicked.
    fn lock_safe(&self) -> MutexGuard<'_, T>;
}

impl<T> LockExt<T> for Mutex<T> {
    fn lock_safe(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|poisoned| {
            log::warn!("recovering from a poisoned mutex");
            self.clear_poison();
            poisoned.into_inner()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::LockExt;
    use std::sync::{Arc, Mutex};

    #[test]
    fn poisoned_mutex_is_recovered() {
        let m = Arc::new(Mutex::new(vec![1]));
        let m2 = Arc::clone(&m);
        let _ = std::thread::spawn(move || {
            let mut g = m2.lock().unwrap();
            g.push(2);
            panic!("poison it");
        })
        .join();
        assert!(m.is_poisoned());
        assert_eq!(*m.lock_safe(), vec![1, 2]);
        assert!(!m.is_poisoned(), "poison flag cleared");
        m.lock_safe().push(3);
        assert_eq!(m.lock_safe().len(), 3);
    }
}
