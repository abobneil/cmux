//! XDG-backed persistence for Linux cmux state.

use crate::{
    session::{AgentKind, AppSessionState, Pane, SessionStatus, WorkspaceModel, WorkspaceSession},
    terminal::{TerminalCommand, TerminalSession},
};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};
use thiserror::Error;

pub const CURRENT_STATE_VERSION: u32 = 2;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("could not resolve an XDG state directory")]
    MissingStateDirectory,
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedState {
    #[serde(default = "current_state_version")]
    pub version: u32,
    #[serde(default)]
    pub workspaces: Vec<SavedWorkspace>,
    #[serde(default)]
    pub active_workspace_id: Option<String>,
    #[serde(default)]
    pub sessions: Vec<SavedSession>,
}

impl Default for SavedState {
    fn default() -> Self {
        Self {
            version: CURRENT_STATE_VERSION,
            workspaces: Vec::new(),
            active_workspace_id: None,
            sessions: Vec::new(),
        }
    }
}

impl SavedState {
    #[must_use]
    pub fn migrated(mut self) -> Self {
        if self.workspaces.is_empty() && !self.sessions.is_empty() {
            let active_session_id = self.sessions.first().map(|session| session.id.clone());
            let panes = active_session_id
                .as_ref()
                .map(|session_id| SavedPane {
                    id: format!("pane-{session_id}"),
                    session_id: session_id.clone(),
                })
                .into_iter()
                .collect();

            self.workspaces.push(SavedWorkspace {
                id: "default".to_string(),
                title: "Default".to_string(),
                sessions: std::mem::take(&mut self.sessions),
                panes,
                active_session_id,
            });
            self.active_workspace_id = Some("default".to_string());
        }
        self.version = CURRENT_STATE_VERSION;
        self
    }
}

fn current_state_version() -> u32 {
    CURRENT_STATE_VERSION
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedWorkspace {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub sessions: Vec<SavedSession>,
    #[serde(default)]
    pub panes: Vec<SavedPane>,
    #[serde(default)]
    pub active_session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedPane {
    pub id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSession {
    pub id: String,
    pub title: String,
    pub program: String,
    pub args: Vec<String>,
    pub working_directory: Option<PathBuf>,
    #[serde(default)]
    pub agent: SavedAgentKind,
    #[serde(default)]
    pub status: SavedSessionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum SavedAgentKind {
    #[default]
    Shell,
    Claude,
    Codex,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", content = "code", rename_all = "snake_case")]
pub enum SavedSessionStatus {
    #[default]
    Running,
    WaitingForInput,
    Exited(i32),
}

impl From<&TerminalSession> for SavedSession {
    fn from(session: &TerminalSession) -> Self {
        Self {
            id: session.id.clone(),
            title: session.title.clone(),
            program: session.command.program.clone(),
            args: session.command.args.clone(),
            working_directory: session.command.working_directory.clone(),
            agent: SavedAgentKind::Shell,
            status: SavedSessionStatus::Running,
        }
    }
}

impl From<&WorkspaceSession> for SavedSession {
    fn from(session: &WorkspaceSession) -> Self {
        Self {
            id: session.terminal.id.clone(),
            title: session.terminal.title.clone(),
            program: session.terminal.command.program.clone(),
            args: session.terminal.command.args.clone(),
            working_directory: session.terminal.command.working_directory.clone(),
            agent: (&session.agent).into(),
            status: (&session.status).into(),
        }
    }
}

impl From<SavedSession> for WorkspaceSession {
    fn from(session: SavedSession) -> Self {
        let status = session.status.into();
        let mut workspace_session = WorkspaceSession::with_command(
            session.id,
            session.title,
            session.agent.into(),
            TerminalCommand {
                program: session.program,
                args: session.args,
                working_directory: session.working_directory,
            },
        );
        workspace_session.status = status;
        workspace_session
    }
}

impl From<&WorkspaceModel> for SavedWorkspace {
    fn from(workspace: &WorkspaceModel) -> Self {
        Self {
            id: workspace.id.clone(),
            title: workspace.title.clone(),
            sessions: workspace
                .sessions()
                .iter()
                .map(SavedSession::from)
                .collect(),
            panes: workspace
                .panes()
                .iter()
                .map(|pane| SavedPane {
                    id: pane.id.clone(),
                    session_id: pane.session_id.clone(),
                })
                .collect(),
            active_session_id: workspace.active_session_id().map(ToString::to_string),
        }
    }
}

impl From<SavedWorkspace> for WorkspaceModel {
    fn from(workspace: SavedWorkspace) -> Self {
        let mut model = WorkspaceModel::new(workspace.id, workspace.title);
        for session in workspace.sessions {
            model.push_session(session.into());
        }
        if let Some(active_session_id) = workspace.active_session_id {
            model.set_active_session(&active_session_id);
        }
        for pane in workspace.panes {
            if !model.panes().iter().any(|existing| existing.id == pane.id) {
                model.push_pane(Pane::new(pane.id, pane.session_id));
            }
        }
        model
    }
}

impl From<&AppSessionState> for SavedState {
    fn from(state: &AppSessionState) -> Self {
        Self {
            version: CURRENT_STATE_VERSION,
            workspaces: state
                .workspaces()
                .iter()
                .map(SavedWorkspace::from)
                .collect(),
            active_workspace_id: state.active_workspace_id().map(ToString::to_string),
            sessions: Vec::new(),
        }
    }
}

impl From<SavedState> for AppSessionState {
    fn from(state: SavedState) -> Self {
        let mut app_state = AppSessionState::new();
        let state = state.migrated();
        for workspace in state.workspaces {
            app_state.push_workspace(workspace.into());
        }
        if let Some(active_workspace_id) = state.active_workspace_id {
            app_state.set_active_workspace(&active_workspace_id);
        }
        app_state
    }
}

impl From<&AgentKind> for SavedAgentKind {
    fn from(agent: &AgentKind) -> Self {
        match agent {
            AgentKind::Shell => Self::Shell,
            AgentKind::Claude => Self::Claude,
            AgentKind::Codex => Self::Codex,
            AgentKind::Custom(value) => Self::Custom(value.clone()),
        }
    }
}

impl From<SavedAgentKind> for AgentKind {
    fn from(agent: SavedAgentKind) -> Self {
        match agent {
            SavedAgentKind::Shell => Self::Shell,
            SavedAgentKind::Claude => Self::Claude,
            SavedAgentKind::Codex => Self::Codex,
            SavedAgentKind::Custom(value) => Self::Custom(value),
        }
    }
}

impl From<&SessionStatus> for SavedSessionStatus {
    fn from(status: &SessionStatus) -> Self {
        match status {
            SessionStatus::Running => Self::Running,
            SessionStatus::WaitingForInput => Self::WaitingForInput,
            SessionStatus::Exited(code) => Self::Exited(*code),
        }
    }
}

impl From<SavedSessionStatus> for SessionStatus {
    fn from(status: SavedSessionStatus) -> Self {
        match status {
            SavedSessionStatus::Running => Self::Running,
            SavedSessionStatus::WaitingForInput => Self::WaitingForInput,
            SavedSessionStatus::Exited(code) => Self::Exited(code),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StateStore {
    path: PathBuf,
}

impl StateStore {
    /// Create a store using the current user's XDG state directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the XDG state directory cannot be resolved.
    pub fn xdg() -> Result<Self, StorageError> {
        let state_dir = dirs::state_dir().ok_or(StorageError::MissingStateDirectory)?;
        Ok(Self {
            path: state_dir.join("cmux").join("state.json"),
        })
    }

    #[must_use]
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Load saved state from disk and migrate it to the current shape.
    ///
    /// # Errors
    ///
    /// Returns an error if the state file cannot be read or decoded.
    pub fn load(&self) -> Result<SavedState, StorageError> {
        if !self.path.exists() {
            return Ok(SavedState::default());
        }
        let bytes = fs::read(&self.path)?;
        Ok(serde_json::from_slice::<SavedState>(&bytes)?.migrated())
    }

    /// Load saved state, recovering corrupt JSON by renaming it aside.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or a corrupt file cannot be
    /// renamed out of the way.
    pub fn load_or_recover(&self) -> Result<SavedState, StorageError> {
        match self.load() {
            Ok(state) => Ok(state),
            Err(StorageError::Json(_)) => {
                let corrupt_path = self.path.with_extension("json.corrupt");
                fs::rename(&self.path, corrupt_path)?;
                Ok(SavedState::default())
            }
            Err(error) => Err(error),
        }
    }

    /// Save state to disk atomically, creating parent directories as needed.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created, the state cannot be
    /// encoded, or the file cannot be written/renamed.
    pub fn save(&self, state: &SavedState) -> Result<(), StorageError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let state = state.clone().migrated();
        let bytes = serde_json::to_vec_pretty(&state)?;
        let tmp_path = self.path.with_extension("json.tmp");
        fs::write(&tmp_path, bytes)?;
        fs::rename(tmp_path, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "cmux-{name}-{}-{}.json",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ))
    }

    #[test]
    fn missing_state_defaults_empty() {
        let store = StateStore::at(test_path("missing"));
        assert_eq!(store.load().unwrap(), SavedState::default());
    }

    #[test]
    fn legacy_sessions_default_to_shell_running() {
        let json = r#"{
            "sessions": [{
                "id": "one",
                "title": "One",
                "program": "/bin/sh",
                "args": [],
                "working_directory": null
            }]
        }"#;

        let state: SavedState = serde_json::from_str(json).unwrap();
        let state = state.migrated();
        let session = &state.workspaces[0].sessions[0];
        assert_eq!(session.agent, SavedAgentKind::Shell);
        assert_eq!(session.status, SavedSessionStatus::Running);
        assert_eq!(state.version, CURRENT_STATE_VERSION);
        assert_eq!(state.active_workspace_id.as_deref(), Some("default"));
    }

    #[test]
    fn workspace_session_round_trips_agent_metadata() {
        let session = WorkspaceSession::with_command(
            "one",
            "Claude",
            AgentKind::Claude,
            TerminalCommand {
                program: "claude".to_string(),
                args: vec!["--dangerously-skip-permissions".to_string()],
                working_directory: None,
            },
        );

        let saved = SavedSession::from(&session);
        let restored = WorkspaceSession::from(saved);
        assert_eq!(restored.agent, AgentKind::Claude);
        assert_eq!(restored.terminal.command.program, "claude");
    }

    #[test]
    fn app_state_round_trips_workspaces_sessions_and_active_ids() {
        let mut workspace = WorkspaceModel::new("workspace-1", "Workspace 1");
        workspace.push_session(WorkspaceSession::shell("session-1", "Shell"));
        workspace.push_session(WorkspaceSession::with_command(
            "session-2",
            "Codex",
            AgentKind::Codex,
            TerminalCommand {
                program: "codex".to_string(),
                args: vec!["--ask-for-approval=never".to_string()],
                working_directory: None,
            },
        ));
        assert!(workspace.set_active_session("session-2"));

        let mut app_state = AppSessionState::new();
        app_state.push_workspace(workspace);

        let saved = SavedState::from(&app_state);
        let restored = AppSessionState::from(saved);
        let restored_workspace = restored.active_workspace().unwrap();

        assert_eq!(restored_workspace.id, "workspace-1");
        assert_eq!(restored_workspace.active_session_id(), Some("session-2"));
        assert_eq!(restored_workspace.sessions()[1].agent, AgentKind::Codex);
    }

    #[test]
    fn save_writes_versioned_state_atomically() {
        let path = test_path("save");
        let store = StateStore::at(&path);
        let state = SavedState {
            sessions: vec![SavedSession {
                id: "one".to_string(),
                title: "One".to_string(),
                program: "/bin/sh".to_string(),
                args: Vec::new(),
                working_directory: None,
                agent: SavedAgentKind::Shell,
                status: SavedSessionStatus::Running,
            }],
            ..SavedState::default()
        };

        store.save(&state).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.version, CURRENT_STATE_VERSION);
        assert!(loaded.sessions.is_empty());
        assert_eq!(loaded.workspaces[0].sessions[0].id, "one");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn corrupt_state_is_renamed_and_defaults_empty() {
        let path = test_path("corrupt");
        fs::write(&path, b"not json").unwrap();
        let store = StateStore::at(&path);

        let recovered = store.load_or_recover().unwrap();

        assert_eq!(recovered, SavedState::default());
        assert!(!path.exists());
        assert!(path.with_extension("json.corrupt").exists());
        let _ = fs::remove_file(path.with_extension("json.corrupt"));
    }
}
