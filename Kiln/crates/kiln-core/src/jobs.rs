//! Background job queue: a fixed pool of worker threads, a configurable parallelism limit,
//! pause/resume of the queue, per-job cancellation and throttled progress events.

use crate::catalog::actions_for_selection;
use crate::catalog::Input;
use crate::detect::detect_path;
use crate::engines::Engines;
use crate::exec::{action_name, execute, JobError, JobSpec, Outcome};
use crate::process::CancelToken;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Skipped,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn finished(self) -> bool {
        matches!(self, JobStatus::Done | JobStatus::Skipped | JobStatus::Failed | JobStatus::Cancelled)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct JobSnapshot {
    pub id: Uuid,
    pub batch: Uuid,
    pub label: String,
    pub inputs: Vec<PathBuf>,
    pub status: JobStatus,
    pub progress: Option<f64>,
    pub stage: String,
    pub eta_secs: Option<u64>,
    pub elapsed_ms: u64,
    pub outputs: Vec<PathBuf>,
    pub error: Option<String>,
    pub note: Option<String>,
    pub created: chrono::DateTime<chrono::Local>,
    #[serde(skip)]
    pub spec: Option<JobSpec>,
}

struct Entry {
    snap: JobSnapshot,
    spec: JobSpec,
    cancel: CancelToken,
    started: Option<Instant>,
}

struct State {
    queue: VecDeque<Uuid>,
    jobs: HashMap<Uuid, Entry>,
    order: Vec<Uuid>,
    paused: bool,
    max_parallel: usize,
    running: usize,
    shutdown: bool,
}

pub type EventFn = dyn Fn(&JobSnapshot) + Send + Sync;

struct Inner {
    state: Mutex<State>,
    cv: Condvar,
    engines: RwLock<Arc<Engines>>,
    on_event: Box<EventFn>,
}

#[derive(Clone)]
pub struct JobManager {
    inner: Arc<Inner>,
}

pub const MAX_WORKERS: usize = 8;

impl JobManager {
    pub fn new(engines: Arc<Engines>, max_parallel: usize, on_event: Box<EventFn>) -> JobManager {
        let inner = Arc::new(Inner {
            state: Mutex::new(State { queue: VecDeque::new(), jobs: HashMap::new(), order: vec![], paused: false, max_parallel: max_parallel.clamp(1, MAX_WORKERS), running: 0, shutdown: false }),
            cv: Condvar::new(),
            engines: RwLock::new(engines),
            on_event,
        });
        for i in 0..MAX_WORKERS {
            let inner = inner.clone();
            std::thread::Builder::new().name(format!("kiln-worker-{i}")).spawn(move || worker(inner)).expect("spawn worker");
        }
        JobManager { inner }
    }

    pub fn set_engines(&self, e: Arc<Engines>) {
        *self.inner.engines.write().unwrap() = e;
    }

    pub fn engines(&self) -> Arc<Engines> {
        self.inner.engines.read().unwrap().clone()
    }

    pub fn set_max_parallel(&self, n: usize) {
        self.inner.state.lock().unwrap().max_parallel = n.clamp(1, MAX_WORKERS);
        self.inner.cv.notify_all();
    }

    /// Queue work. Actions that process one file are split into one job per input so a batch
    /// runs in parallel and a single failure doesn't stop the rest.
    pub fn submit(&self, spec: JobSpec) -> Result<Vec<Uuid>, String> {
        spec.options.validate()?;
        if spec.inputs.is_empty() {
            return Err("No files selected".into());
        }
        let engines = self.engines();
        let kinds: Vec<Input> = spec
            .inputs
            .iter()
            .map(|p| if p.is_dir() { Input::Folder } else { Input::File(detect_path(p).ok().and_then(|d| d.format).unwrap_or(crate::formats::Format::Txt)) })
            .collect();
        let combines = actions_for_selection(&kinds, &engines).iter().find(|a| a.action == spec.action).map(|a| a.combines_inputs).unwrap_or(false);
        let specs: Vec<JobSpec> = if combines || spec.inputs.len() == 1 {
            vec![spec]
        } else {
            spec.inputs.iter().map(|p| JobSpec { inputs: vec![p.clone()], ..spec.clone() }).collect()
        };
        let batch = Uuid::new_v4();
        let mut ids = vec![];
        let mut snaps = vec![];
        {
            let mut st = self.inner.state.lock().unwrap();
            for s in specs {
                let id = Uuid::new_v4();
                let snap = JobSnapshot {
                    id, batch, label: action_name(s.action), inputs: s.inputs.clone(), status: JobStatus::Queued, progress: None,
                    stage: "Waiting".into(), eta_secs: None, elapsed_ms: 0, outputs: vec![], error: None, note: None,
                    created: chrono::Local::now(), spec: Some(s.clone()),
                };
                snaps.push(snap.clone());
                st.jobs.insert(id, Entry { snap, spec: s, cancel: CancelToken::default(), started: None });
                st.queue.push_back(id);
                st.order.push(id);
                ids.push(id);
            }
        }
        for s in &snaps {
            (self.inner.on_event)(s);
        }
        self.inner.cv.notify_all();
        Ok(ids)
    }

    pub fn cancel(&self, id: Uuid) -> bool {
        let snap = {
            let mut st = self.inner.state.lock().unwrap();
            let Some(e) = st.jobs.get_mut(&id) else { return false };
            match e.snap.status {
                JobStatus::Queued => {
                    e.snap.status = JobStatus::Cancelled;
                    e.snap.stage = "Cancelled".into();
                    let s = e.snap.clone();
                    st.queue.retain(|q| *q != id);
                    Some(s)
                }
                JobStatus::Running => {
                    e.cancel.cancel();
                    e.snap.stage = "Cancelling…".into();
                    Some(e.snap.clone())
                }
                _ => None,
            }
        };
        if let Some(s) = snap {
            (self.inner.on_event)(&s);
            true
        } else {
            false
        }
    }

    pub fn cancel_all(&self) {
        let ids: Vec<Uuid> = self.inner.state.lock().unwrap().order.clone();
        for id in ids {
            self.cancel(id);
        }
    }

    /// Pausing holds queued jobs; running conversions finish (engines cannot be suspended safely).
    pub fn set_paused(&self, paused: bool) {
        self.inner.state.lock().unwrap().paused = paused;
        self.inner.cv.notify_all();
    }

    pub fn is_paused(&self) -> bool {
        self.inner.state.lock().unwrap().paused
    }

    pub fn snapshots(&self) -> Vec<JobSnapshot> {
        let st = self.inner.state.lock().unwrap();
        st.order.iter().filter_map(|id| st.jobs.get(id)).map(|e| live(e)).collect()
    }

    pub fn get(&self, id: Uuid) -> Option<JobSnapshot> {
        self.inner.state.lock().unwrap().jobs.get(&id).map(live)
    }

    pub fn clear_finished(&self) {
        let mut st = self.inner.state.lock().unwrap();
        let done: Vec<Uuid> = st.jobs.iter().filter(|(_, e)| e.snap.status.finished()).map(|(k, _)| *k).collect();
        for id in done {
            st.jobs.remove(&id);
        }
        let jobs = std::mem::take(&mut st.jobs);
        st.order.retain(|id| jobs.contains_key(id));
        st.jobs = jobs;
    }

    pub fn active_count(&self) -> usize {
        let st = self.inner.state.lock().unwrap();
        st.jobs.values().filter(|e| !e.snap.status.finished()).count()
    }

    /// Block until every job has finished (used by tests and the QA harness).
    pub fn wait_idle(&self, timeout: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if self.active_count() == 0 {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    pub fn shutdown(&self) {
        self.cancel_all();
        self.inner.state.lock().unwrap().shutdown = true;
        self.inner.cv.notify_all();
    }
}

fn live(e: &Entry) -> JobSnapshot {
    let mut s = e.snap.clone();
    if let Some(t) = e.started {
        if s.status == JobStatus::Running {
            s.elapsed_ms = t.elapsed().as_millis() as u64;
        }
    }
    s
}

fn worker(inner: Arc<Inner>) {
    loop {
        let (id, spec, cancel) = {
            let mut st = inner.state.lock().unwrap();
            loop {
                if st.shutdown {
                    return;
                }
                if !st.paused && st.running < st.max_parallel {
                    if let Some(id) = st.queue.pop_front() {
                        st.running += 1;
                        let e = st.jobs.get_mut(&id).unwrap();
                        e.snap.status = JobStatus::Running;
                        e.snap.stage = "Starting".into();
                        e.started = Some(Instant::now());
                        break (id, e.spec.clone(), e.cancel.clone());
                    }
                }
                st = inner.cv.wait_timeout(st, Duration::from_millis(500)).unwrap().0;
            }
        };
        emit(&inner, id);
        let engines = inner.engines.read().unwrap().clone();
        let started = Instant::now();
        let mut last_emit = Instant::now() - Duration::from_secs(1);
        let inner2 = inner.clone();
        let mut sink = move |fraction: Option<f64>, stage: &str| {
            {
                let mut st = inner2.state.lock().unwrap();
                if let Some(e) = st.jobs.get_mut(&id) {
                    if e.cancel.is_cancelled() {
                        return;
                    }
                    e.snap.progress = fraction;
                    e.snap.stage = stage.to_string();
                    let el = started.elapsed().as_secs_f64();
                    e.snap.eta_secs = match fraction {
                        Some(f) if f > 0.03 && el > 1.0 && f < 1.0 => Some((el * (1.0 - f) / f).round() as u64),
                        _ => None,
                    };
                }
            }
            if last_emit.elapsed() >= Duration::from_millis(150) {
                last_emit = Instant::now();
                emit(&inner2, id);
            }
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute(&spec, &engines, &cancel, &mut sink)));
        {
            let mut st = inner.state.lock().unwrap();
            st.running -= 1;
            if let Some(e) = st.jobs.get_mut(&id) {
                e.snap.elapsed_ms = started.elapsed().as_millis() as u64;
                e.snap.eta_secs = None;
                apply_result(&mut e.snap, result);
            }
        }
        inner.cv.notify_all();
        emit(&inner, id);
    }
}

fn apply_result(s: &mut JobSnapshot, result: std::thread::Result<Result<Outcome, JobError>>) {
    match result {
        Ok(Ok(o)) => {
            s.status = if o.skipped { JobStatus::Skipped } else { JobStatus::Done };
            s.progress = Some(1.0);
            s.stage = if o.skipped { "Skipped".into() } else { "Done".into() };
            s.outputs = o.outputs;
            s.note = o.note;
        }
        Ok(Err(JobError::Cancelled)) => {
            s.status = JobStatus::Cancelled;
            s.stage = "Cancelled".into();
        }
        Ok(Err(err)) => {
            s.status = JobStatus::Failed;
            s.stage = "Failed".into();
            s.error = Some(err.to_string());
        }
        Err(_) => {
            s.status = JobStatus::Failed;
            s.stage = "Failed".into();
            s.error = Some("Internal error while processing this file".into());
        }
    }
}

fn emit(inner: &Inner, id: Uuid) {
    let snap = inner.state.lock().unwrap().jobs.get(&id).map(live);
    if let Some(s) = snap {
        (inner.on_event)(&s);
    }
}
