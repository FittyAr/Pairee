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
        let mut jobs = self.jobs.lock().unwrap();
        // Buscar el primer trabajo Queued
        if let Some(idx) = jobs.iter().position(|j| j.status == TransferJobStatus::Queued) {
            let mut job = jobs.remove(idx).unwrap();
            let mut active_id = self.active_job_id.lock().unwrap();
            *active_id = Some(job.id);
            job.status = TransferJobStatus::Scanning;
            // Lo insertamos de nuevo como el trabajo activo (al principio)
            jobs.push_front(job.clone());
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

    pub fn get_active(&self) -> Option<TransferJob> {
        let active_id = self.active_job_id.lock().unwrap();
        if let Some(id) = *active_id {
            let jobs = self.jobs.lock().unwrap();
            jobs.iter().find(|j| j.id == id).cloned()
        } else {
            None
        }
    }

    pub fn set_active_status(&self, status: TransferJobStatus) {
        let active_id = self.active_job_id.lock().unwrap();
        if let Some(id) = *active_id {
            let mut jobs = self.jobs.lock().unwrap();
            if let Some(job) = jobs.iter_mut().find(|j| j.id == id) {
                job.status = status;
            }
        }
    }

    pub fn update_active_progress<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut super::job::TransferProgress),
    {
        let active_id = self.active_job_id.lock().unwrap();
        if let Some(id) = *active_id {
            let mut jobs = self.jobs.lock().unwrap();
            if let Some(job) = jobs.iter_mut().find(|j| j.id == id) {
                update_fn(&mut job.progress);
            }
        }
    }

    pub fn update_active_results<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut super::job::TransferResults),
    {
        let active_id = self.active_job_id.lock().unwrap();
        if let Some(id) = *active_id {
            let mut jobs = self.jobs.lock().unwrap();
            if let Some(job) = jobs.iter_mut().find(|j| j.id == id) {
                update_fn(&mut job.results);
            }
        }
    }

    pub fn pending_count(&self) -> usize {
        let jobs = self.jobs.lock().unwrap();
        jobs.iter().filter(|j| j.status == TransferJobStatus::Queued).count()
    }

    pub fn clear_completed(&self) {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.retain(|j| !j.is_terminal());
    }

    pub fn clear_active(&self) {
        let mut active_id = self.active_job_id.lock().unwrap();
        *active_id = None;
    }
}
