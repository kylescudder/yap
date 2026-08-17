//! Per-user daemon interface for Linux clients and platform adapters.
//!
//! [`Coordinator`] is the module's interface for hotkey edges. It serializes access to the core
//! session machine, owns the monotonic clock, executes timer effects, and delegates only concrete
//! capture/pipeline work through [`PipelineRuntime`]. D-Bus and tests are thin adapters around it.

use std::{fmt, sync::Arc, time::Instant};

use async_trait::async_trait;
use thiserror::Error;
use tokio::{sync::Mutex, time::Duration};
use yap_core::{Action, Effect, Event, Phase, SessionConfig, SessionMachine};

pub mod dbus;
pub mod model;
pub mod runtime;

pub const BUS_NAME: &str = "com.yap.Yap";
pub const OBJECT_PATH: &str = "/com/yap/Yap/Dictation";
pub const INTERFACE_NAME: &str = "com.yap.Yap.Dictation1";

#[derive(Debug, Error)]
#[error("{0}")]
pub struct RuntimeError(pub String);

/// The platform work driven by session transitions.
///
/// Implementations own `PipeWire` capture and the transcription/insertion pipeline. They must not
/// contain hotkey timing or session-state policy.
#[async_trait]
pub trait PipelineRuntime: Send + Sync + 'static {
    async fn start_capture(&self, action: Action) -> Result<(), RuntimeError>;
    async fn discard_capture(&self) -> Result<(), RuntimeError>;
    async fn stop_and_process(&self, action: Action) -> Result<(), RuntimeError>;
}

trait Clock: Send + Sync + 'static {
    fn now_ms(&self) -> u64;
}

#[derive(Debug)]
struct MonotonicClock {
    origin: Instant,
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Clock for MonotonicClock {
    fn now_ms(&self) -> u64 {
        u64::try_from(self.origin.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Status {
    pub phase: Phase,
    pub last_error: Option<String>,
}

struct CoordinatorState {
    machine: SessionMachine,
    last_error: Option<String>,
}

/// Owns a single user's dictation session and turns edges into platform work.
pub struct Coordinator {
    state: Mutex<CoordinatorState>,
    runtime: Arc<dyn PipelineRuntime>,
    clock: Arc<dyn Clock>,
}

impl fmt::Debug for Coordinator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Coordinator")
            .finish_non_exhaustive()
    }
}

impl Coordinator {
    #[must_use]
    pub fn new(runtime: Arc<dyn PipelineRuntime>) -> Arc<Self> {
        Self::with_config(runtime, SessionConfig::default())
    }

    #[must_use]
    pub fn with_config(runtime: Arc<dyn PipelineRuntime>, config: SessionConfig) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(CoordinatorState {
                machine: SessionMachine::new(config),
                last_error: None,
            }),
            runtime,
            clock: Arc::new(MonotonicClock::default()),
        })
    }

    /// Applies a key edge at the daemon's current monotonic time.
    ///
    /// # Errors
    ///
    /// Returns the platform adapter's error if starting or discarding capture fails.
    pub async fn edge(
        self: &Arc<Self>,
        action: Action,
        pressed: bool,
    ) -> Result<Status, RuntimeError> {
        let at_ms = self.clock.now_ms();
        let event = if pressed {
            Event::Press { action, at_ms }
        } else {
            Event::Release { action, at_ms }
        };
        self.dispatch(event).await
    }

    /// Aborts an active capture.
    ///
    /// # Errors
    ///
    /// Returns the platform adapter's error if discarding capture fails.
    pub async fn cancel(self: &Arc<Self>) -> Result<Status, RuntimeError> {
        self.dispatch(Event::Abort).await
    }

    pub async fn status(&self) -> Status {
        let state = self.state.lock().await;
        Status {
            phase: state.machine.phase(),
            last_error: state.last_error.clone(),
        }
    }

    async fn dispatch(self: &Arc<Self>, event: Event) -> Result<Status, RuntimeError> {
        let mut state = self.state.lock().await;
        let transition = state.machine.apply(event);

        for effect in transition.effects {
            match effect {
                Effect::StartCapture { action } => {
                    if let Err(error) = self.runtime.start_capture(action).await {
                        state.last_error = Some(error.to_string());
                        state.machine.apply(Event::CaptureFailed);
                        return Err(error);
                    }
                    state.last_error = None;
                }
                Effect::DiscardCapture => {
                    if let Err(error) = self.runtime.discard_capture().await {
                        state.last_error = Some(error.to_string());
                        return Err(error);
                    }
                }
                Effect::StopAndProcess { action } => {
                    let coordinator = Arc::clone(self);
                    let runtime = Arc::clone(&self.runtime);
                    tokio::spawn(async move {
                        let result = runtime.stop_and_process(action).await;
                        coordinator.finish_pipeline(result).await;
                    });
                }
                Effect::ScheduleCancel {
                    generation,
                    delay_ms,
                } => {
                    let coordinator = Arc::downgrade(self);
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        if let Some(coordinator) = coordinator.upgrade() {
                            coordinator.expire_quick_tap(generation).await;
                        }
                    });
                }
                Effect::CancelScheduled { .. } => {
                    // Timer generations make already-spawned callbacks harmless.
                }
            }
        }

        Ok(Status {
            phase: state.machine.phase(),
            last_error: state.last_error.clone(),
        })
    }

    async fn finish_pipeline(&self, result: Result<(), RuntimeError>) {
        let mut state = self.state.lock().await;
        match result {
            Ok(()) => {
                state.last_error = None;
                state.machine.apply(Event::PipelineFinished);
            }
            Err(error) => {
                state.last_error = Some(error.to_string());
                state.machine.apply(Event::PipelineFailed);
            }
        }
    }

    async fn expire_quick_tap(&self, generation: u64) {
        let mut state = self.state.lock().await;
        let transition = state.machine.apply(Event::CancelDeadline { generation });
        if transition.effects.contains(&Effect::DiscardCapture) {
            if let Err(error) = self.runtime.discard_capture().await {
                state.last_error = Some(error.to_string());
            }
        }
    }
}

#[must_use]
pub fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Idle => "idle",
        Phase::Recording { locked: false, .. } => "recording",
        Phase::Recording { locked: true, .. } => "recording-locked",
        Phase::AwaitingSecondTap => "awaiting-second-tap",
        Phase::Processing { .. } => "processing",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct RecordingRuntime {
        calls: Mutex<Vec<String>>,
    }

    #[derive(Debug)]
    struct FailingRuntime;

    #[async_trait]
    impl PipelineRuntime for RecordingRuntime {
        async fn start_capture(&self, action: Action) -> Result<(), RuntimeError> {
            self.calls.lock().await.push(format!("start:{action:?}"));
            Ok(())
        }

        async fn discard_capture(&self) -> Result<(), RuntimeError> {
            self.calls.lock().await.push("discard".to_owned());
            Ok(())
        }

        async fn stop_and_process(&self, action: Action) -> Result<(), RuntimeError> {
            self.calls.lock().await.push(format!("process:{action:?}"));
            Ok(())
        }
    }

    #[async_trait]
    impl PipelineRuntime for FailingRuntime {
        async fn start_capture(&self, _action: Action) -> Result<(), RuntimeError> {
            Err(RuntimeError("microphone unavailable".to_owned()))
        }

        async fn discard_capture(&self) -> Result<(), RuntimeError> {
            Ok(())
        }

        async fn stop_and_process(&self, _action: Action) -> Result<(), RuntimeError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn held_edge_pair_reaches_the_runtime() {
        let runtime = Arc::new(RecordingRuntime::default());
        let coordinator = Coordinator::with_config(
            runtime.clone(),
            SessionConfig {
                hold_threshold_ms: 0,
                double_tap_window_ms: 500,
            },
        );

        coordinator
            .edge(Action::Dictation, true)
            .await
            .expect("capture starts");
        let status = coordinator
            .edge(Action::Dictation, false)
            .await
            .expect("processing starts");
        assert_eq!(
            status.phase,
            Phase::Processing {
                action: Action::Dictation
            }
        );

        tokio::task::yield_now().await;
        assert_eq!(coordinator.status().await.phase, Phase::Idle);
        assert_eq!(
            *runtime.calls.lock().await,
            ["start:Dictation", "process:Dictation"]
        );
    }

    #[tokio::test]
    async fn capture_failure_returns_to_idle_and_remains_observable() {
        let coordinator = Coordinator::new(Arc::new(FailingRuntime));

        let error = coordinator
            .edge(Action::Dictation, true)
            .await
            .expect_err("runtime rejects capture");
        let status = coordinator.status().await;

        assert_eq!(status.phase, Phase::Idle);
        assert_eq!(status.last_error.as_deref(), Some(error.0.as_str()));
    }
}
