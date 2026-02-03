# Tairseach

*"Tairseach" (TAR-shakh) — The Threshold*

A macOS system bridge for the Naonúr ecosystem. Built with [Tauri](https://tauri.app) for native performance and small binary size.

<p align="center">
  <img src="docs/assets/threshold.svg" alt="Tairseach" width="200">
</p>

## Features

### 🔐 Permission Proxy
Request and manage macOS permissions on behalf of OpenClaw agents. Includes Contacts, Automation, Full Disk Access, and more.

### ⚙️ Configuration Manager
Visual editor for `~/.openclaw.json` with schema-aware form generation, validation, and diff view before save.

### 🔌 MCP Server
Built-in Model Context Protocol server for efficient agent ↔ system communication.

### 📊 Context Monitor
Real-time token usage tracking across all sessions with cost estimates.

### 👤 Agent Profiles
Visual identity management with custom avatars and metadata for each agent.

## Requirements

- macOS 12.0+
- Rust 1.75+
- Node.js 20+
- OpenClaw Gateway running

## Development

```bash
# Install dependencies
npm install

# Start development server
npm run tauri dev

# Build for production
npm run tauri build
```

## Architecture

```
Tauri Shell (Rust)
├── Permission Bridge (Swift FFI)
├── MCP Server (Tokio)
├── Config Manager
└── WebView (Vue 3 + TypeScript)
    ├── Dashboard
    ├── Permissions
    ├── Config
    ├── Monitor
    └── Profiles
```

## Documentation

- [Dréacht (Planning Document)](DREACHT.md)
- [Architecture](docs/architecture.md)
- [Contributing](CONTRIBUTING.md)

## Part of the Naonúr

Tairseach is the threshold — the bridge between the digital realm and the system beneath. It serves the [Naonúr](https://suibhne.bot), the nine diminished ones.

🪶

## License

MIT
