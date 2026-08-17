use std::sync::Arc;

use yap_core::Action;
use zbus::{connection, fdo, object_server::SignalEmitter};

use crate::{
    BUS_NAME, Coordinator, INTERFACE_NAME, OBJECT_PATH, PipelineRuntime, Status,
    action_name as phase_action_name, is_locked, phase_name,
};

pub struct DictationInterface {
    coordinator: Arc<Coordinator>,
}

impl DictationInterface {
    #[must_use]
    pub fn new(coordinator: Arc<Coordinator>) -> Self {
        Self { coordinator }
    }
}

#[zbus::interface(name = "com.yap.Yap.Dictation1")]
impl DictationInterface {
    async fn edge(&self, action: &str, pressed: bool) -> fdo::Result<String> {
        let action = parse_action(action)?;
        self.coordinator
            .edge(action, pressed)
            .await
            .map(|status| phase_name(status.phase).to_owned())
            .map_err(|error| fdo::Error::Failed(error.to_string()))
    }

    async fn cancel(&self) -> fdo::Result<String> {
        self.coordinator
            .cancel()
            .await
            .map(|status| phase_name(status.phase).to_owned())
            .map_err(|error| fdo::Error::Failed(error.to_string()))
    }

    async fn status(&self) -> (String, String) {
        let status = self.coordinator.status().await;
        (
            phase_name(status.phase).to_owned(),
            status.last_error.unwrap_or_default(),
        )
    }

    async fn state(&self) -> (String, String, bool, String) {
        state_fields(&self.coordinator.status().await)
    }

    #[zbus(signal)]
    async fn state_changed(
        emitter: &SignalEmitter<'_>,
        phase: &str,
        action: &str,
        locked: bool,
        last_error: &str,
    ) -> zbus::Result<()>;
}

fn state_fields(status: &Status) -> (String, String, bool, String) {
    (
        phase_name(status.phase).to_owned(),
        phase_action_name(status.phase).to_owned(),
        is_locked(status.phase),
        status.last_error.clone().unwrap_or_default(),
    )
}

fn parse_action(value: &str) -> fdo::Result<Action> {
    match value {
        "dictation" => Ok(Action::Dictation),
        "command" => Ok(Action::Command),
        _ => Err(fdo::Error::InvalidArgs(format!(
            "unknown action {value:?}; expected 'dictation' or 'command'"
        ))),
    }
}

/// Owns the per-user bus name and serves requests until the process receives an interrupt.
///
/// # Errors
///
/// Returns an error if the session bus cannot be reached, the name or path cannot be registered,
/// or the interrupt listener cannot be installed.
pub async fn serve(runtime: Arc<dyn PipelineRuntime>) -> zbus::Result<()> {
    let coordinator = Coordinator::new(runtime);
    let mut statuses = coordinator.subscribe();
    let connection = connection::Builder::session()?
        .name(BUS_NAME)?
        .serve_at(OBJECT_PATH, DictationInterface::new(coordinator))?
        .build()
        .await?;
    let interface = connection
        .object_server()
        .interface::<_, DictationInterface>(OBJECT_PATH)
        .await?;
    let emitter = interface.signal_emitter().clone();
    tokio::spawn(async move {
        while statuses.changed().await.is_ok() {
            let status = statuses.borrow().clone();
            let (phase, action, locked, last_error) = state_fields(&status);
            if let Err(error) = DictationInterface::state_changed(
                &emitter,
                &phase,
                &action,
                locked,
                &last_error,
            )
            .await
            {
                eprintln!("yapd: could not publish state on {INTERFACE_NAME}: {error}");
            }
        }
    });

    tokio::signal::ctrl_c().await?;
    Ok(())
}

#[zbus::proxy(
    interface = "com.yap.Yap.Dictation1",
    default_service = "com.yap.Yap",
    default_path = "/com/yap/Yap/Dictation"
)]
trait Dictation {
    async fn edge(&self, action: &str, pressed: bool) -> zbus::Result<String>;
    async fn cancel(&self) -> zbus::Result<String>;
    async fn status(&self) -> zbus::Result<(String, String)>;
    async fn state(&self) -> zbus::Result<(String, String, bool, String)>;
}

pub struct Client {
    connection: zbus::Connection,
}

impl Client {
    /// Connects a control client to the user's session bus.
    ///
    /// # Errors
    ///
    /// Returns an error when no session bus is available or the bus rejects the connection.
    pub async fn connect() -> zbus::Result<Self> {
        Ok(Self {
            connection: zbus::Connection::session().await?,
        })
    }

    /// Sends one press or release edge to the daemon.
    ///
    /// # Errors
    ///
    /// Returns a D-Bus transport error or the runtime error returned by the daemon.
    pub async fn edge(&self, action: Action, pressed: bool) -> zbus::Result<String> {
        DictationProxy::new(&self.connection)
            .await?
            .edge(action_name(action), pressed)
            .await
    }

    /// Requests cancellation of the active capture.
    ///
    /// # Errors
    ///
    /// Returns a D-Bus transport error or the runtime error returned by the daemon.
    pub async fn cancel(&self) -> zbus::Result<String> {
        DictationProxy::new(&self.connection).await?.cancel().await
    }

    /// Reads the current phase and most recent runtime error.
    ///
    /// # Errors
    ///
    /// Returns a D-Bus transport error if the daemon cannot be reached.
    pub async fn status(&self) -> zbus::Result<(String, String)> {
        DictationProxy::new(&self.connection).await?.status().await
    }

    /// Reads the complete state used by desktop visual adapters.
    ///
    /// # Errors
    ///
    /// Returns a D-Bus transport error if the daemon cannot be reached.
    pub async fn state(&self) -> zbus::Result<(String, String, bool, String)> {
        DictationProxy::new(&self.connection).await?.state().await
    }
}

fn action_name(action: Action) -> &'static str {
    match action {
        Action::Dictation => "dictation",
        Action::Command => "command",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yap_core::Phase;

    #[test]
    fn recording_state_exposes_action_and_lock() {
        let fields = state_fields(&Status {
            phase: Phase::Recording {
                action: Action::Command,
                locked: true,
            },
            last_error: None,
        });

        assert_eq!(
            fields,
            (
                "recording-locked".to_owned(),
                "command".to_owned(),
                true,
                String::new(),
            )
        );
    }
}
