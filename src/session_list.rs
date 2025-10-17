use gtk4::prelude::*;
use gtk4::{glib, Box as GtkBox, Button, Label, ListBox, Orientation, ScrolledWindow};
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

use crate::api::{Session, SharedApiClient};

pub struct SessionList {
    widget: GtkBox,
    list_box: ListBox,
    api_client: SharedApiClient,
    session_selected_callback: Rc<RefCell<Option<Box<dyn Fn(String)>>>>,
    new_session_callback: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    sessions: Rc<RefCell<Vec<Session>>>,
}

impl SessionList {
    pub fn new(api_client: SharedApiClient) -> Self {
        let widget = GtkBox::new(Orientation::Vertical, 0);
        widget.set_width_request(250);

        let new_button = Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("New Session")
            .build();
        new_button.add_css_class("flat");
        
        let header_box = GtkBox::new(Orientation::Horizontal, 6);
        header_box.set_margin_start(12);
        header_box.set_margin_end(12);
        header_box.set_margin_top(12);
        header_box.set_margin_bottom(12);
        
        let title_label = Label::new(Some("Sessions"));
        title_label.add_css_class("title-4");
        title_label.set_hexpand(true);
        title_label.set_halign(gtk4::Align::Start);
        
        header_box.append(&title_label);
        header_box.append(&new_button);

        let list_box = ListBox::new();
        list_box.add_css_class("navigation-sidebar");
        
        let scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .child(&list_box)
            .build();

        widget.append(&header_box);
        widget.append(&scrolled);

        let session_selected_callback: Rc<RefCell<Option<Box<dyn Fn(String)>>>> = Rc::new(RefCell::new(None));
        let new_session_callback: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let sessions: Rc<RefCell<Vec<Session>>> = Rc::new(RefCell::new(Vec::new()));

        {
            let session_selected_callback = session_selected_callback.clone();
            let sessions = sessions.clone();
            list_box.connect_row_selected(move |_, row| {
                if let Some(row) = row {
                    let index = row.index() as usize;
                    let sessions = sessions.borrow();
                    if let Some(session) = sessions.get(index) {
                        if let Some(callback) = session_selected_callback.borrow().as_ref() {
                            callback(session.id.clone());
                        }
                    }
                }
            });
        }

        {
            let new_session_callback = new_session_callback.clone();
            new_button.connect_clicked(move |_| {
                if let Some(callback) = new_session_callback.borrow().as_ref() {
                    callback();
                }
            });
        }

        Self {
            widget,
            list_box,
            api_client,
            session_selected_callback,
            new_session_callback,
            sessions,
        }
    }

    pub fn widget(&self) -> GtkBox {
        self.widget.clone()
    }

    pub fn connect_session_selected<F: Fn(String) + 'static>(&mut self, callback: F) {
        *self.session_selected_callback.borrow_mut() = Some(Box::new(callback));
    }

    pub fn connect_new_session<F: Fn() + 'static>(&mut self, callback: F) {
        *self.new_session_callback.borrow_mut() = Some(Box::new(callback));
    }

    pub fn refresh_sessions(&mut self) {
        let api_client = self.api_client.clone();
        let list_box = self.list_box.clone();
        let sessions = self.sessions.clone();

        glib::MainContext::default().spawn_local(async move {
            let client = api_client.lock().await;
            match client.get_sessions().await {
                Ok(fetched_sessions) => {
                    drop(client);
                    
                    while let Some(child) = list_box.first_child() {
                        list_box.remove(&child);
                    }

                    *sessions.borrow_mut() = fetched_sessions.clone();

                    for session in fetched_sessions {
                    let row_box = GtkBox::new(Orientation::Horizontal, 8);
                    row_box.set_margin_start(12);
                    row_box.set_margin_end(12);
                    row_box.set_margin_top(8);
                    row_box.set_margin_bottom(8);

                    let label = Label::new(Some(&session.title.unwrap_or_else(|| "Untitled".to_string())));
                    label.set_halign(gtk4::Align::Start);
                    label.set_hexpand(true);
                    label.set_ellipsize(gtk4::pango::EllipsizeMode::End);

                    row_box.append(&label);
                    
                        list_box.append(&row_box);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load sessions: {}", e);
                }
            }
        });
    }
}
