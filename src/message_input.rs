use gtk4::prelude::*;
use gtk4::{glib, Box as GtkBox, Button, DropDown, Orientation, StringList, TextView};
use std::cell::RefCell;
use std::rc::Rc;

use crate::api::{Message, MessageInfo, MessagePart, SendMessageRequest, SharedApiClient};

pub struct MessageInput {
    widget: GtkBox,
    model_dropdown: DropDown,
    api_client: SharedApiClient,
    session_id: Rc<RefCell<Option<String>>>,
    message_sent_callback: Rc<RefCell<Option<Box<dyn Fn(Message)>>>>,
    models: Rc<RefCell<Vec<(String, String, String)>>>,
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
        
        let text_scroll = gtk4::ScrolledWindow::builder()
            .child(&text_view)
            .min_content_height(100)
            .max_content_height(200)
            .vexpand(false)
            .hexpand(true)
            .build();

        let model_list = StringList::new(&["Loading..."]);
        let model_dropdown = DropDown::builder()
            .model(&model_list)
            .build();

        let send_button = Button::builder()
            .label("Send")
            .build();
        send_button.add_css_class("suggested-action");

        let controls_box = GtkBox::new(Orientation::Vertical, 8);
        controls_box.append(&model_dropdown);
        controls_box.append(&send_button);

        widget.append(&text_scroll);
        widget.append(&controls_box);

        let session_id: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let message_sent_callback: Rc<RefCell<Option<Box<dyn Fn(Message)>>>> = Rc::new(RefCell::new(None));
        let models: Rc<RefCell<Vec<(String, String, String)>>> = Rc::new(RefCell::new(Vec::new()));

        {
            let text_view = text_view.clone();
            let model_dropdown = model_dropdown.clone();
            let api_client = api_client.clone();
            let session_id = session_id.clone();
            let message_sent_callback = message_sent_callback.clone();
            let models = models.clone();

            send_button.connect_clicked(move |_| {
                let buffer = text_view.buffer();
                let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                
                if text.is_empty() {
                    eprintln!("Cannot send: message is empty");
                    return;
                }

                let models_vec = models.borrow();
                let selected = model_dropdown.selected();
                if selected as usize >= models_vec.len() {
                    eprintln!("Cannot send: no model selected (selected={}, models={})", selected, models_vec.len());
                    return;
                }

                let (provider_id, model_id, _) = models_vec[selected as usize].clone();
                
                if let Some(sid) = session_id.borrow().as_ref() {
                    let session_id = sid.clone();
                    let text_str = text.to_string();
                    let api_client = api_client.clone();
                    let message_sent_callback = message_sent_callback.clone();

                    buffer.set_text("");

                    let user_message = Message {
                        info: MessageInfo {
                            id: format!("temp-{}", glib::uuid_string_random()),
                            role: "user".to_string(),
                        },
                        parts: vec![MessagePart { 
                            text: Some(text_str.clone()),
                            part_type: "text".to_string(),
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
                            }],
                            model_id: model_id.clone(),
                            provider_id: provider_id.clone(),
                        };

                        eprintln!("API request: parts=[text={}], modelID={}, providerID={}", text_str, model_id, provider_id);

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
                    eprintln!("Cannot send: no session selected");
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
            model_dropdown,
            api_client,
            session_id,
            message_sent_callback,
            models,
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

    pub fn load_models(&mut self) {
        let api_client = self.api_client.clone();
        let model_dropdown = self.model_dropdown.clone();
        let models = self.models.clone();

        glib::MainContext::default().spawn_local(async move {
            let client = api_client.lock().await;
            match client.get_config().await {
                Ok(config) => {
                    drop(client);

                    let mut model_vec = Vec::new();
                    let mut model_names = Vec::new();

                    for provider in config.providers {
                        for (_key, model) in provider.models {
                            model_vec.push((provider.id.clone(), model.id.clone(), model.name.clone()));
                            model_names.push(format!("{} - {}", provider.name, model.name));
                        }
                    }

                    *models.borrow_mut() = model_vec;

                    let string_list = StringList::new(&model_names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
                    model_dropdown.set_model(Some(&string_list));
                }
                Err(e) => {
                    eprintln!("Failed to load models: {}", e);
                }
            }
        });
    }
}
