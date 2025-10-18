use gtk4::glib;
use gtk4::subclass::prelude::*;
use std::cell::RefCell;

use crate::api::Message;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct MessageObject {
        pub message: RefCell<Option<Message>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MessageObject {
        const NAME: &'static str = "OpencodeMessageObject";
        type Type = super::MessageObject;
    }

    impl ObjectImpl for MessageObject {}
}

glib::wrapper! {
    pub struct MessageObject(ObjectSubclass<imp::MessageObject>);
}

impl MessageObject {
    pub fn new(message: Message) -> Self {
        let obj: Self = glib::Object::new();
        obj.imp().message.replace(Some(message));
        obj
    }

    pub fn message(&self) -> Message {
        self.imp().message.borrow().as_ref().unwrap().clone()
    }

    pub fn message_id(&self) -> String {
        self.message().info.id.clone()
    }

    pub fn set_message(&self, message: Message) {
        self.imp().message.replace(Some(message));
    }
}
