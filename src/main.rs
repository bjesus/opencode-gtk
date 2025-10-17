mod api;
mod window;
mod session_list;
mod chat_view;
mod message_input;
mod markdown;

use clap::Parser;
use gtk4::prelude::*;
use gtk4::{glib, Application};
use libadwaita as adw;

const APP_ID: &str = "com.opencode.gtk";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "localhost:5173")]
    server: String,
}

#[tokio::main]
async fn main() -> glib::ExitCode {
    let args = Args::parse();
    let server_url = if args.server.starts_with("http://") || args.server.starts_with("https://") {
        args.server
    } else {
        format!("http://{}", args.server)
    };

    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(move |app| {
        adw::init().expect("Failed to initialize libadwaita");
        
        let window = window::Window::new(app, server_url.clone());
        window.present();
    });

    app.run_with_args(&Vec::<String>::new())
}
