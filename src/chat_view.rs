use gtk4::prelude::*;
use gtk4::{gio, glib, Box as GtkBox, Label, ListView, Orientation, ScrolledWindow, SignalListItemFactory};
use libadwaita as adw;
use adw::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::api::Message;
use crate::markdown::markdown_to_pango;
use crate::message_object::MessageObject;

pub struct ChatView {
    widget: adw::OverlaySplitView,
    scroll_widget: ScrolledWindow,
    listview: ListView,
    list_store: gio::ListStore,
    thinking_widget: Rc<RefCell<Option<GtkBox>>>,
    info_sidebar: Rc<RefCell<GtkBox>>,
    split_view: Rc<RefCell<adw::OverlaySplitView>>,
}

impl ChatView {
    pub fn new() -> Self {
        let list_store = gio::ListStore::new::<MessageObject>();
        let selection_model = gtk4::NoSelection::new(Some(list_store.clone()));
        
        // Info sidebar (initially empty)
        let info_sidebar = GtkBox::new(Orientation::Vertical, 12);
        info_sidebar.set_margin_start(12);
        info_sidebar.set_margin_end(12);
        info_sidebar.set_margin_top(12);
        info_sidebar.set_margin_bottom(12);

        // Wrap in OverlaySplitView
        let widget = adw::OverlaySplitView::builder()
            .sidebar_position(gtk4::PackType::End)
            .show_sidebar(false)
            .collapsed(true)
            .max_sidebar_width(400.0)
            .min_sidebar_width(300.0)
            .build();

        let info_sidebar_rc = Rc::new(RefCell::new(info_sidebar.clone()));
        let split_view_rc = Rc::new(RefCell::new(widget.clone()));
        
        let factory = SignalListItemFactory::new();
        
        factory.connect_setup(move |_, list_item| {
            let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
            let container = GtkBox::new(Orientation::Vertical, 0);
            list_item.set_child(Some(&container));
        });
        
        let info_sidebar_for_factory = info_sidebar_rc.clone();
        let split_view_for_factory = split_view_rc.clone();
        factory.connect_bind(move |_, list_item| {
            let list_item = list_item.downcast_ref::<gtk4::ListItem>().unwrap();
            let Some(message_obj) = list_item.item().and_downcast::<MessageObject>() else {
                return;
            };
            
            let message = message_obj.message();
            let container = list_item.child().and_downcast::<GtkBox>().unwrap();
            
            // Clear previous children
            while let Some(child) = container.first_child() {
                container.remove(&child);
            }
            
            if message.info.role == "thinking" {
                Self::render_thinking(&container);
            } else if message.info.role == "user" {
                Self::render_user_message(&container, &message);
            } else if message.info.role == "assistant" {
                Self::render_assistant_message(&container, &message, &info_sidebar_for_factory, &split_view_for_factory);
            }
        });

        let listview = ListView::new(Some(selection_model), Some(factory));
        listview.set_vexpand(true);
        listview.set_hexpand(true);

        let scroll_widget = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .child(&listview)
            .build();

        // Set content and sidebar on the widget
        widget.set_content(Some(&scroll_widget));
        widget.set_sidebar(Some(&info_sidebar));

        Self {
            widget,
            scroll_widget,
            listview,
            list_store,
            thinking_widget: Rc::new(RefCell::new(None)),
            info_sidebar: info_sidebar_rc,
            split_view: split_view_rc,
        }
    }

    fn render_thinking(container: &GtkBox) {
        let thinking_box = GtkBox::new(Orientation::Horizontal, 12);
        thinking_box.set_margin_start(12);
        thinking_box.set_margin_end(12);
        thinking_box.set_margin_top(12);
        thinking_box.set_margin_bottom(12);

        let spinner = gtk4::Spinner::new();
        spinner.set_spinning(true);
        spinner.set_size_request(16, 16);
        thinking_box.append(&spinner);

        let label = Label::new(Some("Generating..."));
        label.add_css_class("dim-label");
        thinking_box.append(&label);

        container.append(&thinking_box);
    }

    fn render_user_message(container: &GtkBox, message: &Message) {
        let message_box = GtkBox::new(Orientation::Vertical, 0);
        message_box.add_css_class("card");
        message_box.set_margin_top(12);
        message_box.set_margin_bottom(12);
        message_box.set_margin_start(48);
        message_box.set_margin_end(12);

        let text: String = message.parts.iter()
            .filter_map(|p| p.text.as_ref())
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .join("");

        if !text.is_empty() {
            let label = Label::new(Some(&text));
            label.set_wrap(true);
            label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            label.set_xalign(0.0);
            label.set_selectable(true);
            label.set_margin_start(12);
            label.set_margin_end(12);
            label.set_margin_top(12);
            label.set_margin_bottom(12);
            message_box.append(&label);
        }

        container.append(&message_box);
    }

    fn render_assistant_message(container: &GtkBox, message: &Message, info_sidebar: &Rc<RefCell<GtkBox>>, split_view: &Rc<RefCell<adw::OverlaySplitView>>) {
        let message_box = GtkBox::new(Orientation::Vertical, 6);
        message_box.set_margin_top(12);
        message_box.set_margin_bottom(12);
        message_box.set_margin_start(12);
        message_box.set_margin_end(12);

        // Render text parts
        let text: String = message.parts.iter()
            .filter_map(|p| if p.part_type == "text" { p.text.as_ref() } else { None })
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .join("");

        if !text.is_empty() {
            let markdown_text = markdown_to_pango(&text);
            let label = Label::new(None);
            label.set_markup(&markdown_text);
            label.set_wrap(true);
            label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            label.set_xalign(0.0);
            label.set_selectable(true);
            message_box.append(&label);
        }

        // Render tool calls
        let tool_parts: Vec<_> = message.parts.iter()
            .filter(|p| p.part_type == "tool")
            .collect();

        if !tool_parts.is_empty() {
            let tools_box = GtkBox::new(Orientation::Vertical, 12);
            tools_box.set_margin_top(12);

            for tool_part in tool_parts {
                let tool_container = GtkBox::new(Orientation::Vertical, 6);
                tool_container.add_css_class("card");
                tool_container.set_margin_bottom(6);

                // Tool name
                if let Some(tool_name) = &tool_part.tool {
                    let tool_header = Label::new(Some(&format!("🔧 {}", tool_name)));
                    tool_header.add_css_class("title-4");
                    tool_header.set_xalign(0.0);
                    tool_header.set_margin_start(12);
                    tool_header.set_margin_end(12);
                    tool_header.set_margin_top(8);
                    tool_container.append(&tool_header);
                }

                // Tool state (title and output)
                if let Some(state) = &tool_part.state {
                    if let Some(title) = &state.title {
                        let title_label = Label::new(Some(title));
                        title_label.add_css_class("dim-label");
                        title_label.set_xalign(0.0);
                        title_label.set_margin_start(12);
                        title_label.set_margin_end(12);
                        title_label.set_margin_top(4);
                        tool_container.append(&title_label);
                    }

                    if let Some(output) = &state.output {
                        let output_view = gtk4::TextView::new();
                        output_view.set_editable(false);
                        output_view.set_cursor_visible(false);
                        output_view.set_wrap_mode(gtk4::WrapMode::None);
                        output_view.set_monospace(true);
                        output_view.buffer().set_text(output);

                        let output_scroll = ScrolledWindow::builder()
                            .hexpand(true)
                            .vexpand(false)
                            .min_content_height(100)
                            .max_content_height(300)
                            .propagate_natural_width(false)
                            .propagate_natural_height(true)
                            .margin_start(12)
                            .margin_end(12)
                            .margin_top(8)
                            .margin_bottom(8)
                            .child(&output_view)
                            .build();
                        output_scroll.add_css_class("tool-output");

                        tool_container.append(&output_scroll);
                    }
                }

                tools_box.append(&tool_container);
            }

            message_box.append(&tools_box);
        }

        // Action buttons (Copy, View Source, Info)
        let button_box = GtkBox::new(Orientation::Horizontal, 6);
        button_box.set_margin_top(6);

        // Copy button
        let copy_button = gtk4::Button::from_icon_name("edit-copy-symbolic");
        copy_button.add_css_class("flat");
        copy_button.set_tooltip_text(Some("Copy"));
        let text_for_copy = text.clone();
        copy_button.connect_clicked(move |_| {
            if let Some(display) = gtk4::gdk::Display::default() {
                display.clipboard().set_text(&text_for_copy);
            }
        });
        button_box.append(&copy_button);

        // View Source button
        let source_button = gtk4::Button::from_icon_name("document-properties-symbolic");
        source_button.add_css_class("flat");
        source_button.set_tooltip_text(Some("View Source"));
        let message_for_source = message.clone();
        source_button.connect_clicked(move |_| {
            Self::show_source_dialog(&message_for_source);
        });
        button_box.append(&source_button);

        // Info button
        let info_button = gtk4::Button::from_icon_name("info-outline-symbolic");
        info_button.add_css_class("flat");
        info_button.set_tooltip_text(Some("Message Info"));
        let message_for_info = message.clone();
        let info_sidebar_clone = info_sidebar.clone();
        let split_view_clone = split_view.clone();
        info_button.connect_clicked(move |_| {
            Self::show_info_sidebar(&message_for_info, &info_sidebar_clone, &split_view_clone);
        });
        button_box.append(&info_button);

        message_box.append(&button_box);
        container.append(&message_box);
    }

    fn show_source_dialog(message: &Message) {
        let dialog = adw::MessageDialog::builder()
            .heading("Message Source")
            .build();

        let content_box = GtkBox::new(Orientation::Vertical, 6);
        content_box.set_margin_start(12);
        content_box.set_margin_end(12);
        content_box.set_margin_top(12);
        content_box.set_margin_bottom(12);

        let mut source_text = String::new();
        for (i, part) in message.parts.iter().enumerate() {
            if i > 0 {
                source_text.push_str("\n---\n\n");
            }
            match part.part_type.as_str() {
                "text" => {
                    if let Some(text) = &part.text {
                        source_text.push_str(text);
                    }
                }
                "tool" => {
                    source_text.push_str("[Tool Call]\n");
                    if let Some(tool_name) = &part.tool {
                        source_text.push_str(&format!("Tool: {}\n", tool_name));
                    }
                    if let Some(state) = &part.state {
                        if let Some(title) = &state.title {
                            source_text.push_str(&format!("Title: {}\n", title));
                        }
                        if let Some(output) = &state.output {
                            source_text.push_str(&format!("Output: {}\n", output));
                        }
                    }
                }
                _ => {
                    source_text.push_str(&format!("[{}]\n", part.part_type));
                    if let Some(text) = &part.text {
                        source_text.push_str(text);
                    }
                }
            }
        }

        let text_view = gtk4::TextView::new();
        text_view.set_editable(false);
        text_view.set_cursor_visible(false);
        text_view.set_wrap_mode(gtk4::WrapMode::Word);
        text_view.set_monospace(true);
        text_view.buffer().set_text(&source_text);

        let scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .min_content_height(300)
            .max_content_height(500)
            .min_content_width(500)
            .child(&text_view)
            .build();

        content_box.append(&scrolled);
        dialog.set_extra_child(Some(&content_box));
        dialog.add_response("close", "Close");
        dialog.set_default_response(Some("close"));
        dialog.present();
    }

    fn show_info_sidebar(message: &Message, sidebar: &Rc<RefCell<GtkBox>>, split_view: &Rc<RefCell<adw::OverlaySplitView>>) {
        let sidebar_box = sidebar.borrow();
        
        // Clear previous content
        while let Some(child) = sidebar_box.first_child() {
            sidebar_box.remove(&child);
        }

        // Add close button header
        let header_box = GtkBox::new(Orientation::Horizontal, 6);
        header_box.set_margin_bottom(12);
        
        let heading = Label::new(Some("Message Info"));
        heading.add_css_class("title-2");
        heading.set_hexpand(true);
        heading.set_halign(gtk4::Align::Start);
        header_box.append(&heading);
        
        let close_button = gtk4::Button::from_icon_name("window-close-symbolic");
        close_button.add_css_class("flat");
        let split_view_for_close = split_view.clone();
        close_button.connect_clicked(move |_| {
            split_view_for_close.borrow().set_show_sidebar(false);
        });
        header_box.append(&close_button);
        sidebar_box.append(&header_box);

        let content_box = GtkBox::new(Orientation::Vertical, 12);

        // Message ID
        let message_id_title = Label::new(Some("Message ID"));
        message_id_title.add_css_class("heading");
        message_id_title.set_xalign(0.0);
        content_box.append(&message_id_title);

        let message_id_row = GtkBox::new(Orientation::Horizontal, 6);
        let message_id_value = Label::new(Some(&message.info.id));
        message_id_value.set_selectable(true);
        message_id_value.set_xalign(0.0);
        message_id_value.set_wrap(true);
        message_id_value.set_hexpand(true);
        message_id_row.append(&message_id_value);

        let copy_msg_id_button = gtk4::Button::from_icon_name("edit-copy-symbolic");
        copy_msg_id_button.add_css_class("flat");
        copy_msg_id_button.set_tooltip_text(Some("Copy"));
        let msg_id_for_copy = message.info.id.clone();
        copy_msg_id_button.connect_clicked(move |_| {
            if let Some(display) = gtk4::gdk::Display::default() {
                display.clipboard().set_text(&msg_id_for_copy);
            }
        });
        message_id_row.append(&copy_msg_id_button);
        content_box.append(&message_id_row);

        // Session ID
        if let Some(sid) = &message.info.session_id {
            let session_id_title = Label::new(Some("Session ID"));
            session_id_title.add_css_class("heading");
            session_id_title.set_xalign(0.0);
            content_box.append(&session_id_title);

            let session_id_row = GtkBox::new(Orientation::Horizontal, 6);
            let session_id_value = Label::new(Some(sid));
            session_id_value.set_selectable(true);
            session_id_value.set_xalign(0.0);
            session_id_value.set_wrap(true);
            session_id_value.set_hexpand(true);
            session_id_row.append(&session_id_value);

            let copy_sess_id_button = gtk4::Button::from_icon_name("edit-copy-symbolic");
            copy_sess_id_button.add_css_class("flat");
            copy_sess_id_button.set_tooltip_text(Some("Copy"));
            let sess_id_for_copy = sid.clone();
            copy_sess_id_button.connect_clicked(move |_| {
                if let Some(display) = gtk4::gdk::Display::default() {
                    display.clipboard().set_text(&sess_id_for_copy);
                }
            });
            session_id_row.append(&copy_sess_id_button);
            content_box.append(&session_id_row);
        }

        // Model Section
        if message.info.provider_id.is_some() || message.info.model_id.is_some() || message.info.mode.is_some() {
            let model_label = Label::new(Some("Model"));
            model_label.add_css_class("title-4");
            model_label.set_xalign(0.0);
            model_label.set_margin_top(8);
            content_box.append(&model_label);

            if let Some(provider_id) = &message.info.provider_id {
                let provider_label = Label::new(Some(&format!("Provider: {}", provider_id)));
                provider_label.set_selectable(true);
                provider_label.set_xalign(0.0);
                content_box.append(&provider_label);
            }

            if let Some(model_id) = &message.info.model_id {
                let model_id_label = Label::new(Some(&format!("Model: {}", model_id)));
                model_id_label.set_selectable(true);
                model_id_label.set_xalign(0.0);
                model_id_label.set_wrap(true);
                content_box.append(&model_id_label);
            }

            if let Some(mode) = &message.info.mode {
                let mode_label = Label::new(Some(&format!("Mode: {}", mode)));
                mode_label.set_selectable(true);
                mode_label.set_xalign(0.0);
                content_box.append(&mode_label);
            }
        }

        // Path Section
        if let Some(path) = &message.info.path {
            let path_label = Label::new(Some("Path"));
            path_label.add_css_class("title-4");
            path_label.set_xalign(0.0);
            path_label.set_margin_top(8);
            content_box.append(&path_label);

            let cwd_label = Label::new(Some(&format!("CWD: {}", path.cwd)));
            cwd_label.set_selectable(true);
            cwd_label.set_xalign(0.0);
            cwd_label.set_wrap(true);
            content_box.append(&cwd_label);

            let root_label = Label::new(Some(&format!("Root: {}", path.root)));
            root_label.set_selectable(true);
            root_label.set_xalign(0.0);
            root_label.set_wrap(true);
            content_box.append(&root_label);
        }

        // Usage Section
        if message.info.tokens.is_some() || message.info.cost.is_some() {
            let tokens_label = Label::new(Some("Usage"));
            tokens_label.add_css_class("title-4");
            tokens_label.set_xalign(0.0);
            content_box.append(&tokens_label);

            if let Some(cost) = message.info.cost {
                let cost_label = Label::new(Some(&format!("Cost: ${:.6}", cost)));
                cost_label.set_selectable(true);
                cost_label.set_xalign(0.0);
                content_box.append(&cost_label);
            }

            if let Some(tokens) = &message.info.tokens {
                let input_label = Label::new(Some(&format!("Input: {}", tokens.input)));
                input_label.set_selectable(true);
                input_label.set_xalign(0.0);
                content_box.append(&input_label);

                let output_label = Label::new(Some(&format!("Output: {}", tokens.output)));
                output_label.set_selectable(true);
                output_label.set_xalign(0.0);
                content_box.append(&output_label);

                if tokens.reasoning > 0 {
                    let reasoning_label = Label::new(Some(&format!("Reasoning: {}", tokens.reasoning)));
                    reasoning_label.set_selectable(true);
                    reasoning_label.set_xalign(0.0);
                    content_box.append(&reasoning_label);
                }

                let cache_read_label = Label::new(Some(&format!("Cache Read: {}", tokens.cache.read)));
                cache_read_label.set_selectable(true);
                cache_read_label.set_xalign(0.0);
                content_box.append(&cache_read_label);

                let cache_write_label = Label::new(Some(&format!("Cache Write: {}", tokens.cache.write)));
                cache_write_label.set_selectable(true);
                cache_write_label.set_xalign(0.0);
                content_box.append(&cache_write_label);

                let total = tokens.input as u64 + tokens.output as u64;
                let total_label = Label::new(Some(&format!("Total: {}", total)));
                total_label.set_selectable(true);
                total_label.set_xalign(0.0);
                content_box.append(&total_label);
            }
        }

        sidebar_box.append(&content_box);
        drop(sidebar_box);
        
        // Show the sidebar
        split_view.borrow().set_show_sidebar(true);
    }

    pub fn widget(&self) -> adw::OverlaySplitView {
        self.widget.clone()
    }

    pub fn set_messages(&self, messages: Vec<Message>) {
        self.list_store.remove_all();
        
        for message in messages {
            let message_obj = MessageObject::new(message);
            self.list_store.append(&message_obj);
        }
        
        // Scroll to bottom after a delay
        glib::timeout_add_local_once(std::time::Duration::from_millis(100), {
            let listview = self.listview.clone();
            let list_store = self.list_store.clone();
            move || {
                let n_items = list_store.n_items();
                if n_items > 0 {
                    listview.scroll_to(n_items - 1, gtk4::ListScrollFlags::FOCUS, None);
                }
            }
        });
    }

    pub fn add_message(&self, message: Message) {
        let message_obj = MessageObject::new(message);
        self.list_store.append(&message_obj);
        
        // Scroll to bottom
        glib::idle_add_local_once({
            let listview = self.listview.clone();
            let list_store = self.list_store.clone();
            move || {
                let n_items = list_store.n_items();
                if n_items > 0 {
                    listview.scroll_to(n_items - 1, gtk4::ListScrollFlags::FOCUS, None);
                }
            }
        });
    }

    pub fn update_message(&self, message_id: &str, text: &str) {
        for i in 0..self.list_store.n_items() {
            if let Some(message_obj) = self.list_store.item(i).and_downcast::<MessageObject>() {
                if message_obj.message_id() == message_id {
                    let mut message = message_obj.message();
                    
                    if let Some(part) = message.parts.first_mut() {
                        if let Some(existing_text) = &part.text {
                            part.text = Some(format!("{}{}", existing_text, text));
                        }
                    }
                    
                    message_obj.set_message(message);
                    
                    // Force re-render by removing and re-adding
                    self.list_store.remove(i);
                    self.list_store.insert(i, &message_obj);
                    
                    break;
                }
            }
        }
    }

    pub fn show_thinking(&self) {
        if self.thinking_widget.borrow().is_some() {
            return;
        }

        let thinking_message = crate::api::Message {
            info: crate::api::MessageInfo {
                id: "thinking".to_string(),
                session_id: None,
                role: "thinking".to_string(),
                cost: None,
                tokens: None,
                model_id: None,
                provider_id: None,
                mode: None,
                path: None,
            },
            parts: vec![crate::api::MessagePart {
                text: Some("Generating...".to_string()),
                part_type: "thinking".to_string(),
                tool: None,
                state: None,
            }],
        };

        let message_obj = MessageObject::new(thinking_message);
        self.list_store.append(&message_obj);
        *self.thinking_widget.borrow_mut() = Some(GtkBox::new(Orientation::Horizontal, 0));
        
        // Scroll to bottom to show thinking indicator
        glib::idle_add_local_once({
            let listview = self.listview.clone();
            let list_store = self.list_store.clone();
            move || {
                let n_items = list_store.n_items();
                if n_items > 0 {
                    listview.scroll_to(n_items - 1, gtk4::ListScrollFlags::FOCUS, None);
                }
            }
        });
    }

    pub fn hide_thinking(&self) {
        if self.thinking_widget.borrow().is_some() {
            let n_items = self.list_store.n_items();
            if n_items > 0 {
                if let Some(last_item) = self.list_store.item(n_items - 1).and_downcast::<MessageObject>() {
                    if last_item.message().info.role == "thinking" {
                        self.list_store.remove(n_items - 1);
                    }
                }
            }
            *self.thinking_widget.borrow_mut() = None;
        }
    }
}
