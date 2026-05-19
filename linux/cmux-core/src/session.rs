//! Workspace/session domain model for the Linux port.

use crate::terminal::{TerminalCommand, TerminalSession};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AgentKind {
    #[default]
    Shell,
    Claude,
    Codex,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCommand {
    pub kind: AgentKind,
    pub label: String,
    pub command: TerminalCommand,
}

impl AgentCommand {
    #[must_use]
    pub fn new(kind: AgentKind, label: impl Into<String>, command: TerminalCommand) -> Self {
        Self {
            kind,
            label: label.into(),
            command,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SessionStatus {
    #[default]
    Running,
    WaitingForInput,
    Exited(i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSession {
    pub terminal: TerminalSession,
    pub agent: AgentKind,
    pub status: SessionStatus,
}

impl WorkspaceSession {
    #[must_use]
    pub fn shell(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            terminal: TerminalSession::new(id, title, TerminalCommand::user_shell()),
            agent: AgentKind::Shell,
            status: SessionStatus::Running,
        }
    }

    #[must_use]
    pub fn with_command(
        id: impl Into<String>,
        title: impl Into<String>,
        agent: AgentKind,
        command: TerminalCommand,
    ) -> Self {
        Self {
            terminal: TerminalSession::new(id, title, command),
            agent,
            status: SessionStatus::Running,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pane {
    pub id: String,
    pub session_id: String,
}

impl Pane {
    #[must_use]
    pub fn new(id: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            session_id: session_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceModel {
    pub id: String,
    pub title: String,
    sessions: Vec<WorkspaceSession>,
    panes: Vec<Pane>,
    active_session_id: Option<String>,
}

impl WorkspaceModel {
    #[must_use]
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            sessions: Vec::new(),
            panes: Vec::new(),
            active_session_id: None,
        }
    }

    pub fn push_session(&mut self, session: WorkspaceSession) {
        let session_id = session.terminal.id.clone();
        if self.active_session_id.is_none() {
            self.active_session_id = Some(session_id.clone());
        }
        if self.panes.is_empty() {
            self.panes
                .push(Pane::new(format!("pane-{session_id}"), session_id));
        }
        self.sessions.push(session);
    }

    pub fn set_active_session(&mut self, id: &str) -> bool {
        if self
            .sessions
            .iter()
            .any(|session| session.terminal.id == id)
        {
            self.active_session_id = Some(id.to_string());
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn active_session(&self) -> Option<&WorkspaceSession> {
        let active_id = self.active_session_id.as_ref()?;
        self.sessions
            .iter()
            .find(|session| &session.terminal.id == active_id)
    }

    #[must_use]
    pub fn sessions(&self) -> &[WorkspaceSession] {
        &self.sessions
    }

    pub fn push_pane(&mut self, pane: Pane) {
        self.panes.push(pane);
    }

    #[must_use]
    pub fn panes(&self) -> &[Pane] {
        &self.panes
    }

    #[must_use]
    pub fn active_session_id(&self) -> Option<&str> {
        self.active_session_id.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionList {
    sessions: Vec<WorkspaceSession>,
    active_id: Option<String>,
}

impl SessionList {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, session: WorkspaceSession) {
        if self.active_id.is_none() {
            self.active_id = Some(session.terminal.id.clone());
        }
        self.sessions.push(session);
    }

    #[must_use]
    pub fn active(&self) -> Option<&WorkspaceSession> {
        let active_id = self.active_id.as_ref()?;
        self.sessions
            .iter()
            .find(|session| &session.terminal.id == active_id)
    }

    pub fn set_active(&mut self, id: &str) -> bool {
        if self
            .sessions
            .iter()
            .any(|session| session.terminal.id == id)
        {
            self.active_id = Some(id.to_string());
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn sessions(&self) -> &[WorkspaceSession] {
        &self.sessions
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppSessionState {
    workspaces: Vec<WorkspaceModel>,
    active_workspace_id: Option<String>,
}

impl AppSessionState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_workspace(&mut self, workspace: WorkspaceModel) {
        if self.active_workspace_id.is_none() {
            self.active_workspace_id = Some(workspace.id.clone());
        }
        self.workspaces.push(workspace);
    }

    pub fn set_active_workspace(&mut self, id: &str) -> bool {
        if self.workspaces.iter().any(|workspace| workspace.id == id) {
            self.active_workspace_id = Some(id.to_string());
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn active_workspace(&self) -> Option<&WorkspaceModel> {
        let active_id = self.active_workspace_id.as_ref()?;
        self.workspaces
            .iter()
            .find(|workspace| &workspace.id == active_id)
    }

    #[must_use]
    pub fn workspaces(&self) -> &[WorkspaceModel] {
        &self.workspaces
    }

    #[must_use]
    pub fn active_workspace_id(&self) -> Option<&str> {
        self.active_workspace_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_pushed_session_becomes_active() {
        let mut list = SessionList::new();
        list.push(WorkspaceSession::shell("one", "One"));
        list.push(WorkspaceSession::shell("two", "Two"));

        assert_eq!(list.active().unwrap().terminal.id, "one");
    }

    #[test]
    fn active_session_can_be_changed() {
        let mut list = SessionList::new();
        list.push(WorkspaceSession::shell("one", "One"));
        list.push(WorkspaceSession::shell("two", "Two"));

        assert!(list.set_active("two"));
        assert_eq!(list.active().unwrap().terminal.title, "Two");
        assert!(!list.set_active("missing"));
    }

    #[test]
    fn workspace_tracks_sessions_panes_and_active_session() {
        let mut workspace = WorkspaceModel::new("workspace-1", "Workspace 1");
        workspace.push_session(WorkspaceSession::shell("session-1", "Shell"));
        workspace.push_session(WorkspaceSession::shell("session-2", "Build"));

        assert_eq!(workspace.sessions().len(), 2);
        assert_eq!(workspace.panes().len(), 1);
        assert_eq!(workspace.active_session().unwrap().terminal.id, "session-1");
        assert!(workspace.set_active_session("session-2"));
        assert_eq!(workspace.active_session_id(), Some("session-2"));
        assert!(!workspace.set_active_session("missing"));
    }

    #[test]
    fn app_state_tracks_active_workspace() {
        let mut state = AppSessionState::new();
        state.push_workspace(WorkspaceModel::new("one", "One"));
        state.push_workspace(WorkspaceModel::new("two", "Two"));

        assert_eq!(state.active_workspace_id(), Some("one"));
        assert!(state.set_active_workspace("two"));
        assert_eq!(state.active_workspace().unwrap().title, "Two");
        assert!(!state.set_active_workspace("missing"));
    }
}
