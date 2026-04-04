//! Perfetto-compatible trace recording helpers.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;

const DEFAULT_PID: u32 = 1;

#[derive(Clone, Debug, Default, Serialize)]
struct TraceFile {
    #[serde(rename = "traceEvents")]
    trace_events: Vec<PerfettoEvent>,
    #[serde(rename = "displayTimeUnit")]
    display_time_unit: &'static str,
    metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
struct PerfettoEvent {
    name: String,
    cat: String,
    ph: String,
    ts: u64,
    pid: u32,
    tid: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    dur: Option<u64>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    args: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
struct ActiveGuestSpan {
    category: String,
    name: String,
    tid: u32,
}

struct TraceRecorderState {
    started_at: Instant,
    events: Vec<PerfettoEvent>,
    threads: HashMap<String, u32>,
    next_tid: u32,
    next_span_id: i32,
    active_guest_spans: HashMap<i32, ActiveGuestSpan>,
    metadata: BTreeMap<String, String>,
}

impl TraceRecorderState {
    fn now_us(&self) -> u64 {
        self.started_at.elapsed().as_micros() as u64
    }
}

struct TraceRecorderInner {
    output_dir: PathBuf,
    trace_path: PathBuf,
    session_path: PathBuf,
    host_kind: String,
    label: String,
    state: Mutex<TraceRecorderState>,
}

/// Shared recorder that writes Perfetto-compatible trace JSON and session metadata.
#[derive(Clone)]
pub struct TraceRecorder {
    inner: Arc<TraceRecorderInner>,
}

/// RAII host-side span guard that records a complete event when dropped.
pub struct TraceSpanGuard {
    recorder: TraceRecorder,
    thread: String,
    category: String,
    name: String,
    start_us: u64,
    args: BTreeMap<String, String>,
    finished: bool,
}

impl TraceRecorder {
    /// Create a new trace recorder rooted at `output_dir`.
    pub fn new(
        output_dir: impl AsRef<Path>,
        host_kind: impl Into<String>,
        label: impl Into<String>,
    ) -> io::Result<Self> {
        let output_dir = output_dir.as_ref().to_path_buf();
        fs::create_dir_all(&output_dir)?;
        let trace_path = output_dir.join("trace.json");
        let session_path = output_dir.join("session.json");
        let host_kind = host_kind.into();
        let label = label.into();

        let mut state = TraceRecorderState {
            started_at: Instant::now(),
            events: Vec::new(),
            threads: HashMap::new(),
            next_tid: 1,
            next_span_id: 1,
            active_guest_spans: HashMap::new(),
            metadata: BTreeMap::from([
                ("host_kind".to_string(), host_kind.clone()),
                ("label".to_string(), label.clone()),
            ]),
        };
        state.events.push(PerfettoEvent {
            name: "process_name".to_string(),
            cat: "__metadata".to_string(),
            ph: "M".to_string(),
            ts: 0,
            pid: DEFAULT_PID,
            tid: 0,
            dur: None,
            args: BTreeMap::from([("name".to_string(), format!("vzglyd-{host_kind}"))]),
        });

        Ok(Self {
            inner: Arc::new(TraceRecorderInner {
                output_dir,
                trace_path,
                session_path,
                host_kind,
                label,
                state: Mutex::new(state),
            }),
        })
    }

    /// Attach extra string metadata to the current session.
    pub fn set_metadata(&self, key: impl Into<String>, value: impl Into<String>) {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.metadata.insert(key.into(), value.into());
    }

    /// Start a host-side timed span on a logical thread.
    pub fn scoped(
        &self,
        thread: impl Into<String>,
        category: impl Into<String>,
        name: impl Into<String>,
    ) -> TraceSpanGuard {
        self.scoped_with_args(thread, category, name, BTreeMap::new())
    }

    /// Start a host-side timed span with initial string arguments.
    pub fn scoped_with_args(
        &self,
        thread: impl Into<String>,
        category: impl Into<String>,
        name: impl Into<String>,
        args: BTreeMap<String, String>,
    ) -> TraceSpanGuard {
        let start_us = self.now_us();
        TraceSpanGuard {
            recorder: self.clone(),
            thread: thread.into(),
            category: category.into(),
            name: name.into(),
            start_us,
            args,
            finished: false,
        }
    }

    /// Emit an instant event on a logical thread.
    pub fn instant(
        &self,
        thread: impl Into<String>,
        category: impl Into<String>,
        name: impl Into<String>,
        args: BTreeMap<String, String>,
    ) {
        let thread = thread.into();
        let category = category.into();
        let name = name.into();
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let ts = state.now_us();
        let tid = resolve_thread(&mut state, &thread);
        state.events.push(PerfettoEvent {
            name,
            cat: category,
            ph: "i".to_string(),
            ts,
            pid: DEFAULT_PID,
            tid,
            dur: None,
            args,
        });
    }

    /// Start a guest span originating from slide or sidecar WASM code.
    pub fn guest_span_start(
        &self,
        thread: impl Into<String>,
        category: impl Into<String>,
        name: impl Into<String>,
        args: BTreeMap<String, String>,
    ) -> i32 {
        let thread = thread.into();
        let category = category.into();
        let name = name.into();
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let ts = state.now_us();
        let tid = resolve_thread(&mut state, &thread);
        let span_id = state.next_span_id;
        state.next_span_id += 1;
        state.active_guest_spans.insert(
            span_id,
            ActiveGuestSpan {
                category: category.clone(),
                name: name.clone(),
                tid,
            },
        );
        state.events.push(PerfettoEvent {
            name,
            cat: category,
            ph: "B".to_string(),
            ts,
            pid: DEFAULT_PID,
            tid,
            dur: None,
            args,
        });
        span_id
    }

    /// End a guest span previously started by [`TraceRecorder::guest_span_start`].
    pub fn guest_span_end(
        &self,
        span_id: i32,
        status: Option<String>,
        mut args: BTreeMap<String, String>,
    ) {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(active) = state.active_guest_spans.remove(&span_id) else {
            return;
        };
        if let Some(status) = status {
            args.insert("status".to_string(), status);
        }
        let ts = state.now_us();
        state.events.push(PerfettoEvent {
            name: active.name,
            cat: active.category,
            ph: "E".to_string(),
            ts,
            pid: DEFAULT_PID,
            tid: active.tid,
            dur: None,
            args,
        });
    }

    /// Flush trace and session metadata to disk.
    pub fn flush(&self) -> io::Result<PathBuf> {
        let (trace_file, session_metadata) = {
            let state = self
                .inner
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (
                TraceFile {
                    trace_events: state.events.clone(),
                    display_time_unit: "ms",
                    metadata: state.metadata.clone(),
                },
                SessionMetadata {
                    host_kind: self.inner.host_kind.clone(),
                    label: self.inner.label.clone(),
                    output_dir: self.inner.output_dir.display().to_string(),
                    trace_path: self.inner.trace_path.display().to_string(),
                    metadata: state.metadata.clone(),
                },
            )
        };

        let trace_json = serde_json::to_vec_pretty(&trace_file)
            .map_err(|error| io::Error::other(error.to_string()))?;
        let session_json = serde_json::to_vec_pretty(&session_metadata)
            .map_err(|error| io::Error::other(error.to_string()))?;
        fs::write(&self.inner.trace_path, trace_json)?;
        fs::write(&self.inner.session_path, session_json)?;
        Ok(self.inner.trace_path.clone())
    }

    /// Return the trace output path.
    pub fn trace_path(&self) -> &Path {
        &self.inner.trace_path
    }

    fn now_us(&self) -> u64 {
        let state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.now_us()
    }

    fn record_complete(
        &self,
        thread: &str,
        category: &str,
        name: &str,
        start_us: u64,
        args: BTreeMap<String, String>,
    ) {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let tid = resolve_thread(&mut state, thread);
        let end_us = state.now_us();
        state.events.push(PerfettoEvent {
            name: name.to_string(),
            cat: category.to_string(),
            ph: "X".to_string(),
            ts: start_us,
            pid: DEFAULT_PID,
            tid,
            dur: Some(end_us.saturating_sub(start_us)),
            args,
        });
    }
}

impl TraceSpanGuard {
    /// Add a string attribute to the eventual complete span event.
    pub fn add_attr(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.args.insert(key.into(), value.into());
    }

    /// Finish the span immediately.
    pub fn finish(mut self) {
        self.flush();
    }

    fn flush(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        self.recorder.record_complete(
            &self.thread,
            &self.category,
            &self.name,
            self.start_us,
            self.args.clone(),
        );
    }
}

impl Drop for TraceSpanGuard {
    fn drop(&mut self) {
        self.flush();
    }
}

#[derive(Serialize)]
struct SessionMetadata {
    host_kind: String,
    label: String,
    output_dir: String,
    trace_path: String,
    metadata: BTreeMap<String, String>,
}

fn resolve_thread(state: &mut TraceRecorderState, key: &str) -> u32 {
    if let Some(existing) = state.threads.get(key) {
        return *existing;
    }

    let tid = state.next_tid;
    state.next_tid += 1;
    state.threads.insert(key.to_string(), tid);
    state.events.push(PerfettoEvent {
        name: "thread_name".to_string(),
        cat: "__metadata".to_string(),
        ph: "M".to_string(),
        ts: 0,
        pid: DEFAULT_PID,
        tid,
        dur: None,
        args: BTreeMap::from([("name".to_string(), key.to_string())]),
    });
    tid
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::TraceRecorder;

    fn unique_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("vzglyd_trace_{name}_{nanos}"))
    }

    #[test]
    fn writes_perfetto_trace_file() {
        let out_dir = unique_dir("perfetto");
        let recorder = TraceRecorder::new(&out_dir, "native", "test").expect("recorder");
        {
            let mut span = recorder.scoped("main", "frame", "render_frame");
            span.add_attr("slide", "air_quality");
        }
        recorder.flush().expect("flush");
        let trace = std::fs::read_to_string(out_dir.join("trace.json")).expect("trace file");
        assert!(trace.contains("\"name\": \"render_frame\""));
        assert!(trace.contains("\"ph\": \"X\""));
    }
}
