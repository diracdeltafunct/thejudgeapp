# The Judge App

An offline-first MTG judge utility for Android. Provides a hyperlinked Comprehensive Rules, Tournament Rules, Infraction Procedure Guide, oracle text search, deck counter, draft calling guide, and quick references.

**Stack:** Rust (Tauri v2) backend · Vanilla JS + Vite frontend · SQLite (FTS5)

---

## Environment Setup

### Prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| Rust | stable (MSVC) | `rustup default stable-x86_64-pc-windows-msvc` |
| Node.js | 22+ | Add `C:\Program Files\nodejs` to System PATH |
| Android Studio | latest | For SDK Manager and emulator |
| MSVC Build Tools | 2022 | "Desktop development with C++" workload |

### 1. Rust toolchain

```powershell
# Install MSVC toolchain (required — GNU toolchain hits DLL export limits)
rustup toolchain install stable-x86_64-pc-windows-msvc
rustup default stable-x86_64-pc-windows-msvc

# Add Android cross-compilation targets
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
```

### 2. Android SDK

Open Android Studio → **Settings → Languages & Frameworks → Android SDK**

- **SDK Platforms:** Android 14 (API 34) or higher
- **SDK Tools:** NDK (Side by side), Android SDK Command-line Tools, Platform-Tools, Build-Tools

### 3. Environment variables

Add these to your Windows System environment variables (not just user):

```
ANDROID_HOME=C:\Users\<you>\AppData\Local\Android\Sdk
JAVA_HOME=C:\Program Files\Android\Android Studio\jbr
NDK_HOME=%ANDROID_HOME%\ndk\28.0.13004108
```

Also add to System PATH:
```
C:\Program Files\nodejs
%ANDROID_HOME%\platform-tools
```

Restart your terminal after changing environment variables.

### 4. Node dependencies

```powershell
npm install
```

### 5. Windows Defender exclusion

Freshly compiled binaries in `src-tauri/target/` are sometimes blocked. Add an exclusion:

```powershell
# Run as Administrator
Add-MpPreference -ExclusionPath "C:\Users\<you>\software\thejudgeapp\src-tauri\target"
```

---

## Development

### Desktop (fast iteration)

```powershell
npx tauri dev
```

Launches a desktop window with hot-reload. Use this for most development — no emulator needed.

### Android

```powershell
# First time only — generates the Android project in src-tauri/gen/android/
npx tauri android init

# Deploy to connected device or running emulator
npx tauri android dev

# Build a release APK/AAB
npx tauri android build
```

### Run tests

```powershell
cargo test
```

---

## Updating Rules Data

Rules data is stored in SQLite at `%APPDATA%\thejudgeapp\judge.db`. The update scripts download and parse the official documents into the database.

### Comprehensive Rules (CR)

```powershell
# Download latest CR from Wizards and import into the app database
cargo run --bin update_cr

# Import from a local file instead
cargo run --bin update_cr -- --file path\to\MagicCompRules.txt

# Use a custom database path
cargo run --bin update_cr -- --db path\to\judge.db

# Combine flags
cargo run --bin update_cr -- --url https://... --db path\to\judge.db
```

The CR URL is hardcoded in `src-tauri/src/bin/update_cr.rs`. When Wizards publishes a new set, update the `CR_URL` constant to point to the new file.

### Finding the latest CR URL

The current rules document and its URL are listed at:
**https://magic.wizards.com/en/rules**

The URL pattern is:
```
https://media.wizards.com/<year>/downloads/MagicCompRules%20<YYYYMMDD>.txt
```

### Magic Tournament Rules (MTR)

```powershell
# Download latest MTR PDF from Wizards and import into the app database
cargo run --bin update_mtr

# Import from a locally downloaded PDF instead
cargo run --bin update_mtr -- --file path\to\MTG_MTR.pdf

# Use a custom database path
cargo run --bin update_mtr -- --db path\to\judge.db

# Dump raw extracted PDF text for debugging (does not import)
cargo run --bin update_mtr -- --dump mtr_extracted.txt
```

The MTR URL is hardcoded in `src-tauri/src/bin/update_mtr.rs`. The latest PDF is published at:
**https://magic.wizards.com/en/resources/rules**

The URL pattern is:
```
https://media.wizards.com/ContentResources/WPN/MTG_MTR_<YYYY>_<MonDD>_EN.pdf
```

---

## Project Structure

```
thejudgeapp/
├── Cargo.toml               # Workspace root
├── package.json             # npm deps (Tauri CLI, Vite, @tauri-apps/api)
├── vite.config.js
├── index.html               # App entry point
├── src/                     # Frontend (vanilla JS)
│   ├── main.js              # Router + Tauri IPC calls
│   ├── styles/base.css
│   └── pages/               # rules-viewer, card-search, deck-counter, etc.
└── src-tauri/               # Rust backend
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── src/
    │   ├── lib.rs           # Tauri setup, AppState
    │   ├── main.rs          # Desktop entry point
    │   ├── bin/
    │   │   ├── update_cr.rs # CR import script
    │   │   └── update_mtr.rs # MTR import script
    │   ├── commands/        # Tauri IPC handlers
    │   ├── db/              # SQLite schema + repositories
    │   ├── models/          # Shared data types
    │   ├── parser/          # CR/MTR/IPG text parsers
    │   ├── search/          # FTS5 utilities
    │   └── sync/            # Online update logic
    └── gen/android/         # Generated Android project (git-ignored)
```

---

## Database

SQLite at `%APPDATA%\thejudgeapp\judge.db` (Windows) or `~/.local/share/thejudgeapp/judge.db` (Linux).

Key tables:

| Table | Contents |
|-------|----------|
| `documents` | One row per imported document (CR, MTR, IPG) with version |
| `rules` | All rules and section headers with pre-rendered hyperlinked HTML |
| `rules_fts` | FTS5 virtual table for full-text rule search |
| `glossary` | CR glossary terms and definitions |
| `glossary_fts` | FTS5 virtual table for glossary search |
| `cards` | Scryfall oracle card data (populated separately) |
| `cards_fts` | FTS5 virtual table for card search |

Rules are stored with `body_html` pre-rendered at import time — cross-references like "see rule 704.5k" become `<a href="#R704.5k">` links. The frontend renders this HTML directly.
