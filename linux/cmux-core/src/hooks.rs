//! Agent hook/status detection shared by Linux frontends.

use crate::session::{AgentKind, SessionStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentContext {
    pub workspace_id: String,
    pub workspace_title: String,
    pub session_id: String,
    pub session_title: String,
    pub agent: AgentKind,
}

impl AgentContext {
    #[must_use]
    pub fn new(
        workspace_id: impl Into<String>,
        workspace_title: impl Into<String>,
        session_id: impl Into<String>,
        session_title: impl Into<String>,
        agent: AgentKind,
    ) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            workspace_title: workspace_title.into(),
            session_id: session_id.into(),
            session_title: session_title.into(),
            agent,
        }
    }

    #[must_use]
    pub fn agent_name(&self) -> &'static str {
        agent_name(&self.agent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentStatusEvent {
    pub agent: AgentKind,
    pub status: SessionStatus,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentNotificationDecision {
    pub workspace_id: String,
    pub workspace_title: String,
    pub session_id: String,
    pub session_title: String,
    pub agent_name: String,
    pub title: String,
    pub body: String,
}

#[must_use]
pub fn detect_status(agent: &AgentKind, output: &str) -> Option<AgentStatusEvent> {
    let normalized = output.to_ascii_lowercase();
    if contains_waiting_signal(&normalized) {
        Some(AgentStatusEvent {
            agent: agent.clone(),
            status: SessionStatus::WaitingForInput,
            reason: "agent output indicates it is waiting for user input".to_string(),
        })
    } else {
        None
    }
}

#[must_use]
pub fn notification_for_status(
    context: &AgentContext,
    event: &AgentStatusEvent,
) -> Option<AgentNotificationDecision> {
    if event.status != SessionStatus::WaitingForInput {
        return None;
    }

    let agent_name = context.agent_name();
    Some(AgentNotificationDecision {
        workspace_id: context.workspace_id.clone(),
        workspace_title: context.workspace_title.clone(),
        session_id: context.session_id.clone(),
        session_title: context.session_title.clone(),
        agent_name: agent_name.to_string(),
        title: format!("{agent_name} needs input"),
        body: format!(
            "{} in {} is waiting for your response.",
            context.session_title, context.workspace_title
        ),
    })
}

#[must_use]
pub fn detect_notification(
    context: &AgentContext,
    output: &str,
) -> Option<AgentNotificationDecision> {
    let event = detect_status(&context.agent, output)?;
    notification_for_status(context, &event)
}

#[must_use]
pub fn agent_name(agent: &AgentKind) -> &'static str {
    match agent {
        AgentKind::Shell => "Shell",
        AgentKind::Claude => "Claude",
        AgentKind::Codex => "Codex",
        AgentKind::Custom(_) => "Agent",
    }
}

fn contains_waiting_signal(output: &str) -> bool {
    [
        "waiting for input",
        "needs input",
        "requires your input",
        "please respond",
        "do you want to proceed",
        "approve or deny",
        "waiting for your response",
    ]
    .iter()
    .any(|signal| output.contains(signal))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_waiting_for_input_phrases() {
        let event = detect_status(&AgentKind::Claude, "Claude needs input before continuing")
            .expect("status event");

        assert_eq!(event.agent, AgentKind::Claude);
        assert_eq!(event.status, SessionStatus::WaitingForInput);
    }

    #[test]
    fn ignores_regular_output() {
        assert_eq!(detect_status(&AgentKind::Codex, "compiling crate"), None);
    }

    #[test]
    fn waiting_status_builds_contextual_notification_decision() {
        let context = AgentContext::new(
            "workspace-1",
            "API refactor",
            "session-1",
            "Claude plan",
            AgentKind::Claude,
        );
        let event = AgentStatusEvent {
            agent: AgentKind::Claude,
            status: SessionStatus::WaitingForInput,
            reason: "test".to_string(),
        };

        let decision = notification_for_status(&context, &event).expect("notification decision");

        assert_eq!(decision.workspace_id, "workspace-1");
        assert_eq!(decision.session_id, "session-1");
        assert_eq!(decision.title, "Claude needs input");
        assert!(decision.body.contains("Claude plan"));
        assert!(decision.body.contains("API refactor"));
    }

    #[test]
    fn non_waiting_status_does_not_notify() {
        let context = AgentContext::new(
            "workspace-1",
            "API refactor",
            "session-1",
            "Codex tests",
            AgentKind::Codex,
        );
        let event = AgentStatusEvent {
            agent: AgentKind::Codex,
            status: SessionStatus::Running,
            reason: "still running".to_string(),
        };

        assert_eq!(notification_for_status(&context, &event), None);
    }

    #[test]
    fn detect_notification_combines_detection_and_decision() {
        let context = AgentContext::new(
            "workspace-2",
            "Checkout",
            "session-9",
            "Codex",
            AgentKind::Codex,
        );

        let decision = detect_notification(&context, "Approve or deny this command?")
            .expect("notification decision");

        assert_eq!(decision.agent_name, "Codex");
        assert_eq!(decision.workspace_title, "Checkout");
    }
}
