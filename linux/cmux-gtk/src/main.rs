mod notifications;
mod terminal;

use std::{cell::Cell, rc::Rc};

use adw::prelude::*;
use cmux_core::{
    terminal::{TerminalCommand, TerminalSession},
    APP_ID,
};
use gtk::glib;

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &adw::Application) {
    let header = adw::HeaderBar::new();
    let new_session_button = gtk::Button::builder()
        .icon_name("tab-new-symbolic")
        .tooltip_text("New terminal session")
        .build();
    header.pack_start(&new_session_button);

    let sidebar = gtk::ListBox::new();
    sidebar.add_css_class("navigation-sidebar");
    sidebar.set_selection_mode(gtk::SelectionMode::Single);
    sidebar.set_size_request(220, -1);

    let terminal_stack = gtk::Stack::builder()
        .hexpand(true)
        .vexpand(true)
        .transition_type(gtk::StackTransitionType::Crossfade)
        .build();

    let next_session_number = Rc::new(Cell::new(1_u32));
    add_session(&sidebar, &terminal_stack, &next_session_number);
    add_session(&sidebar, &terminal_stack, &next_session_number);

    if let Some(row) = sidebar.row_at_index(0) {
        sidebar.select_row(Some(&row));
    }

    {
        let terminal_stack = terminal_stack.clone();
        sidebar.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                terminal_stack.set_visible_child_name(&session_id_for_row(row));
            }
        });
    }

    {
        let sidebar = sidebar.clone();
        let terminal_stack = terminal_stack.clone();
        let next_session_number = Rc::clone(&next_session_number);
        new_session_button.connect_clicked(move |_| {
            let row = add_session(&sidebar, &terminal_stack, &next_session_number);
            sidebar.select_row(Some(&row));
        });
    }

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

    window.present();
}

fn add_session(
    sidebar: &gtk::ListBox,
    terminal_stack: &gtk::Stack,
    next_session_number: &Cell<u32>,
) -> gtk::ListBoxRow {
    let session_number = next_session_number.get();
    next_session_number.set(session_number + 1);

    let session_id = format!("session-{session_number}");
    let title = format!("Session {session_number}");
    let session = TerminalSession::new(
        session_id.clone(),
        title.clone(),
        TerminalCommand::user_shell(),
    );

    let row = gtk::ListBoxRow::new();
    row.set_widget_name(&session_id);
    row.set_child(Some(
        &gtk::Label::builder().label(&title).xalign(0.0).build(),
    ));
    sidebar.append(&row);

    let terminal = terminal::terminal(&session);
    terminal_stack.add_titled(&terminal, Some(&session_id), &title);
    row
}

fn session_id_for_row(row: &gtk::ListBoxRow) -> glib::GString {
    row.widget_name()
}
