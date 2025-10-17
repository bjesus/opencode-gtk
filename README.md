# OpenCode GTK

A minimal GTK4/libadwaita desktop client for OpenCode.

## Requirements

- Rust toolchain
- GTK4 development libraries
- libadwaita development libraries

### Installation on Linux

**Fedora/RHEL:**
```bash
sudo dnf install gtk4-devel libadwaita-devel
```

**Ubuntu/Debian:**
```bash
sudo apt install libgtk-4-dev libadwaita-1-dev
```

**Arch:**
```bash
sudo pacman -S gtk4 libadwaita
```

## Building

```bash
cargo build
```

## Running

Make sure the OpenCode server is running first. Then start the GTK client:

By default, connects to OpenCode on `localhost:5173`:
```bash
cargo run
```

Connect to a custom server:
```bash
cargo run -- --server 127.0.0.1:42165
```

Or with a full URL:
```bash
cargo run -- --server http://192.168.1.100:5173
```

## Features

- Session management (create, list, delete)
- Real-time chat with streaming responses via SSE
- Model selection from available providers
- Multi-line message input with Ctrl+Enter to send
- Simple two-pane interface (sessions sidebar + chat view)
