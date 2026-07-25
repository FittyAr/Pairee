use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::job::{TransferJob, TransferJobStatus};

#[derive(Clone)]
pub struct TransferQueue {
    jobs: Arc<Mutex<VecDeque<TransferJob>>>,
    active_job_id: Arc<Mutex<Option<Uuid>>>,
}

impl TransferQueue {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(VecDeque::new())),
            active_job_id: Arc::new(Mutex::new(None)),
        }
    }

    pub fn enqueue(&self, job: TransferJob) {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.push_back(job);
    }

    pub fn dequeue(&self) -> Option<TransferJob> {
        let mut active_id = self.active_job_id.lock().unwrap();
        let mut jobs = self.jobs.lock().unwrap();
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
        let mut jobs = self.jobs.lock().unwrap();
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
        let mut jobs = self.jobs.lock().unwrap();
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

    pub fn get_all(&self) -> Vec<TransferJob> {
        let jobs = self.jobs.lock().unwrap();
        jobs.iter().cloned().collect()
    }

    pub fn pending_count(&self) -> usize {
        let jobs = self.jobs.lock().unwrap();
        jobs.iter()
            .filter(|j| j.status == TransferJobStatus::Queued)
            .count()
    }

    pub fn clear_completed(&self) {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.retain(|j| !j.is_terminal());
    }

    pub fn update_job<F>(&self, job_id: Uuid, update_fn: F)
    where
        F: FnOnce(&mut TransferJob),
    {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.iter_mut().find(|j| j.id == job_id) {
            update_fn(job);
        }
    }
}
