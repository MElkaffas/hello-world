//! Conversion history persisted as JSON in the app data folder. Entries are written when a job
//! starts and updated when it ends, so a crash leaves a "running" record that is reported as
//! "interrupted" on the next launch.

use crate::exec::JobSpec;
use crate::jobs::{JobSnapshot, JobStatus};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: Uuid,
    pub time: chrono::DateTime<chrono::Local>,
    pub label: String,
    pub inputs: Vec<PathBuf>,
    pub outputs: Vec<PathBuf>,
    /// queued | running | done | skipped | failed | cancelled | interrupted
    pub status: String,
    pub error: Option<String>,
    pub note: Option<String>,
    pub elapsed_ms: u64,
    pub spec: Option<JobSpec>,
}

pub struct History {
    path: PathBuf,
    pub entries: Vec<HistoryEntry>,
    limit: usize,
}

impl History {
    pub fn load(path: &Path) -> History {
        let mut entries: Vec<HistoryEntry> = fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let mut changed = false;
        for e in &mut entries {
            if e.status == "running" || e.status == "queued" {
                e.status = "interrupted".into();
                e.error = Some("Kiln closed before this conversion finished. Nothing was written to the destination.".into());
                changed = true;
            }
        }
        let h = History { path: path.to_path_buf(), entries, limit: 1000 };
        if changed {
            let _ = h.save();
        }
        h
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(p) = self.path.parent() {
            fs::create_dir_all(p)?;
        }
        // Write-then-rename so a crash never leaves a truncated history file.
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(&self.entries)?)?;
        fs::rename(&tmp, &self.path)
    }

    pub fn record(&mut self, s: &JobSnapshot) {
        // Queued and cancelled-before-start jobs are not interesting history.
        if s.status == JobStatus::Queued {
            return;
        }
        let status = match s.status {
            JobStatus::Queued => "queued",
            JobStatus::Running => "running",
            JobStatus::Done => "done",
            JobStatus::Skipped => "skipped",
            JobStatus::Failed => "failed",
            JobStatus::Cancelled => "cancelled",
        };
        let entry = HistoryEntry {
            id: s.id, time: s.created, label: s.label.clone(), inputs: s.inputs.clone(), outputs: s.outputs.clone(), status: status.into(),
            error: s.error.clone(), note: s.note.clone(), elapsed_ms: s.elapsed_ms, spec: s.spec.clone(),
        };
        match self.entries.iter_mut().find(|e| e.id == s.id) {
            Some(e) => {
                if e.status == status && !s.status.finished() {
                    return;
                }
                *e = entry
            }
            None => self.entries.insert(0, entry),
        }
        self.entries.truncate(self.limit);
        let _ = self.save();
    }

    pub fn clear(&mut self) -> std::io::Result<()> {
        self.entries.clear();
        self.save()
    }

    pub fn remove(&mut self, id: Uuid) -> std::io::Result<()> {
        self.entries.retain(|e| e.id != id);
        self.save()
    }
}
