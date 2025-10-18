use gtk4::prelude::*;
use gtk4::{glib, Box as GtkBox, Button, Orientation, TextView};
use std::cell::RefCell;
use std::rc::Rc;

use crate::api::{Agent, Message, MessageInfo, MessagePart, SendMessageRequest, SharedApiClient};
use gtk4::gio::prelude::ActionMapExt;

pub struct MessageInput {
    widget: GtkBox,
    model_button: gtk4::MenuButton,
    model_menu: gtk4::gio::Menu,
    agent_button: gtk4::MenuButton,
    agent_menu: gtk4::gio::Menu,
    api_client: SharedApiClient,
    session_id: Rc<RefCell<Option<String>>>,
    message_sent_callback: Rc<RefCell<Option<Box<dyn Fn(Message)>>>>,
    no_session_callback: Rc<RefCell<Option<Box<dyn Fn(String, String, String, String)>>>>,
    models: Rc<RefCell<Vec<(String, String, String)>>>,
    selected_model: Rc<RefCell<usize>>,
    agents: Rc<RefCell<Vec<Agent>>>,
    selected_agent: Rc<RefCell<usize>>,
    agent_action: Rc<RefCell<Option<gtk4::gio::SimpleAction>>>,
}

impl MessageInput {
    pub fn new(api_client: SharedApiClient) -> Self {
        let widget = GtkBox::new(Orientation::Horizontal, 8);
        widget.set_margin_start(12);
        widget.set_margin_end(12);
        widget.set_margin_top(8);
        widget.set_margin_bottom(12);

        let text_view = TextView::builder()
            .wrap_mode(gtk4::WrapMode::Word)
            .accepts_tab(false)
            .build();
        text_view.set_vexpand(true);
        text_view.set_hexpand(true);
        
        let buffer = text_view.buffer();
        let placeholder_text = "Type a message...";
        buffer.set_text(placeholder_text);
        
        let focus_in_controller = gtk4::EventControllerFocus::new();
        focus_in_controller.connect_enter({
            let text_view = text_view.clone();
            let placeholder = placeholder_text.to_string();
            move |_| {
                let buffer = text_view.buffer();
                let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                if text.as_str() == placeholder {
                    buffer.set_text("");
                }
            }
        });
        text_view.add_controller(focus_in_controller);
        
        let focus_out_controller = gtk4::EventControllerFocus::new();
        focus_out_controller.connect_leave({
            let text_view = text_view.clone();
            let placeholder = placeholder_text.to_string();
            move |_| {
                let buffer = text_view.buffer();
                let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                if text.trim().is_empty() {
                    buffer.set_text(&placeholder);
                }
            }
        });
        text_view.add_controller(focus_out_controller);
        
        text_view.set_margin_start(12);
        text_view.set_margin_end(12);
        text_view.set_margin_top(8);
        text_view.set_margin_bottom(8);
        
        let text_scroll = gtk4::ScrolledWindow::builder()
            .child(&text_view)
            .vexpand(false)
            .hexpand(true)
            .propagate_natural_height(true)
            .build();
        
        let buffer_clone = buffer.clone();
        let text_scroll_clone = text_scroll.clone();
        buffer.connect_changed(move |buffer| {
            let line_count = buffer.line_count();
            let single_line_height = 11;
            let max_lines = 5;
            
            let height = (line_count.min(max_lines) as i32) * single_line_height;
            text_scroll_clone.set_min_content_height(height);
            text_scroll_clone.set_max_content_height(max_lines * single_line_height);
        });
        
        text_scroll.set_min_content_height(11);
        text_scroll.set_max_content_height(11 * 5);
        
        let model_menu = gtk4::gio::Menu::new();
        let model_button = gtk4::MenuButton::builder()
            .icon_name("pan-up-symbolic")
            .menu_model(&model_menu)
            .label("Select Model")
            .direction(gtk4::ArrowType::Up)
            .build();
        model_button.set_halign(gtk4::Align::Start);
        model_button.add_css_class("flat");
        model_button.add_css_class("caption");
        model_button.set_margin_start(8);
        model_button.set_margin_bottom(4);
        
        let agent_menu = gtk4::gio::Menu::new();
        let agent_button = gtk4::MenuButton::builder()
            .icon_name("pan-up-symbolic")
            .menu_model(&agent_menu)
            .label("Select Agent")
            .direction(gtk4::ArrowType::Up)
            .build();
        agent_button.set_halign(gtk4::Align::Start);
        agent_button.add_css_class("flat");
        agent_button.add_css_class("caption");
        agent_button.set_margin_start(8);
        agent_button.set_margin_bottom(4);
        
        let selector_box = GtkBox::new(Orientation::Horizontal, 12);
        selector_box.append(&model_button);
        selector_box.append(&agent_button);
        selector_box.set_margin_start(8);
        selector_box.set_margin_bottom(4);
        
        let frame_content = GtkBox::new(Orientation::Vertical, 0);
        frame_content.append(&text_scroll);
        frame_content.append(&selector_box);
        
        let text_frame = gtk4::Frame::new(None);
        text_frame.set_child(Some(&frame_content));
        text_frame.add_css_class("view");

        let send_button = Button::builder()
            .label("Send")
            .sensitive(false)
            .build();
        send_button.add_css_class("suggested-action");

        widget.append(&text_frame);
        widget.append(&send_button);

        // Add key event controller for Enter/Shift+Enter
        {
            let send_button_for_key = send_button.clone();
            let key_controller = gtk4::EventControllerKey::new();
            key_controller.connect_key_pressed(move |_, key, _, modifier| {
                if key == gtk4::gdk::Key::Return || key == gtk4::gdk::Key::KP_Enter {
                    // If Shift is pressed, allow normal newline behavior
                    if modifier.contains(gtk4::gdk::ModifierType::SHIFT_MASK) {
                        return glib::Propagation::Proceed;
                    }
                    
                    // Otherwise, trigger send button if it's sensitive
                    if send_button_for_key.is_sensitive() {
                        send_button_for_key.emit_clicked();
                    }
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            });
            text_view.add_controller(key_controller);
        }

        let session_id: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let message_sent_callback: Rc<RefCell<Option<Box<dyn Fn(Message)>>>> = Rc::new(RefCell::new(None));
        let no_session_callback: Rc<RefCell<Option<Box<dyn Fn(String, String, String, String)>>>> = Rc::new(RefCell::new(None));
        let models: Rc<RefCell<Vec<(String, String, String)>>> = Rc::new(RefCell::new(Vec::new()));
        let selected_model: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
        let agents: Rc<RefCell<Vec<Agent>>> = Rc::new(RefCell::new(Vec::new()));
        let selected_agent: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
        let agent_action: Rc<RefCell<Option<gtk4::gio::SimpleAction>>> = Rc::new(RefCell::new(None));

        {
            let send_button = send_button.clone();
            let buffer = text_view.buffer();
            let placeholder = placeholder_text.to_string();
            
            buffer.connect_changed(move |buffer| {
                let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                let is_empty = text.trim().is_empty() || text.as_str() == placeholder;
                send_button.set_sensitive(!is_empty);
            });
        }

        {
            let text_view = text_view.clone();
            let api_client = api_client.clone();
            let session_id = session_id.clone();
            let message_sent_callback = message_sent_callback.clone();
            let models = models.clone();
            let selected_model = selected_model.clone();
            let agents = agents.clone();
            let selected_agent = selected_agent.clone();
            let api_client = api_client.clone();
            let message_sent_callback = message_sent_callback.clone();
            let no_session_callback = no_session_callback.clone();

            send_button.connect_clicked(move |_| {
                let buffer = text_view.buffer();
                let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                
                if text.is_empty() {
                    eprintln!("Cannot send: message is empty");
                    return;
                }

                let models_vec = models.borrow();
                let selected = *selected_model.borrow();
                if selected >= models_vec.len() {
                    eprintln!("Cannot send: no model selected (selected={}, models={})", selected, models_vec.len());
                    return;
                }

                let (provider_id, model_id, _) = models_vec[selected].clone();
                
                let agents_vec = agents.borrow();
                let selected_agent_idx = *selected_agent.borrow();
                if selected_agent_idx >= agents_vec.len() {
                    eprintln!("Cannot send: no agent selected (selected={}, agents={})", selected_agent_idx, agents_vec.len());
                    return;
                }
                
                let agent_name = agents_vec[selected_agent_idx].name.clone();
                
                if let Some(sid) = session_id.borrow().as_ref() {
                    let session_id = sid.clone();
                    let text_str = text.to_string();
                    let api_client = api_client.clone();
                    let message_sent_callback = message_sent_callback.clone();

                    buffer.set_text("");

                    let user_message = Message {
                        info: MessageInfo {
                            id: format!("temp-{}", glib::uuid_string_random()),
                            session_id: Some(session_id.clone()),
                            role: "user".to_string(),
                            cost: None,
                            tokens: None,
                            model_id: None,
                            provider_id: None,
                            mode: None,
                            path: None,
                        },
                        parts: vec![MessagePart { 
                            text: Some(text_str.clone()),
                            part_type: "text".to_string(),
                            tool: None,
                            state: None,
                        }],
                    };

                    if let Some(callback) = message_sent_callback.borrow().as_ref() {
                        callback(user_message);
                    }

                    eprintln!("Sending message to session: {}", session_id);
                    eprintln!("Using model: {} from provider: {}", model_id, provider_id);
                    
                    glib::MainContext::default().spawn_local(async move {
                        let request = SendMessageRequest {
                            parts: vec![MessagePart {
                                text: Some(text_str.clone()),
                                part_type: "text".to_string(),
                                tool: None,
                                state: None,
                            }],
                            model: crate::api::ModelInfo {
                                model_id: model_id.clone(),
                                provider_id: provider_id.clone(),
                            },
                            agent: agent_name.clone(),
                        };

                        eprintln!("API request: parts=[text={}], model={{modelID={}, providerID={}}}", text_str, model_id, provider_id);

                        let client = api_client.lock().await;
                        match client.send_message(&session_id, request).await {
                            Ok(_) => {
                                eprintln!("Message sent successfully!");
                            }
                            Err(e) => {
                                eprintln!("Failed to send message: {} (Details: {:?})", e, e);
                            }
                        }
                    });
                } else {
                    if let Some(callback) = no_session_callback.borrow().as_ref() {
                        let text_str = text.to_string();
                        callback(text_str, provider_id, model_id, agent_name);
                        buffer.set_text("");
                    } else {
                        eprintln!("Cannot send: no session selected and no callback set");
                    }
                }
            });
        }

        {
            let send_button = send_button.clone();

            let key_controller = gtk4::EventControllerKey::new();
            key_controller.connect_key_pressed(move |_, key, _, modifier| {
                if key == gtk4::gdk::Key::Return && modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                    send_button.emit_clicked();
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            });
            text_view.add_controller(key_controller);
        }

        Self {
            widget,
            model_button,
            model_menu,
            agent_button,
            agent_menu,
            api_client,
            session_id,
            message_sent_callback,
            no_session_callback,
            models,
            selected_model,
            agents,
            selected_agent,
            agent_action,
        }
    }

    pub fn widget(&self) -> GtkBox {
        self.widget.clone()
    }

    pub fn set_session_id(&mut self, session_id: Option<String>) {
        *self.session_id.borrow_mut() = session_id;
    }

    pub fn connect_message_sent<F: Fn(Message) + 'static>(&mut self, callback: F) {
        *self.message_sent_callback.borrow_mut() = Some(Box::new(callback));
    }

    pub fn connect_no_session<F: Fn(String, String, String, String) + 'static>(&mut self, callback: F) {
        *self.no_session_callback.borrow_mut() = Some(Box::new(callback));
    }

    pub fn set_model_by_id(&mut self, model_id: &str) {
        let models = self.models.borrow();
        if let Some(index) = models.iter().position(|(_, id, _)| id == model_id) {
            *self.selected_model.borrow_mut() = index;
            if let Some((_, _, model_name)) = models.get(index) {
                self.model_button.set_label(model_name);
            }
        }
    }

    pub fn set_agent_by_name(&mut self, agent_name: &str) {
        // First, find the index and get the name while we have the borrow
        let agents = self.agents.borrow();
        let index_and_name = agents.iter()
            .position(|a| a.name == agent_name)
            .and_then(|idx| agents.get(idx).map(|a| (idx, a.name.clone())));
        drop(agents); // Drop the borrow early to avoid conflicts with the callback
        
        if let Some((index, name)) = index_and_name {
            *self.selected_agent.borrow_mut() = index;
            self.agent_button.set_label(&name);
            
            // Now update the action state - all borrows are dropped
            if let Some(action) = self.agent_action.borrow().as_ref() {
                action.set_state(&(index as i32).to_variant());
            }
        }
    }

    pub fn load_models(&mut self) {
        let api_client = self.api_client.clone();
        let model_menu = self.model_menu.clone();
        let models = self.models.clone();
        let selected_model = self.selected_model.clone();
        let model_button = self.model_button.clone();

        glib::MainContext::default().spawn_local(async move {
            let client = api_client.lock().await;
            match client.get_config().await {
                Ok(config) => {
                    drop(client);

                    let mut model_vec = Vec::new();

                    for provider in config.providers {
                        for (model_key, model) in provider.models {
                            // Use the HashMap key as the model ID for API requests
                            eprintln!("Loading model: key='{}', name='{}', provider='{}'", model_key, model.name, provider.id);
                            model_vec.push((provider.id.clone(), model_key, model.name.clone()));
                        }
                    }

                    *models.borrow_mut() = model_vec.clone();

                    model_menu.remove_all();
                    
                    let action_group = gtk4::gio::SimpleActionGroup::new();
                    let state_action = gtk4::gio::SimpleAction::new_stateful(
                        "select-model",
                        Some(&glib::VariantTy::new("i").unwrap()),
                        &0i32.to_variant()
                    );
                    
                    let selected_model_clone = selected_model.clone();
                    let model_button_clone = model_button.clone();
                    let models_clone = models.clone();
                    state_action.connect_change_state(move |action, value| {
                        if let Some(index) = value.and_then(|v| v.get::<i32>()) {
                            *selected_model_clone.borrow_mut() = index as usize;
                            action.set_state(value.unwrap());
                            
                            if let Some((provider_id, model_id, model_name)) = models_clone.borrow().get(index as usize) {
                                eprintln!("Model selected: index={}, provider='{}', model_id='{}', name='{}'", 
                                    index, provider_id, model_id, model_name);
                                model_button_clone.set_label(model_name);
                            }
                        }
                    });
                    
                    action_group.add_action(&state_action);
                    
                    for (index, (_provider_id, _model_id, model_name)) in model_vec.iter().enumerate() {
                        let item = gtk4::gio::MenuItem::new(
                            Some(model_name),
                            Some(&format!("models.select-model"))
                        );
                        item.set_attribute_value("target", Some(&(index as i32).to_variant()));
                        model_menu.append_item(&item);
                    }
                    
                    model_button.insert_action_group("models", Some(&action_group));
                    
                    if !model_vec.is_empty() {
                        model_button.set_label(&model_vec[0].2);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load models: {}", e);
                }
            }
        });
    }

    pub fn load_agents(&mut self) {
        let api_client = self.api_client.clone();
        let agent_menu = self.agent_menu.clone();
        let agents = self.agents.clone();
        let selected_agent = self.selected_agent.clone();
        let agent_button = self.agent_button.clone();
        let agent_action = self.agent_action.clone();

        glib::MainContext::default().spawn_local(async move {
            let client = api_client.lock().await;
            match client.get_agents().await {
                Ok(agent_list) => {
                    drop(client);

                    eprintln!("Loaded {} agents", agent_list.len());
                    for agent in &agent_list {
                        let desc = agent.description.as_deref().unwrap_or("no description");
                        eprintln!("  Agent: name='{}', mode='{}', description='{}'", agent.name, agent.mode, desc);
                    }

                    *agents.borrow_mut() = agent_list.clone();

                    agent_menu.remove_all();
                    
                    let action_group = gtk4::gio::SimpleActionGroup::new();
                    let state_action = gtk4::gio::SimpleAction::new_stateful(
                        "select-agent",
                        Some(&glib::VariantTy::new("i").unwrap()),
                        &0i32.to_variant()
                    );
                    
                    let selected_agent_clone = selected_agent.clone();
                    let agent_button_clone = agent_button.clone();
                    let agents_clone = agents.clone();
                    state_action.connect_change_state(move |action, value| {
                        if let Some(index) = value.and_then(|v| v.get::<i32>()) {
                            *selected_agent_clone.borrow_mut() = index as usize;
                            action.set_state(value.unwrap());
                            
                            if let Some(agent) = agents_clone.borrow().get(index as usize) {
                                eprintln!("Agent selected: index={}, name='{}', mode='{}'", 
                                    index, agent.name, agent.mode);
                                agent_button_clone.set_label(&agent.name);
                            }
                        }
                    });
                    
                    action_group.add_action(&state_action);
                    
                    // Store the action so we can update it later without triggering callbacks
                    *agent_action.borrow_mut() = Some(state_action);
                    
                    for (index, agent) in agent_list.iter().enumerate() {
                        let item = gtk4::gio::MenuItem::new(
                            Some(&agent.name),
                            Some(&format!("agents.select-agent"))
                        );
                        item.set_attribute_value("target", Some(&(index as i32).to_variant()));
                        agent_menu.append_item(&item);
                    }
                    
                    agent_button.insert_action_group("agents", Some(&action_group));
                    
                    if !agent_list.is_empty() {
                        agent_button.set_label(&agent_list[0].name);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load agents: {}", e);
                }
            }
        });
    }
}
