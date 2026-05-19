#![allow(dead_code)]

//! `FreeDesktop` notification integration for Linux desktops.

use cmux_core::hooks::AgentNotificationDecision;
use notify_rust::{Notification, NotificationHandle};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentNotification {
    pub title: String,
    pub body: String,
    pub workspace_id: Option<String>,
    pub session_id: Option<String>,
}

impl AgentNotification {
    #[must_use]
    pub fn waiting_for_input(agent_name: &str, workspace_title: &str) -> Self {
        Self {
            title: format!("{agent_name} needs input"),
            body: format!("{workspace_title} is waiting for your response."),
            workspace_id: None,
            session_id: None,
        }
    }

    #[must_use]
    pub fn waiting_for_input_in_workspace(
        agent_name: &str,
        workspace_id: impl Into<String>,
        workspace_title: &str,
    ) -> Self {
        Self {
            workspace_id: Some(workspace_id.into()),
            ..Self::waiting_for_input(agent_name, workspace_title)
        }
    }
}

impl From<AgentNotificationDecision> for AgentNotification {
    fn from(decision: AgentNotificationDecision) -> Self {
        Self {
            title: decision.title,
            body: decision.body,
            workspace_id: Some(decision.workspace_id),
            session_id: Some(decision.session_id),
        }
    }
}

pub trait Notifier {
    type Handle;
    type Error;

    fn notify(&self, notification: &AgentNotification) -> Result<Self::Handle, Self::Error>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FreedesktopNotifier;

impl Notifier for FreedesktopNotifier {
    type Handle = NotificationHandle;
    type Error = notify_rust::error::Error;

    fn notify(&self, notification: &AgentNotification) -> Result<Self::Handle, Self::Error> {
        let mut desktop_notification = Notification::new();
        desktop_notification
            .appname("cmux")
            .summary(&notification.title)
            .body(&notification.body)
            .icon("cmux");

        if let Some(workspace_id) = &notification.workspace_id {
            desktop_notification.hint(notify_rust::Hint::Custom(
                "x-cmux-workspace-id".to_string(),
                workspace_id.clone(),
            ));
        }
        if let Some(session_id) = &notification.session_id {
            desktop_notification.hint(notify_rust::Hint::Custom(
                "x-cmux-session-id".to_string(),
                session_id.clone(),
            ));
        }

        desktop_notification.show()
    }
}

pub fn notify_agent_decision<N: Notifier>(
    notifier: &N,
    decision: AgentNotificationDecision,
) -> Result<N::Handle, N::Error> {
    notifier.notify(&decision.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingNotifier {
        notifications: std::cell::RefCell<Vec<AgentNotification>>,
    }

    impl Notifier for RecordingNotifier {
        type Handle = ();
        type Error = std::convert::Infallible;

        fn notify(&self, notification: &AgentNotification) -> Result<Self::Handle, Self::Error> {
            self.notifications.borrow_mut().push(notification.clone());
            Ok(())
        }
    }

    #[test]
    fn waiting_notification_includes_context() {
        let notification = AgentNotification::waiting_for_input("Claude", "api refactor");
        assert_eq!(notification.title, "Claude needs input");
        assert!(notification.body.contains("api refactor"));
    }

    #[test]
    fn waiting_notification_can_include_workspace_id() {
        let notification =
            AgentNotification::waiting_for_input_in_workspace("Codex", "workspace-1", "tests");
        assert_eq!(notification.workspace_id.as_deref(), Some("workspace-1"));
    }

    #[test]
    fn notification_decision_is_sent_through_notifier() {
        let notifier = RecordingNotifier::default();
        let decision = AgentNotificationDecision {
            workspace_id: "workspace-1".to_string(),
            workspace_title: "API".to_string(),
            session_id: "session-1".to_string(),
            session_title: "Claude".to_string(),
            agent_name: "Claude".to_string(),
            title: "Claude needs input".to_string(),
            body: "Claude in API is waiting for your response.".to_string(),
        };

        notify_agent_decision(&notifier, decision).unwrap();

        let notifications = notifier.notifications.borrow();
        assert_eq!(notifications.len(), 1);
        assert_eq!(
            notifications[0].workspace_id.as_deref(),
            Some("workspace-1")
        );
        assert_eq!(notifications[0].session_id.as_deref(), Some("session-1"));
    }
}
