# Signal-Tauri

A native Rust implementation of a Signal Desktop client built with [egui](https://github.com/emilk/egui) and [presage](https://github.com/whisperfish/presage).

## ✨ Features

- 🔐 **End-to-end encrypted messaging** using the Signal Protocol
- 💬 **Full messaging support**: text, emojis, reactions, replies, quotes
- 📎 **Rich attachments**: images, videos, audio, documents
- 🎙️ **Voice notes** with recording and playback
- 👥 **Group chats** with full management capabilities
- 🔗 **Device linking** via QR code
- 🔔 **Desktop notifications** for new messages
- 🎨 **Native UI** with light and dark themes
- 🖼️ **Avatar support** with sync across devices
- 📋 **Clipboard integration** for easy sharing
- 🔒 **SQLCipher database encryption** for at-rest data security
- 🍎 **macOS Dock integration** with proper app bundle support

## 🏗️ Architecture

Signal-Tauri is organized into several key modules:

### Core Modules

- **`app.rs`**: Main application logic and state management
- **`signal/`**: Signal Protocol implementation
  - `manager.rs`: Connection management and message dispatch
  - `messages.rs`: Message sending and receiving
  - `contacts.rs`: Contact management and syncing
  - `groups.rs`: Group chat functionality
  - `profiles.rs`: User profile management
  - `attachments.rs`: Attachment upload/download
  - `provisioning.rs`: Device linking and registration
- **`storage/`**: Data persistence layer
  - `database.rs`: SQLCipher database management
  - `messages.rs`: Message storage and queries
  - `conversations.rs`: Conversation management
  - `contacts.rs`: Contact storage
  - `settings.rs`: User preferences
  - `encryption.rs`: Database encryption setup
- **`ui/`**: User interface components
  - `views/`: Main application views (chat, settings, etc.)
  - `components/`: Reusable UI components
  - `widgets/`: Custom widgets (emoji picker, voice recorder)
  - `theme.rs`: Theme configuration
  - `avatar_cache.rs`: Avatar image caching
  - `emoji_rasterizer.rs`: Color emoji rendering
- **`services/`**: Background services
  - `sync.rs`: Contact and profile synchronization
  - `notifications.rs`: Desktop notification handling
  - `updates.rs`: Message update processing

## 🚀 Getting Started

### Prerequisites

- **Rust** 1.70 or later
- **SQLCipher** (for database encryption)
- **Platform-specific dependencies**:
  - **macOS**: Xcode Command Line Tools
  - **Linux**:
    - Development headers for: `gtk3`, `atk`, `cairo`, `pango`, `gdk-pixbuf`, `glib`
    - OpenSSL development headers
    - SQLCipher development headers
  - **Windows**: Visual Studio 2019 or later with C++ build tools

### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/signal-tauri.git
cd signal-tauri

# Build the project
cargo build --release

# Run the application
cargo run --release
```

### Development Build

For faster compilation during development:

```bash
cargo run
```

## 📱 Usage

### First Time Setup

1. **Launch the application**
2. **Link as a secondary device**:
   - Open Signal on your primary device (phone)
   - Go to Settings → Linked Devices → Link New Device
   - Scan the QR code displayed in Signal-Tauri
   - Wait for the linking process to complete
3. **Set up database encryption** when prompted (first launch only)

### Daily Use

- **Send messages**: Click on a conversation and type in the message box
- **Send attachments**: Click the attachment button to select files
- **Record voice notes**: Hold the microphone button to record
- **React to messages**: Right-click on a message to react
- **Quote/reply**: Select a message and click the reply button
- **Search**: Use the search bar to find conversations or contacts
- **Settings**: Access via the gear icon to customize themes and preferences

## 🔧 Configuration

Configuration is stored in platform-specific directories:

- **macOS**: `~/Library/Application Support/signal-tauri/`
- **Linux**: `~/.local/share/signal-tauri/`
- **Windows**: `%APPDATA%\signal-tauri\`

### Database Location

The encrypted SQLite database is stored at:
- `<config_dir>/signal-tauri.db`

### Logs

Logs are written to stdout. You can control log levels via the `RUST_LOG` environment variable:

```bash
RUST_LOG=signal_tauri=debug,presage=debug cargo run
```

## 🛠️ Development

### Project Structure

```
signal-tauri/
├── src/
│   ├── app.rs              # Main app state and logic
│   ├── main.rs             # Entry point
│   ├── signal/             # Signal protocol layer
│   ├── storage/            # Database and persistence
│   ├── ui/                 # User interface
│   └── services/           # Background services
├── Cargo.toml              # Rust dependencies
├── LICENSE                 # MIT license
└── README.md               # This file
```

### Key Dependencies

- **[eframe](https://github.com/emilk/egui)**: Cross-platform GUI framework
- **[presage](https://github.com/whisperfish/presage)**: Signal protocol implementation
- **[rusqlite](https://github.com/rusqlite/rusqlite)**: SQLite database with SQLCipher support
- **[tokio](https://tokio.rs/)**: Async runtime
- **[serde](https://serde.rs/)**: Serialization/deserialization

### Building for Release

The release profile is optimized for size and performance:

```bash
cargo build --release
```

Release builds include:
- Link-time optimization (LTO)
- Dead code stripping
- Size optimization (`opt-level = "z"`)
- Panic abort strategy

## 🤝 Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

### Development Guidelines

1. Follow Rust's standard formatting (`cargo fmt`)
2. Ensure code passes linting (`cargo clippy`)
3. Write tests for new functionality
4. Update documentation as needed

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

Note: The Cargo.toml file mentions AGPL-3.0, but the LICENSE file contains MIT license text. Please clarify the intended license.

## ⚠️ Disclaimer

This is an **unofficial** Signal client implementation. It is not affiliated with or endorsed by Signal Messenger LLC. Use at your own risk.

- This client uses the Signal protocol but is not officially supported
- Some features may not be fully compatible with official Signal clients
- Data encryption and security are implemented but have not been formally audited

## 🙏 Acknowledgments

- [Signal](https://signal.org/) for the excellent messaging protocol
- [Whisperfish](https://gitlab.com/whisperfish/whisperfish) for the presage library
- [egui](https://github.com/emilk/egui) for the immediate mode GUI framework
- All the amazing open source contributors

## 📞 Support

If you encounter issues:

1. Check the [Issues](https://github.com/yourusername/signal-tauri/issues) page
2. Enable debug logging: `RUST_LOG=debug cargo run`
3. Review the logs for error messages
4. Create a new issue with:
   - Your OS and version
   - Steps to reproduce
   - Relevant log output (redact sensitive information)

## 🗺️ Roadmap

- [ ] Message search functionality
- [ ] Sticker support
- [ ] Video/audio calls (if feasible)
- [ ] Message export functionality
- [ ] Multi-account support
- [ ] Custom notification sounds
- [ ] Message deletion and editing
- [ ] Backup and restore

---

**Built with ❤️ using Rust and egui**
