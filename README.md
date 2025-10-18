# OpenCode GTK

A modern GTK4/libadwaita desktop client for OpenCode.

<img width="2666" height="1844" alt="image" src="https://github.com/user-attachments/assets/428f6b48-021d-4f84-83f8-67975a9cb161" />

## Features

- Session management (create, list, delete, fork, rename)
- Real-time chat with streaming responses via SSE
- Model and Agent selector
- Show message information like tokens count, costs etc
- Responsive interface

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

OpenCode GTK will automatically launch `opencode serve` unless you specify a server using the `--server` flag.

```bash
cargo run
```

Connect to a custom server:
```bash
cargo run -- --server 127.0.0.1:42165
```
