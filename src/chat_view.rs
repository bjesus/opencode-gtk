use gtk4::prelude::*;
use gtk4::{glib, Box as GtkBox, Label, Orientation, ScrolledWindow};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::api::Message;
use crate::markdown::markdown_to_pango;

pub struct ChatView {
    widget: ScrolledWindow,
    messages_box: GtkBox,
    message_widgets: Rc<RefCell<HashMap<String, Label>>>,
    thinking_widget: Rc<RefCell<Option<GtkBox>>>,
    revert_callback: Rc<RefCell<Option<Box<dyn Fn(String)>>>>,
}

impl ChatView {
    pub fn new() -> Self {
        let messages_box = GtkBox::new(Orientation::Vertical, 12);
        messages_box.set_margin_start(12);
        messages_box.set_margin_end(12);
        messages_box.set_margin_top(12);
        messages_box.set_margin_bottom(12);
        messages_box.set_valign(gtk4::Align::Start);

        let widget = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .child(&messages_box)
            .build();

        Self {
            widget,
            messages_box,
            message_widgets: Rc::new(RefCell::new(HashMap::new())),
            thinking_widget: Rc::new(RefCell::new(None)),
            revert_callback: Rc::new(RefCell::new(None)),
        }
    }

    pub fn connect_revert<F>(&self, callback: F)
    where
        F: Fn(String) + 'static,
    {
        *self.revert_callback.borrow_mut() = Some(Box::new(callback));
    }

    pub fn widget(&self) -> ScrolledWindow {
        self.widget.clone()
    }

    pub fn set_messages(&self, messages: Vec<Message>) {
        while let Some(child) = self.messages_box.first_child() {
            self.messages_box.remove(&child);
        }
        self.message_widgets.borrow_mut().clear();

        for message in messages {
            self.add_message_internal(message);
        }

        self.scroll_to_bottom();
    }

    pub fn add_message(&self, message: Message) {
        self.add_message_internal(message);
        self.scroll_to_bottom();
    }

    fn add_message_internal(&self, message: Message) {
        let text = message.parts.iter()
            .filter_map(|p| p.text.as_ref())
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("");

        if message.info.role == "user" {
            let message_box = GtkBox::new(Orientation::Vertical, 0);
            message_box.add_css_class("card");
            message_box.set_margin_top(12);
            message_box.set_margin_bottom(12);
            message_box.set_margin_start(48);
            message_box.set_margin_end(12);
            
            let content_label = Label::new(Some(&text));
            content_label.set_halign(gtk4::Align::Start);
            content_label.set_wrap(true);
            content_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            content_label.set_selectable(true);
            content_label.set_xalign(0.0);
            content_label.set_margin_start(12);
            content_label.set_margin_end(12);
            content_label.set_margin_top(12);
            content_label.set_margin_bottom(12);
            
            message_box.append(&content_label);
            self.messages_box.append(&message_box);
        } else {
            let message_box = GtkBox::new(Orientation::Vertical, 6);
            message_box.set_margin_top(12);
            message_box.set_margin_bottom(12);
            
            let content_label = Label::new(None);
            content_label.set_halign(gtk4::Align::Start);
            content_label.set_wrap(true);
            content_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            content_label.set_selectable(true);
            content_label.set_xalign(0.0);
            content_label.set_use_markup(true);
            
            let markup = markdown_to_pango(&text);
            content_label.set_markup(&markup);
            
            message_box.append(&content_label);
            
            let button_box = GtkBox::new(Orientation::Horizontal, 6);
            button_box.set_margin_top(6);
            
            let copy_button = gtk4::Button::builder()
                .icon_name("edit-copy-symbolic")
                .tooltip_text("Copy")
                .build();
            copy_button.add_css_class("flat");
            copy_button.add_css_class("dim-label");
            
            let text_clone = text.clone();
            copy_button.connect_clicked(move |_| {
                if let Some(display) = gtk4::gdk::Display::default() {
                    let clipboard = display.clipboard();
                    clipboard.set_text(&text_clone);
                }
            });
            
            let toggle_source_button = gtk4::Button::builder()
                .icon_name("view-reveal-symbolic")
                .tooltip_text("Toggle Source")
                .build();
            toggle_source_button.add_css_class("flat");
            toggle_source_button.add_css_class("dim-label");
            
            let source_label = Label::new(Some(&text));
            source_label.set_halign(gtk4::Align::Start);
            source_label.set_wrap(true);
            source_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            source_label.set_selectable(true);
            source_label.set_xalign(0.0);
            source_label.add_css_class("monospace");
            source_label.set_visible(false);
            
            toggle_source_button.connect_clicked({
                let source_label = source_label.clone();
                move |_| {
                    source_label.set_visible(!source_label.is_visible());
                }
            });
            
            let revert_button = gtk4::Button::builder()
                .icon_name("edit-undo-symbolic")
                .tooltip_text("Revert")
                .build();
            revert_button.add_css_class("flat");
            revert_button.add_css_class("dim-label");
            
            let revert_callback = self.revert_callback.clone();
            let message_id = message.info.id.clone();
            revert_button.connect_clicked(move |_| {
                if let Some(callback) = revert_callback.borrow().as_ref() {
                    callback(message_id.clone());
                }
            });
            
            button_box.append(&copy_button);
            button_box.append(&toggle_source_button);
            button_box.append(&revert_button);
            
            message_box.append(&source_label);
            message_box.append(&button_box);
            
            self.messages_box.append(&message_box);
            self.message_widgets.borrow_mut().insert(message.info.id.clone(), content_label);
        }
    }

    pub fn update_message(&self, message_id: &str, text: &str) {
        if let Some(label) = self.message_widgets.borrow().get(message_id) {
            let current_text = label.text();
            label.set_text(&format!("{}{}", current_text, text));
            self.scroll_to_bottom();
        }
    }

    pub fn show_thinking(&self) {
        if self.thinking_widget.borrow().is_some() {
            return;
        }

        let thinking_box = GtkBox::new(Orientation::Vertical, 6);
        thinking_box.add_css_class("card");
        thinking_box.set_margin_top(6);
        thinking_box.set_margin_bottom(6);
        
        let role_label = Label::new(Some("Assistant:"));
        role_label.set_halign(gtk4::Align::Start);
        role_label.add_css_class("heading");
        thinking_box.append(&role_label);

        let thinking_label = Label::new(Some("Thinking..."));
        thinking_label.set_halign(gtk4::Align::Start);
        thinking_label.add_css_class("dim-label");
        thinking_box.append(&thinking_label);
        
        self.messages_box.append(&thinking_box);
        *self.thinking_widget.borrow_mut() = Some(thinking_box);
        
        self.scroll_to_bottom();
    }

    pub fn hide_thinking(&self) {
        if let Some(widget) = self.thinking_widget.borrow_mut().take() {
            self.messages_box.remove(&widget);
        }
    }

    fn scroll_to_bottom(&self) {
        glib::idle_add_local_once({
            let vadj = self.widget.vadjustment();
            move || {
                vadj.set_value(vadj.upper() - vadj.page_size());
            }
        });
    }
}
