mod api;
mod window;
mod session_list;
mod chat_view;
mod message_input;
mod markdown;
mod message_object;

use clap::Parser;
use gtk4::prelude::*;
use gtk4::{glib, Application};
use libadwaita as adw;

const APP_ID: &str = "com.opencode.gtk";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// OpenCode server URL (will auto-launch 'opencode serve' if not provided or unreachable)
    #[arg(short, long)]
    server: Option<String>,
}

async fn check_server(url: &str) -> bool {
    let client = reqwest::Client::new();
    match client.get(format!("{}/health", url)).send().await {
        Ok(_) => true,
        Err(_) => false,
    }
}

async fn launch_server() -> Option<String> {
    use std::process::Stdio;
    use tokio::io::{BufReader, AsyncBufReadExt};
    use tokio::process::Command;
    
    eprintln!("Server not reachable, launching opencode serve...");
    
    let mut child = match Command::new("opencode")
        .arg("serve")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn() {
            Ok(child) => child,
            Err(e) => {
                eprintln!("Failed to launch opencode serve: {}", e);
                return None;
            }
        };
    
    let stdout = child.stdout.take()?;
    let stderr = child.stderr.take()?;
    
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();
    
    tokio::spawn(async move {
        let _ = child.wait().await;
    });
    
    loop {
        tokio::select! {
            Ok(Some(line)) = stdout_reader.next_line() => {
                eprintln!("opencode stdout: {}", line);
                if line.contains("listening on") {
                    if let Some(url_start) = line.find("http://") {
                        let url = line[url_start..].trim();
                        eprintln!("Detected server URL: {}", url);
                        return Some(url.to_string());
                    }
                }
            }
            Ok(Some(line)) = stderr_reader.next_line() => {
                eprintln!("opencode stderr: {}", line);
                if line.contains("listening on") {
                    if let Some(url_start) = line.find("http://") {
                        let url = line[url_start..].trim();
                        eprintln!("Detected server URL: {}", url);
                        return Some(url.to_string());
                    }
                }
            }
            else => break,
        }
    }
    
    None
}

#[tokio::main]
async fn main() -> glib::ExitCode {
    let args = Args::parse();
    
    let server_url = if let Some(server) = args.server {
        // User provided a server URL
        let url = if server.starts_with("http://") || server.starts_with("https://") {
            server
        } else {
            format!("http://{}", server)
        };
        
        // Check if the provided server is reachable
        if !check_server(&url).await {
            eprintln!("Server not reachable at {}", url);
            return glib::ExitCode::FAILURE;
        }
        url
    } else {
        // No server provided, auto-launch opencode serve
        if let Some(launched_url) = launch_server().await {
            launched_url
        } else {
            eprintln!("Failed to launch opencode server");
            return glib::ExitCode::FAILURE;
        }
    };

    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(move |app| {
        adw::init().expect("Failed to initialize libadwaita");
        
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(
            r#"
            .code-block {
                background-color: alpha(@theme_fg_color, 0.05);
                border-left: 3px solid alpha(@accent_bg_color, 0.5);
                padding: 12px;
                margin: 6px 0;
                border-radius: 6px;
                font-family: monospace;
            }
            
            .tool-output {
                background-color: alpha(@theme_fg_color, 0.03);
                border-left: 2px solid alpha(@theme_fg_color, 0.2);
                border-radius: 0;
                box-shadow: none;
                padding-left: 8px;
            }
            "#
        );
        
        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        
        let window = window::Window::new(app, server_url.clone());
        window.present();
    });

    app.run_with_args(&Vec::<String>::new())
}
