use crate::triage::TriageResult;
use chrono::{Local, TimeZone};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sysinfo::System;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineStage {
    Idle,
    Triage,
    ReaExtraction,
    CodeSynthesis,
    CargoBuild,
    Validation,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMessage {
    pub timestamp: String,
    pub level: String, // "INFO", "REA", "WARN", "ERROR", "BUILD"
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostMetrics {
    pub cpu_percent: f32,
    pub cpu_cores: usize,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
}

impl Default for HostMetrics {
    fn default() -> Self {
        Self {
            cpu_percent: 0.0,
            cpu_cores: 0,
            memory_used_bytes: 0,
            memory_total_bytes: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub id: String,
    #[serde(skip)]
    pub started_epoch_ms: u64,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub elapsed_ms: u64,
    pub host: HostMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub stage: PipelineStage,
    pub progress_pct: u8,
    pub target_file: Option<String>,
    pub technology: Option<String>,
    pub architecture: Option<String>,
    pub total_apis: usize,
    pub triage: Option<TriageResult>,
    pub session: SessionState,
    pub logs: Vec<LogMessage>,
}

impl Default for SystemState {
    fn default() -> Self {
        let now = now_epoch_ms();
        Self {
            stage: PipelineStage::Idle,
            progress_pct: 0,
            target_file: None,
            technology: None,
            architecture: None,
            total_apis: 0,
            triage: None,
            session: SessionState {
                id: format!("session-{now}"),
                started_epoch_ms: now,
                started_at: format_timestamp(now),
                finished_at: None,
                elapsed_ms: 0,
                host: HostMetrics::default(),
            },
            logs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct StateManager {
    state: Arc<Mutex<SystemState>>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(SystemState::default())),
        }
    }

    pub fn set_stage(&self, stage: PipelineStage, progress: u8) {
        if let Ok(mut state) = self.state.lock() {
            state.stage = stage;
            state.progress_pct = progress;
        }
    }

    pub fn set_target(&self, file: &str, tech: &str, arch: &str, apis: usize) {
        if let Ok(mut state) = self.state.lock() {
            state.target_file = Some(file.to_string());
            state.technology = Some(tech.to_string());
            state.architecture = Some(arch.to_string());
            state.total_apis = apis;
        }
    }

    pub fn set_triage(&self, triage: TriageResult) {
        if let Ok(mut state) = self.state.lock() {
            state.triage = Some(triage);
        }
    }

    pub fn finish_session(&self) {
        if let Ok(mut state) = self.state.lock() {
            let now = now_epoch_ms();
            state.session.finished_at = Some(format_timestamp(now));
            state.session.elapsed_ms = now.saturating_sub(state.session.started_epoch_ms);
        }
    }

    pub fn refresh_host_metrics(&self, system: &System) {
        if let Ok(mut state) = self.state.lock() {
            state.session.host = HostMetrics {
                cpu_percent: system.global_cpu_usage(),
                cpu_cores: system.cpus().len(),
                memory_used_bytes: system.used_memory(),
                memory_total_bytes: system.total_memory(),
            };
            if state.session.finished_at.is_none() {
                state.session.elapsed_ms =
                    now_epoch_ms().saturating_sub(state.session.started_epoch_ms);
            }
        }
    }

    pub fn log(&self, level: &str, msg: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs() % 86400;
        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        let seconds = secs % 60;
        let time_str = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

        let entry = LogMessage {
            timestamp: time_str,
            level: level.to_string(),
            message: msg.to_string(),
        };

        if let Ok(mut state) = self.state.lock() {
            state.logs.push(entry);
            // Mantener un búfer circular de hasta 200 logs
            if state.logs.len() > 200 {
                state.logs.remove(0);
            }
        }
    }

    pub fn snapshot(&self) -> SystemState {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

pub fn start_host_monitor(state_mgr: StateManager) {
    thread::spawn(move || {
        let mut system = System::new_all();
        system.refresh_all();
        loop {
            thread::sleep(Duration::from_secs(1));
            system.refresh_cpu_usage();
            system.refresh_memory();
            state_mgr.refresh_host_metrics(&system);
        }
    });
}

pub struct SessionGuard(StateManager);

impl SessionGuard {
    pub fn new(state_mgr: StateManager) -> Self {
        Self(state_mgr)
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        self.0.finish_session();
    }
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn format_timestamp(epoch_ms: u64) -> String {
    Local
        .timestamp_millis_opt(epoch_ms as i64)
        .single()
        .map(|date| date.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "hora no disponible".to_string())
}
