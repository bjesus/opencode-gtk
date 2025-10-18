use gtk4::prelude::*;
use gtk4::{glib, Application, Box as GtkBox, Orientation};
use libadwaita as adw;
use adw::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::api::create_shared_client;
use crate::session_list::SessionList;
use crate::chat_view::ChatView;
use crate::message_input::MessageInput;

pub struct Window {
    window: adw::ApplicationWindow,
}

impl Window {
    pub fn new(app: &Application, server_url: String) -> adw::ApplicationWindow {
        let api_client = create_shared_client(server_url);
        
        let session_list = Rc::new(RefCell::new(SessionList::new(api_client.clone())));
        let chat_view = Rc::new(RefCell::new(ChatView::new()));
        let message_input = Rc::new(RefCell::new(MessageInput::new(api_client.clone())));
        let current_session_id: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

        // Back button for collapsed sidebar
        let back_button = gtk4::Button::builder()
            .icon_name("go-previous-symbolic")
            .visible(false)
            .build();

        let menu_button = gtk4::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .build();

        let menu = gtk4::gio::Menu::new();
        menu.append(Some("Rename Session"), Some("win.rename-session"));
        menu.append(Some("Fork Session"), Some("win.fork-session"));
        menu.append(Some("Delete Session"), Some("win.delete-session"));
        
        menu_button.set_menu_model(Some(&menu));

        let window_title = adw::WindowTitle::new("OpenCode", "");
        
        let progress_bar = gtk4::ProgressBar::new();
        progress_bar.add_css_class("osd");
        progress_bar.set_visible(false);
        
        let header_bar = adw::HeaderBar::builder()
            .title_widget(&window_title)
            .build();
        header_bar.pack_start(&back_button);
        header_bar.pack_end(&menu_button);
        
        let header_overlay = gtk4::Overlay::new();
        header_overlay.set_child(Some(&header_bar));
        header_overlay.add_overlay(&progress_bar);
        progress_bar.set_valign(gtk4::Align::End);
        progress_bar.set_halign(gtk4::Align::Fill);

        let chat_box = GtkBox::new(Orientation::Vertical, 0);
        chat_box.set_vexpand(true);
        
        let chat_view_widget = chat_view.borrow().widget();
        chat_view_widget.set_vexpand(true);
        chat_box.append(&chat_view_widget);
        
        let message_input_widget = message_input.borrow().widget();
        message_input_widget.set_vexpand(false);
        chat_box.append(&message_input_widget);

        let content_box = GtkBox::new(Orientation::Vertical, 0);
        content_box.append(&header_overlay);
        content_box.append(&chat_box);

        // Sidebar header with New Session button
        let sidebar_header = adw::HeaderBar::builder()
            .title_widget(&adw::WindowTitle::new("Sessions", ""))
            .build();
        sidebar_header.pack_start(&session_list.borrow().new_button());

        let sidebar_toolbar = adw::ToolbarView::new();
        sidebar_toolbar.add_top_bar(&sidebar_header);
        sidebar_toolbar.set_content(Some(&session_list.borrow().widget()));

        let sidebar_page = adw::NavigationPage::builder()
            .title("Sessions")
            .child(&sidebar_toolbar)
            .build();
        
        let content_page = adw::NavigationPage::builder()
            .title("Chat")
            .child(&content_box)
            .can_pop(false)
            .build();

        let split_view = adw::NavigationSplitView::builder()
            .sidebar(&sidebar_page)
            .content(&content_page)
            .max_sidebar_width(400.0)
            .min_sidebar_width(280.0)
            .sidebar_width_fraction(0.3)
            .collapsed(false)
            .show_content(true)
            .build();

        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&split_view));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .default_width(1200)
            .default_height(800)
            .content(&toast_overlay)
            .build();

        window.set_size_request(360, 500);

        // Back button functionality
        {
            let split_view_clone = split_view.clone();
            back_button.connect_clicked(move |_| {
                split_view_clone.set_show_content(false);
            });
        }

        // Show/hide back button based on collapsed state
        {
            let back_button_weak = back_button.downgrade();
            split_view.connect_show_content_notify(move |sv| {
                if let Some(btn) = back_button_weak.upgrade() {
                    btn.set_visible(sv.shows_content() && sv.is_collapsed());
                }
            });
        }

        {
            let back_button_weak = back_button.downgrade();
            split_view.connect_collapsed_notify(move |sv| {
                if let Some(btn) = back_button_weak.upgrade() {
                    btn.set_visible(sv.shows_content() && sv.is_collapsed());
                }
            });
        }

        // Responsive breakpoint
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 720px").unwrap());
        let split_view_weak = split_view.downgrade();
        breakpoint.connect_apply(move |_| {
            if let Some(sv) = split_view_weak.upgrade() {
                sv.set_collapsed(true);
            }
        });
        let split_view_weak = split_view.downgrade();
        breakpoint.connect_unapply(move |_| {
            if let Some(sv) = split_view_weak.upgrade() {
                sv.set_collapsed(false);
            }
        });
        window.add_breakpoint(breakpoint);

        // Session selected callback
        {
            let api_client = api_client.clone();
            let chat_view = chat_view.clone();
            let message_input = message_input.clone();
            let current_session_id = current_session_id.clone();
            let split_view_clone = split_view.clone();
            let window_title_clone = window_title.clone();
            let session_list_for_title = session_list.clone();
            let progress_bar_clone = progress_bar.clone();
            
            session_list.borrow_mut().connect_session_selected(move |session_id| {
                let api_client = api_client.clone();
                let chat_view = chat_view.clone();
                let message_input = message_input.clone();
                let current_session_id = current_session_id.clone();
                let session_id = session_id.to_string();
                let split_view = split_view_clone.clone();
                let window_title = window_title_clone.clone();
                let session_list = session_list_for_title.clone();
                let progress_bar = progress_bar_clone.clone();
                
                // Switch to content view when session is selected
                split_view.set_show_content(true);
                
                // 1. Clear old content immediately
                chat_view.borrow().set_messages(vec![]);
                
                // 2. Show progress bar and start pulsing
                progress_bar.set_visible(true);
                progress_bar.pulse();
                
                let progress_bar_pulse = progress_bar.clone();
                let pulse_source_id = glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    progress_bar_pulse.pulse();
                    glib::ControlFlow::Continue
                });
                
                // 3. Load data asynchronously without blocking UI
                glib::MainContext::default().spawn_local(async move {
                    *current_session_id.borrow_mut() = Some(session_id.clone());
                    
                    // Update window title with session name
                    let session_title = session_list.borrow().get_session_title(&session_id);
                    if let Some(title) = session_title {
                        window_title.set_title(&format!("{} - OpenCode", title));
                    } else {
                        window_title.set_title("OpenCode");
                    }
                    
                    let client = api_client.lock().await;
                    match client.get_messages(&session_id).await {
                        Ok(messages) => {
                            drop(client);
                            
                            // Find last model and agent used
                            let last_model = messages.iter()
                                .rev()
                                .find(|m| m.info.role == "assistant")
                                .and_then(|m| m.info.model_id.as_ref())
                                .cloned();
                            
                            let last_agent = messages.iter()
                                .rev()
                                .find(|m| m.info.role == "assistant")
                                .and_then(|m| m.info.mode.as_ref())
                                .cloned();
                            
                            // Update UI in idle callback to avoid blocking
                            glib::idle_add_local_once(move || {
                                chat_view.borrow().set_messages(messages);
                                message_input.borrow_mut().set_session_id(Some(session_id));
                                
                                // Update model selector
                                if let Some(model_id) = last_model {
                                    message_input.borrow_mut().set_model_by_id(&model_id);
                                }
                                
                                // Update agent selector
                                if let Some(agent_name) = last_agent {
                                    message_input.borrow_mut().set_agent_by_name(&agent_name);
                                }
                                
                                // Hide progress bar
                                pulse_source_id.remove();
                                progress_bar.set_visible(false);
                            });
                        }
                        Err(e) => {
                            eprintln!("Failed to load messages for session {}: {}", session_id, e);
                            pulse_source_id.remove();
                            progress_bar.set_visible(false);
                        }
                    }
                });
            });
        }

        // New session callback
        {
            let api_client = api_client.clone();
            let session_list_clone = session_list.clone();
            let chat_view = chat_view.clone();
            let message_input = message_input.clone();
            let current_session_id = current_session_id.clone();
            let split_view_clone = split_view.clone();
            let window_title_clone = window_title.clone();
            
            session_list.borrow_mut().connect_new_session(move || {
                let api_client = api_client.clone();
                let session_list = session_list_clone.clone();
                let chat_view = chat_view.clone();
                let message_input = message_input.clone();
                let current_session_id = current_session_id.clone();
                let split_view = split_view_clone.clone();
                let window_title = window_title_clone.clone();
                
                glib::MainContext::default().spawn_local(async move {
                    let client = api_client.lock().await;
                    match client.create_session().await {
                        Ok(session) => {
                            drop(client);
                            session_list.borrow_mut().refresh_sessions();
                            *current_session_id.borrow_mut() = Some(session.id.clone());
                            chat_view.borrow().set_messages(vec![]);
                            message_input.borrow_mut().set_session_id(Some(session.id.clone()));
                            
                            window_title.set_title("New Session - OpenCode");
                            split_view.set_show_content(true);
                        }
                        Err(e) => {
                            eprintln!("Failed to create session: {}", e);
                        }
                    }
                });
            });
        }

        // Message sent callback
        {
            let chat_view = chat_view.clone();
            let current_session_id = current_session_id.clone();
            
            message_input.borrow_mut().connect_message_sent(move |message| {
                if let Some(_session_id) = current_session_id.borrow().as_ref() {
                    chat_view.borrow().add_message(message.clone());
                    chat_view.borrow().show_thinking();
                }
            });
        }

        // No session callback (creates new session)
        {
            let api_client = api_client.clone();
            let session_list_clone = session_list.clone();
            let chat_view = chat_view.clone();
            let message_input_clone = message_input.clone();
            let current_session_id = current_session_id.clone();
            
            message_input.borrow_mut().connect_no_session(move |text, provider_id, model_id, agent| {
                let api_client = api_client.clone();
                let session_list = session_list_clone.clone();
                let chat_view = chat_view.clone();
                let message_input = message_input_clone.clone();
                let current_session_id = current_session_id.clone();
                
                glib::MainContext::default().spawn_local(async move {
                    let client = api_client.lock().await;
                    match client.create_session().await {
                        Ok(session) => {
                            drop(client);
                            session_list.borrow_mut().refresh_sessions();
                            *current_session_id.borrow_mut() = Some(session.id.clone());
                            message_input.borrow_mut().set_session_id(Some(session.id.clone()));
                            
                            chat_view.borrow().set_messages(vec![]);
                            
                            let user_message = crate::api::Message {
                                info: crate::api::MessageInfo {
                                    id: format!("temp-{}", glib::uuid_string_random()),
                                    session_id: Some(session.id.clone()),
                                    role: "user".to_string(),
                                    cost: None,
                                    tokens: None,
                                    model_id: None,
                                    provider_id: None,
                                    mode: None,
                                    path: None,
                                },
                                parts: vec![crate::api::MessagePart { 
                                    text: Some(text.clone()),
                                    part_type: "text".to_string(),
                                    tool: None,
                                    state: None,
                                }],
                            };
                            
                            chat_view.borrow().add_message(user_message);
                            chat_view.borrow().show_thinking();
                            
                            let request = crate::api::SendMessageRequest {
                                parts: vec![crate::api::MessagePart {
                                    text: Some(text.clone()),
                                    part_type: "text".to_string(),
                                    tool: None,
                                    state: None,
                                }],
                                model: crate::api::ModelInfo {
                                    model_id,
                                    provider_id,
                                },
                                agent,
                            };
                            
                            let client = api_client.lock().await;
                            match client.send_message(&session.id, request).await {
                                Ok(_) => {
                                    eprintln!("Message sent successfully to new session!");
                                }
                                Err(e) => {
                                    eprintln!("Failed to send message to new session: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to create session: {}", e);
                        }
                    }
                });
            });
        }

        // RENAME SESSION ACTION
        {
            let api_client = api_client.clone();
            let current_session_id = current_session_id.clone();
            let session_list = session_list.clone();
            let toast_overlay = toast_overlay.clone();
            let window_clone = window.clone();
            let window_title_clone = window_title.clone();
            
            let rename_action = gtk4::gio::SimpleAction::new("rename-session", None);
            rename_action.connect_activate(move |_, _| {
                if let Some(session_id) = current_session_id.borrow().as_ref() {
                    let current_title = session_list.borrow().get_session_title(session_id);
                    
                    let dialog = adw::MessageDialog::builder()
                        .heading("Rename Session")
                        .build();
                    
                    let entry = gtk4::Entry::new();
                    if let Some(title) = current_title {
                        entry.set_text(&title);
                    }
                    entry.set_placeholder_text(Some("Enter new name"));
                    entry.set_margin_start(12);
                    entry.set_margin_end(12);
                    entry.set_margin_top(6);
                    entry.set_margin_bottom(6);
                    
                    let entry_box = GtkBox::new(Orientation::Vertical, 0);
                    entry_box.append(&entry);
                    dialog.set_extra_child(Some(&entry_box));
                    
                    dialog.add_response("cancel", "Cancel");
                    dialog.add_response("rename", "Rename");
                    dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);
                    dialog.set_default_response(Some("rename"));
                    
                    // Submit on Enter key
                    let dialog_for_enter = dialog.clone();
                    entry.connect_activate(move |_| {
                        dialog_for_enter.response("rename");
                    });
                    
                    let api_client = api_client.clone();
                    let session_id = session_id.clone();
                    let session_list = session_list.clone();
                    let toast_overlay = toast_overlay.clone();
                    let window_title = window_title_clone.clone();
                    
                    dialog.connect_response(None, move |dialog, response| {
                        if response == "rename" {
                            let new_title = entry.text().to_string();
                            if !new_title.trim().is_empty() {
                                let api_client = api_client.clone();
                                let session_id = session_id.clone();
                                let session_list = session_list.clone();
                                let toast_overlay = toast_overlay.clone();
                                let window_title = window_title.clone();
                                
                                glib::MainContext::default().spawn_local(async move {
                                    let client = api_client.lock().await;
                                    match client.rename_session(&session_id, &new_title).await {
                                        Ok(_) => {
                                            drop(client);
                                            session_list.borrow_mut().refresh_sessions();
                                            window_title.set_title(&format!("{} - OpenCode", new_title));
                                            let toast = adw::Toast::new("Session renamed");
                                            toast_overlay.add_toast(toast);
                                        }
                                        Err(e) => {
                                            eprintln!("Failed to rename session: {}", e);
                                            let toast = adw::Toast::new("Failed to rename session");
                                            toast_overlay.add_toast(toast);
                                        }
                                    }
                                });
                            }
                        }
                        dialog.close();
                    });
                    
                    dialog.set_transient_for(Some(&window_clone));
                    dialog.present();
                }
            });
            window.add_action(&rename_action);
        }

        // FORK SESSION ACTION
        {
            let api_client = api_client.clone();
            let current_session_id = current_session_id.clone();
            let session_list = session_list.clone();
            let chat_view = chat_view.clone();
            let toast_overlay = toast_overlay.clone();
            let split_view_clone = split_view.clone();
            let message_input_clone = message_input.clone();
            
            let fork_action = gtk4::gio::SimpleAction::new("fork-session", None);
            fork_action.connect_activate(move |_, _| {
                if let Some(session_id) = current_session_id.borrow().as_ref() {
                    let api_client = api_client.clone();
                    let session_id = session_id.clone();
                    let session_list = session_list.clone();
                    let chat_view = chat_view.clone();
                    let toast_overlay = toast_overlay.clone();
                    let split_view = split_view_clone.clone();
                    let message_input = message_input_clone.clone();
                    
                    glib::MainContext::default().spawn_local(async move {
                        let client = api_client.lock().await;
                        match client.fork_session(&session_id, None).await {
                            Ok(new_session) => {
                                let new_session_id = new_session.id.clone();
                                drop(client);
                                
                                session_list.borrow_mut().refresh_sessions();
                                
                                // Load the forked session
                                let client = api_client.lock().await;
                                match client.get_messages(&new_session_id).await {
                                    Ok(messages) => {
                                        drop(client);
                                        chat_view.borrow().set_messages(messages);
                                        message_input.borrow_mut().set_session_id(Some(new_session_id));
                                        
                                        let toast = adw::Toast::new("Session forked");
                                        toast_overlay.add_toast(toast);
                                        
                                        split_view.set_show_content(true);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to load forked session messages: {}", e);
                                        let toast = adw::Toast::new("Failed to load forked session");
                                        toast_overlay.add_toast(toast);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to fork session: {}", e);
                                let toast = adw::Toast::new("Failed to fork session");
                                toast_overlay.add_toast(toast);
                            }
                        }
                    });
                }
            });
            window.add_action(&fork_action);
        }

        // DELETE SESSION ACTION (with confirmation)
        {
            let api_client = api_client.clone();
            let current_session_id = current_session_id.clone();
            let session_list = session_list.clone();
            let chat_view = chat_view.clone();
            let toast_overlay = toast_overlay.clone();
            let window_clone = window.clone();
            
            let delete_action = gtk4::gio::SimpleAction::new("delete-session", None);
            delete_action.connect_activate(move |_, _| {
                if let Some(session_id) = current_session_id.borrow().as_ref() {
                    let dialog = adw::MessageDialog::builder()
                        .heading("Delete Session")
                        .body("Are you sure you want to delete this session? This action cannot be undone.")
                        .build();
                    
                    dialog.add_response("cancel", "Cancel");
                    dialog.add_response("delete", "Delete");
                    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
                    dialog.set_default_response(Some("cancel"));
                    dialog.set_close_response("cancel");
                    
                    let api_client = api_client.clone();
                    let session_id = session_id.clone();
                    let session_list = session_list.clone();
                    let chat_view = chat_view.clone();
                    let toast_overlay = toast_overlay.clone();
                    
                    dialog.connect_response(None, move |dialog, response| {
                        if response == "delete" {
                            let api_client = api_client.clone();
                            let session_id = session_id.clone();
                            let session_list = session_list.clone();
                            let chat_view = chat_view.clone();
                            let toast_overlay = toast_overlay.clone();
                            
                            glib::MainContext::default().spawn_local(async move {
                                let client = api_client.lock().await;
                                match client.delete_session(&session_id).await {
                                    Ok(_) => {
                                        drop(client);
                                        session_list.borrow_mut().refresh_sessions();
                                        chat_view.borrow().set_messages(vec![]);
                                        
                                        let toast = adw::Toast::new("Session deleted");
                                        toast_overlay.add_toast(toast);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to delete session: {}", e);
                                        let toast = adw::Toast::new("Failed to delete session");
                                        toast_overlay.add_toast(toast);
                                    }
                                }
                            });
                        }
                        dialog.close();
                    });
                    
                    dialog.set_transient_for(Some(&window_clone));
                    dialog.present();
                }
            });
            window.add_action(&delete_action);
        }

        // Initialize: Load models, agents, and sessions
        message_input.borrow_mut().load_models();
        message_input.borrow_mut().load_agents();
        
        // Load sessions and auto-select the first (most recent) one
        {
            let api_client = api_client.clone();
            let session_list_clone = session_list.clone();
            
            glib::MainContext::default().spawn_local(async move {
                let client = api_client.lock().await;
                match client.get_sessions().await {
                    Ok(sessions) => {
                        drop(client);
                        
                        // Store the first session ID
                        let first_session_id = sessions.first().map(|s| s.id.clone());
                        
                        // Set sessions synchronously (this updates the internal list)
                        session_list_clone.borrow_mut().set_sessions(sessions);
                        
                        // Now auto-load the first (most recent) session
                        if let Some(session_id) = first_session_id {
                            session_list_clone.borrow_mut().select_session(&session_id);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to load sessions: {}", e);
                    }
                }
            });
        }

        // Event stream for live updates
        let api_client_clone = api_client.clone();
        let chat_view_clone = chat_view.clone();
        let current_session_id_clone = current_session_id.clone();
        let session_list_for_events = session_list.clone();
        let window_title_for_events = window_title.clone();
        
        glib::MainContext::default().spawn_local(async move {
            use futures::StreamExt;
            
            let client = api_client_clone.lock().await;
            match client.get_event_stream().await {
                Ok(stream) => {
                    drop(client);
                    
                    let mut stream = std::pin::pin!(stream);
                    while let Some(Ok(event)) = stream.next().await {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&event.data) {
                            let event_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
                            
                            if event_type == "message.updated" {
                                if let Some(properties) = data.get("properties") {
                                    if let Some(session_id) = properties.get("sessionID").and_then(|v| v.as_str()) {
                                        if let Some(current_id) = current_session_id_clone.borrow().as_ref() {
                                            if session_id == current_id {
                                                chat_view_clone.borrow().show_thinking();
                                            }
                                        }
                                    }
                                }
                            } else if event_type == "message.part.updated" {
                                if let Some(properties) = data.get("properties") {
                                    let message_id = properties.get("messageID").and_then(|v| v.as_str());
                                    let text = properties.get("text").and_then(|v| v.as_str());
                                    let session_id = properties.get("sessionID").and_then(|v| v.as_str());
                                    
                                    if let (Some(message_id), Some(text), Some(session_id)) = (message_id, text, session_id) {
                                        if let Some(current_id) = current_session_id_clone.borrow().as_ref() {
                                            if session_id == current_id {
                                                chat_view_clone.borrow().hide_thinking();
                                                chat_view_clone.borrow().update_message(message_id, text);
                                            }
                                        }
                                    }
                                }
                            } else if event_type == "session.idle" {
                                if let Some(properties) = data.get("properties") {
                                    if let Some(session_id) = properties.get("sessionID").and_then(|v| v.as_str()) {
                                        if let Some(current_id) = current_session_id_clone.borrow().as_ref() {
                                            if session_id == current_id {
                                                let api_client = api_client_clone.clone();
                                                let chat_view = chat_view_clone.clone();
                                                let session_id = session_id.to_string();
                                                let session_list = session_list_for_events.clone();
                                                let window_title = window_title_for_events.clone();
                                                
                                                glib::MainContext::default().spawn_local(async move {
                                                    let client = api_client.lock().await;
                                                    
                                                    // Fetch updated messages
                                                    if let Ok(messages) = client.get_messages(&session_id).await {
                                                        chat_view.borrow().set_messages(messages);
                                                    }
                                                    
                                                    // Refresh sessions list to get updated title
                                                    if let Ok(sessions) = client.get_sessions().await {
                                                        drop(client);
                                                        session_list.borrow_mut().set_sessions(sessions.clone());
                                                        
                                                        // Update window title with new session name
                                                        if let Some(session) = sessions.iter().find(|s| s.id == session_id) {
                                                            if let Some(title) = &session.title {
                                                                window_title.set_title(&format!("{} - OpenCode", title));
                                                            }
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to connect to event stream: {}", e);
                }
            }
        });

        window
    }

    pub fn present(&self) {
        self.window.present();
    }
}
