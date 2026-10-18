use crate::lock::LockExt;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use uuid::Uuid;

use super::job::{TransferJob, TransferJobStatus};

#[derive(Clone)]
pub struct TransferQueue {
    jobs: Arc<Mutex<VecDeque<TransferJob>>>,
    active_job_id: Arc<Mutex<Option<Uuid>>>,
    /// Signalled whenever jobs are added or changed, so the engine's
    /// coordinator waits for work instead of polling.
    changed: Arc<Notify>,
}

impl TransferQueue {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(VecDeque::new())),
            active_job_id: Arc::new(Mutex::new(None)),
            changed: Arc::new(Notify::new()),
        }
    }

    /// Waits until the queue changes (a change made while nobody waited
    /// is remembered).
    pub async fn changed(&self) {
        self.changed.notified().await;
    }

    pub fn enqueue(&self, job: TransferJob) {
        self.jobs.lock_safe().push_back(job);
        self.changed.notify_one();
    }

    pub fn dequeue(&self) -> Option<TransferJob> {
        let mut active_id = self.active_job_id.lock_safe();
        let mut jobs = self.jobs.lock_safe();
        if let Some(idx) = jobs
            .iter()
            .position(|j| j.status == TransferJobStatus::Queued)
        {
            let mut job = jobs[idx].clone();
            *active_id = Some(job.id);
            job.status = TransferJobStatus::Scanning;
            jobs[idx] = job.clone();
            Some(job)
        } else {
            None
        }
    }

    pub fn remove(&self, job_id: Uuid) -> bool {
        let mut jobs = self.jobs.lock_safe();
        if let Some(idx) = jobs.iter().position(|j| j.id == job_id) {
            let job = &jobs[idx];
            // No podemos remover el trabajo activo si está corriendo
            if job.is_active() {
                return false;
            }
            jobs.remove(idx);
            true
        } else {
            false
        }
    }

    /// Mueve el trabajo en la cola. Dirección: -1 para subir, 1 para bajar.
    pub fn reorder(&self, job_id: Uuid, direction: i32) -> bool {
        let mut jobs = self.jobs.lock_safe();
        if let Some(idx) = jobs.iter().position(|j| j.id == job_id) {
            // El trabajo activo o completados no se pueden mover
            if jobs[idx].is_active() || jobs[idx].is_terminal() {
                return false;
            }

            let new_idx = idx as i32 + direction;
            if new_idx < 0 || new_idx >= jobs.len() as i32 {
                return false;
            }

            let new_idx = new_idx as usize;
            // Tampoco podemos intercambiar con un trabajo activo
            if jobs[new_idx].is_active() || jobs[new_idx].is_terminal() {
                return false;
            }

            jobs.swap(idx, new_idx);
            true
        } else {
            false
        }
    }

    /// Moves the queued job `job_id` ahead of every other queued job, so
    /// it runs next; `false` when it is not queued.
    pub fn move_to_front(&self, job_id: Uuid) -> bool {
        let mut jobs = self.jobs.lock_safe();
        let Some(idx) = jobs
            .iter()
            .position(|j| j.id == job_id && j.status == TransferJobStatus::Queued)
        else {
            return false;
        };
        let first = jobs
            .iter()
            .position(|j| j.status == TransferJobStatus::Queued)
            .unwrap_or(idx);
        if let Some(job) = jobs.remove(idx) {
            jobs.insert(first, job);
        }
        true
    }

    pub fn get_all(&self) -> Vec<TransferJob> {
        let jobs = self.jobs.lock_safe();
        jobs.iter().cloned().collect()
    }

    /// `f` over the listed jobs without copying them (a job's logs and
    /// results grow with every file, so the UI must not clone them per frame).
    pub fn with_jobs<R>(&self, f: impl FnOnce(&VecDeque<TransferJob>) -> R) -> R {
        f(&self.jobs.lock_safe())
    }

    /// A copy of the job `job_id`, if it is still listed.
    pub fn get(&self, job_id: Uuid) -> Option<TransferJob> {
        self.jobs
            .lock_safe()
            .iter()
            .find(|j| j.id == job_id)
            .cloned()
    }

    pub fn pending_count(&self) -> usize {
        let jobs = self.jobs.lock_safe();
        jobs.iter()
            .filter(|j| j.status == TransferJobStatus::Queued)
            .count()
    }

    pub fn clear_completed(&self) {
        let mut jobs = self.jobs.lock_safe();
        jobs.retain(|j| !j.is_terminal());
    }

    pub fn update_job<F>(&self, job_id: Uuid, update_fn: F)
    where
        F: FnOnce(&mut TransferJob),
    {
        let mut jobs = self.jobs.lock_safe();
        if let Some(job) = jobs.iter_mut().find(|j| j.id == job_id) {
            update_fn(job);
        }
        drop(jobs);
        self.changed.notify_one();
    }

    /// Whether a job is running (checked without copying the jobs).
    pub fn has_active(&self) -> bool {
        self.jobs.lock_safe().iter().any(TransferJob::is_active)
    }

    /// Number of listed jobs.
    pub fn len(&self) -> usize {
        self.jobs.lock_safe().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Drops the oldest log lines of every job so each keeps at most `keep`
    /// (in place: no job is copied).
    pub fn trim_logs(&self, keep: usize) {
        for job in self.jobs.lock_safe().iter_mut() {
            let excess = job.log_lines.len().saturating_sub(keep);
            job.log_lines.drain(..excess);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::transfer::job::TransferOperation;
    use crate::fs::transfer::options::TransferOptions;

    #[test]
    fn trim_logs_keeps_the_newest_lines() {
        let queue = TransferQueue::new();
        let mut job = TransferJob::new(
            TransferOperation::Copy,
            Vec::new(),
            Default::default(),
            TransferOptions::default(),
        );
        job.log_lines = (0..5).map(|i| i.to_string()).collect();
        let id = job.id;
        queue.enqueue(job);
        queue.trim_logs(2);
        assert_eq!(queue.get(id).unwrap().log_lines, ["3", "4"]);
        queue.trim_logs(10);
        assert_eq!(queue.get(id).unwrap().log_lines.len(), 2);
    }
}
