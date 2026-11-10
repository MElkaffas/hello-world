//! Running engine processes with cancellation, progress parsing and stderr capture.

use crate::engines::{engine_command, EngineId, Engines};
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("Cancelled")]
    Cancelled,
    #[error("{0} is not installed")]
    MissingEngine(&'static str),
    #[error("{engine} failed (exit code {code}): {detail}")]
    Failed { engine: &'static str, code: i32, detail: String },
    #[error("Could not start {0}: {1}")]
    Spawn(&'static str, std::io::Error),
}

#[derive(Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst)
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Run an engine. `on_line` receives stdout lines (used for ffmpeg `-progress`).
pub fn run(
    engines: &Engines,
    id: EngineId,
    args: &[OsString],
    cwd: Option<&Path>,
    cancel: &CancelToken,
    mut on_line: impl FnMut(&str),
) -> Result<String, RunError> {
    let program = engines.path(id).ok_or(RunError::MissingEngine(id.label()))?;
    let mut cmd = engine_command(id, program);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    let mut child = cmd.spawn().map_err(|e| RunError::Spawn(id.label(), e))?;

    let stderr = child.stderr.take().unwrap();
    let err_buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let eb = err_buf.clone();
    let err_thread = std::thread::spawn(move || {
        let mut r = BufReader::new(stderr);
        let mut chunk = [0u8; 4096];
        while let Ok(n) = r.read(&mut chunk) {
            if n == 0 {
                break;
            }
            let mut b = eb.lock().unwrap();
            b.extend_from_slice(&chunk[..n]);
            // Keep only the last 64 KB
            if b.len() > 65536 {
                let cut = b.len() - 65536;
                b.drain(..cut);
            }
        }
    });

    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let out_thread = std::thread::spawn(move || {
        let r = BufReader::new(stdout);
        for line in r.split(b'\n').map_while(Result::ok) {
            if tx.send(String::from_utf8_lossy(&line).trim_end().to_string()).is_err() {
                break;
            }
        }
    });

    let mut stdout_text = String::new();
    let status = loop {
        while let Ok(line) = rx.try_recv() {
            on_line(&line);
            if stdout_text.len() < 1 << 20 {
                stdout_text.push_str(&line);
                stdout_text.push('\n');
            }
        }
        if cancel.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = out_thread.join();
            let _ = err_thread.join();
            return Err(RunError::Cancelled);
        }
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(RunError::Spawn(id.label(), e)),
        }
    };
    let _ = out_thread.join();
    while let Ok(line) = rx.try_recv() {
        on_line(&line);
        stdout_text.push_str(&line);
        stdout_text.push('\n');
    }
    let _ = err_thread.join();
    let stderr_text = String::from_utf8_lossy(&err_buf.lock().unwrap()).to_string();
    // qpdf exits with 3 for "succeeded with warnings".
    let ok = status.success() || (id == EngineId::Qpdf && status.code() == Some(3));
    if !ok {
        return Err(RunError::Failed { engine: id.label(), code: status.code().unwrap_or(-1), detail: summarize_error(&stderr_text, &stdout_text) });
    }
    Ok(stdout_text)
}

/// Pull the most useful lines out of an engine's error output and translate common causes.
pub fn summarize_error(stderr: &str, stdout: &str) -> String {
    let all = format!("{stderr}\n{stdout}");
    let lower = all.to_lowercase();
    if lower.contains("no space left") || lower.contains("not enough space") || lower.contains("disk full") {
        return "Not enough disk space to write the output".into();
    }
    if lower.contains("invalid password") || lower.contains("wrong password") || lower.contains("incorrect password") {
        return "The password is incorrect".into();
    }
    if lower.contains("unknown encoder") || lower.contains("encoder not found") {
        return "The required codec is not available in the installed FFmpeg build".into();
    }
    if lower.contains("invalid data found when processing input") || lower.contains("moov atom not found") {
        return "The file appears to be damaged or is not a valid media file".into();
    }
    if lower.contains("no delegate for this image format") || lower.contains("no decode delegate") {
        return "ImageMagick cannot decode this image (missing delegate)".into();
    }
    if lower.contains("syntax error") && lower.contains("pdf") || lower.contains("not a pdf file") || lower.contains("couldn't read xref") {
        return "The PDF appears to be damaged".into();
    }
    let lines: Vec<&str> = all.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let interesting: Vec<&str> = lines.iter().rev().filter(|l| {
        let l = l.to_lowercase();
        l.contains("error") || l.contains("fail") || l.contains("invalid") || l.contains("cannot") || l.contains("unable")
    }).take(3).copied().collect();
    let pick = if interesting.is_empty() { lines.iter().rev().take(3).copied().collect::<Vec<_>>() } else { interesting };
    let s = pick.into_iter().rev().collect::<Vec<_>>().join(" | ");
    if s.is_empty() { "No error details were reported".into() } else { s.chars().take(600).collect() }
}

pub fn os(s: impl Into<OsString>) -> OsString {
    s.into()
}
