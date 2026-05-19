mod notifications;
mod terminal;

use std::{cell::RefCell, rc::Rc};

use adw::prelude::*;
use cmux_core::{
    agent::AgentCommand,
    session::WorkspaceSession,
    storage::{SavedSession, SavedState, StateStore},
    APP_ID,
};
use gtk::{gio, glib};

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &adw::Application) {
    let header = adw::HeaderBar::new();
    let add_button = gtk::MenuButton::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text("New session")
        .build();
    let add_menu = gio::Menu::new();
    add_menu.append(Some("Shell"), Some("app.new-shell"));
    add_menu.append(Some("Claude"), Some("app.new-claude"));
    add_menu.append(Some("Codex"), Some("app.new-codex"));
    add_button.set_menu_model(Some(&add_menu));
    header.pack_start(&add_button);

    let sidebar = gtk::ListBox::new();
    sidebar.add_css_class("navigation-sidebar");
    sidebar.set_selection_mode(gtk::SelectionMode::Single);
    sidebar.set_size_request(220, -1);

    let terminal_stack = gtk::Stack::builder()
        .hexpand(true)
        .vexpand(true)
        .transition_type(gtk::StackTransitionType::Crossfade)
        .build();

    let store = StateStore::xdg().ok();
    let initial_sessions = load_sessions(store.as_ref());
    let sessions = Rc::new(RefCell::new(Vec::<WorkspaceSession>::new()));

    for session in initial_sessions {
        append_session(&sidebar, &terminal_stack, &sessions, session);
    }
    sidebar.select_row(sidebar.row_at_index(0).as_ref());

    {
        let terminal_stack = terminal_stack.clone();
        let sessions = Rc::clone(&sessions);
        sidebar.connect_row_selected(move |_, row| {
            let Some(row) = row else {
                return;
            };
            let index = usize::try_from(row.index()).expect("GTK row indexes are non-negative");
            if let Some(session) = sessions.borrow().get(index) {
                terminal_stack.set_visible_child_name(&session.terminal.id);
            }
        });
    }

    install_new_session_action(
        app,
        "new-shell",
        AgentCommand::shell,
        &sidebar,
        &terminal_stack,
        &sessions,
    );
    install_new_session_action(
        app,
        "new-claude",
        || AgentCommand::claude(None),
        &sidebar,
        &terminal_stack,
        &sessions,
    );
    install_new_session_action(
        app,
        "new-codex",
        || AgentCommand::codex(None),
        &sidebar,
        &terminal_stack,
        &sessions,
    );

    let split = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(&sidebar)
        .end_child(&terminal_stack)
        .resize_start_child(false)
        .shrink_start_child(false)
        .build();

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&split));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("cmux")
        .default_width(1200)
        .default_height(800)
        .content(&toolbar)
        .build();

    if let Some(store) = store {
        let sessions = Rc::clone(&sessions);
        window.connect_close_request(move |_| {
            let state = SavedState {
                sessions: sessions.borrow().iter().map(SavedSession::from).collect(),
                ..SavedState::default()
            };
            if let Err(error) = store.save(&state) {
                eprintln!("failed to save cmux state: {error}");
            }
            glib::Propagation::Proceed
        });
    }

    window.present();
}

fn install_new_session_action(
    app: &adw::Application,
    name: &str,
    command: impl Fn() -> AgentCommand + 'static,
    sidebar: &gtk::ListBox,
    terminal_stack: &gtk::Stack,
    sessions: &Rc<RefCell<Vec<WorkspaceSession>>>,
) {
    let sidebar = sidebar.clone();
    let terminal_stack = terminal_stack.clone();
    let sessions = Rc::clone(sessions);
    let action = gio::SimpleAction::new(name, None);
    action.connect_activate(move |_, _| {
        let next = sessions.borrow().len() + 1;
        let command = command();
        let session = WorkspaceSession::with_command(
            format!("session-{next}"),
            command.title,
            command.kind,
            command.command,
        );
        append_session(&sidebar, &terminal_stack, &sessions, session);
        let last_index =
            i32::try_from(sessions.borrow().len() - 1).expect("session count fits i32");
        sidebar.select_row(sidebar.row_at_index(last_index).as_ref());
    });
    app.add_action(&action);
}

fn load_sessions(store: Option<&StateStore>) -> Vec<WorkspaceSession> {
    let sessions: Vec<WorkspaceSession> = store
        .and_then(|store| match store.load_or_recover() {
            Ok(state) => Some(
                state
                    .migrated()
                    .workspaces
                    .into_iter()
                    .flat_map(|workspace| workspace.sessions)
                    .map(WorkspaceSession::from)
                    .collect(),
            ),
            Err(error) => {
                eprintln!("failed to load cmux state: {error}");
                None
            }
        })
        .unwrap_or_default();

    if sessions.is_empty() {
        vec![WorkspaceSession::shell("session-1", "Session 1")]
    } else {
        sessions
    }
}

fn append_session(
    sidebar: &gtk::ListBox,
    terminal_stack: &gtk::Stack,
    sessions: &Rc<RefCell<Vec<WorkspaceSession>>>,
    session: WorkspaceSession,
) {
    let row = gtk::ListBoxRow::new();
    row.set_child(Some(
        &gtk::Label::builder()
            .label(&session.terminal.title)
            .xalign(0.0)
            .build(),
    ));
    sidebar.append(&row);

    let terminal = terminal::terminal(&session.terminal);
    terminal_stack.add_titled(&terminal, Some(&session.terminal.id), &session.terminal.title);
    sessions.borrow_mut().push(session);
}
