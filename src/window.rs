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

        let menu_button = gtk4::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .build();

        let menu = gtk4::gio::Menu::new();
        menu.append(Some("Delete Session"), Some("win.delete-session"));
        menu.append(Some("Share Session"), Some("win.share-session"));
        menu.append(Some("Summarize"), Some("win.summarize"));
        
        menu_button.set_menu_model(Some(&menu));

        let header_bar = adw::HeaderBar::builder()
            .title_widget(&adw::WindowTitle::new("OpenCode", ""))
            .build();
        header_bar.pack_end(&menu_button);

        let chat_box = GtkBox::new(Orientation::Vertical, 0);
        chat_box.append(&chat_view.borrow().widget());
        chat_box.append(&message_input.borrow().widget());

        let content_box = GtkBox::new(Orientation::Vertical, 0);
        content_box.append(&header_bar);
        content_box.append(&chat_box);

        let split_view = adw::NavigationSplitView::builder()
            .sidebar(
                &adw::NavigationPage::builder()
                    .title("Sessions")
                    .child(&session_list.borrow().widget())
                    .build()
            )
            .content(
                &adw::NavigationPage::builder()
                    .title("Chat")
                    .child(&content_box)
                    .build()
            )
            .max_sidebar_width(300.0)
            .min_sidebar_width(200.0)
            .sidebar_width_fraction(0.25)
            .build();

        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&split_view));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .default_width(1200)
            .default_height(800)
            .content(&toast_overlay)
            .build();

        {
            let api_client = api_client.clone();
            let chat_view = chat_view.clone();
            let message_input = message_input.clone();
            let current_session_id = current_session_id.clone();
            
            session_list.borrow_mut().connect_session_selected(move |session_id| {
                let api_client = api_client.clone();
                let chat_view = chat_view.clone();
                let message_input = message_input.clone();
                let current_session_id = current_session_id.clone();
                let session_id = session_id.to_string();
                
                glib::MainContext::default().spawn_local(async move {
                    *current_session_id.borrow_mut() = Some(session_id.clone());
                    
                    let client = api_client.lock().await;
                    match client.get_messages(&session_id).await {
                        Ok(messages) => {
                            drop(client);
                            chat_view.borrow().set_messages(messages);
                            message_input.borrow_mut().set_session_id(Some(session_id));
                        }
                        Err(e) => {
                            eprintln!("Failed to load messages for session {}: {}", session_id, e);
                        }
                    }
                });
            });
        }

        {
            let api_client = api_client.clone();
            let session_list_clone = session_list.clone();
            let chat_view = chat_view.clone();
            let message_input = message_input.clone();
            let current_session_id = current_session_id.clone();
            
            session_list.borrow_mut().connect_new_session(move || {
                let api_client = api_client.clone();
                let session_list = session_list_clone.clone();
                let chat_view = chat_view.clone();
                let message_input = message_input.clone();
                let current_session_id = current_session_id.clone();
                
                glib::MainContext::default().spawn_local(async move {
                    let client = api_client.lock().await;
                    match client.create_session().await {
                        Ok(session) => {
                            drop(client);
                            session_list.borrow_mut().refresh_sessions();
                            *current_session_id.borrow_mut() = Some(session.id.clone());
                            chat_view.borrow().set_messages(vec![]);
                            message_input.borrow_mut().set_session_id(Some(session.id));
                        }
                        Err(e) => {
                            eprintln!("Failed to create session: {}", e);
                        }
                    }
                });
            });
        }

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

        {
            let api_client = api_client.clone();
            let current_session_id = current_session_id.clone();
            let chat_view_clone = chat_view.clone();
            let toast_overlay = toast_overlay.clone();
            
            chat_view.borrow().connect_revert(move |message_id| {
                if let Some(session_id) = current_session_id.borrow().as_ref() {
                    let api_client = api_client.clone();
                    let session_id = session_id.clone();
                    let chat_view = chat_view_clone.clone();
                    let toast_overlay = toast_overlay.clone();
                    
                    glib::MainContext::default().spawn_local(async move {
                        let client = api_client.lock().await;
                        match client.revert_message(&session_id, &message_id).await {
                            Ok(_) => {
                                match client.get_messages(&session_id).await {
                                    Ok(messages) => {
                                        drop(client);
                                        chat_view.borrow().set_messages(messages);
                                        let toast = adw::Toast::new("Message reverted");
                                        toast_overlay.add_toast(toast);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to refresh messages: {}", e);
                                        let toast = adw::Toast::new("Failed to refresh messages");
                                        toast_overlay.add_toast(toast);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to revert message: {}", e);
                                let toast = adw::Toast::new("Failed to revert message");
                                toast_overlay.add_toast(toast);
                            }
                        }
                    });
                }
            });
        }

        {
            let api_client = api_client.clone();
            let current_session_id = current_session_id.clone();
            let session_list = session_list.clone();
            let chat_view = chat_view.clone();
            let toast_overlay = toast_overlay.clone();
            
            let delete_action = gtk4::gio::SimpleAction::new("delete-session", None);
            delete_action.connect_activate(move |_, _| {
                if let Some(session_id) = current_session_id.borrow().as_ref() {
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
            });
            window.add_action(&delete_action);
        }

        {
            let api_client = api_client.clone();
            let current_session_id = current_session_id.clone();
            let toast_overlay = toast_overlay.clone();
            
            let share_action = gtk4::gio::SimpleAction::new("share-session", None);
            share_action.connect_activate(move |_, _| {
                if let Some(session_id) = current_session_id.borrow().as_ref() {
                    let api_client = api_client.clone();
                    let session_id = session_id.clone();
                    let toast_overlay = toast_overlay.clone();
                    
                    glib::MainContext::default().spawn_local(async move {
                        let client = api_client.lock().await;
                        match client.share_session(&session_id).await {
                            Ok(_) => {
                                let toast = adw::Toast::new("Session shared successfully");
                                toast_overlay.add_toast(toast);
                            }
                            Err(e) => {
                                eprintln!("Failed to share session: {}", e);
                                let toast = adw::Toast::new("Failed to share session");
                                toast_overlay.add_toast(toast);
                            }
                        }
                    });
                }
            });
            window.add_action(&share_action);
        }

        {
            let api_client = api_client.clone();
            let current_session_id = current_session_id.clone();
            let toast_overlay = toast_overlay.clone();
            
            let summarize_action = gtk4::gio::SimpleAction::new("summarize", None);
            summarize_action.connect_activate(move |_, _| {
                if let Some(session_id) = current_session_id.borrow().as_ref() {
                    let api_client = api_client.clone();
                    let session_id = session_id.clone();
                    let toast_overlay = toast_overlay.clone();
                    
                    glib::MainContext::default().spawn_local(async move {
                        let client = api_client.lock().await;
                        match client.summarize_session(&session_id).await {
                            Ok(_) => {
                                let toast = adw::Toast::new("Summarizing session...");
                                toast_overlay.add_toast(toast);
                            }
                            Err(e) => {
                                eprintln!("Failed to summarize session: {}", e);
                                let toast = adw::Toast::new("Failed to summarize session");
                                toast_overlay.add_toast(toast);
                            }
                        }
                    });
                }
            });
            window.add_action(&summarize_action);
        }

        session_list.borrow_mut().refresh_sessions();
        message_input.borrow_mut().load_models();

        let api_client_clone = api_client.clone();
        let chat_view_clone = chat_view.clone();
        let current_session_id_clone = current_session_id.clone();
        
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
                                                
                                                glib::MainContext::default().spawn_local(async move {
                                                    let client = api_client.lock().await;
                                                    if let Ok(messages) = client.get_messages(&session_id).await {
                                                        drop(client);
                                                        chat_view.borrow().set_messages(messages);
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
