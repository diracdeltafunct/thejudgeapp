# The Judge App

An offline-first MTG judge utility for Android. Provides a hyperlinked Comprehensive Rules, Tournament Rules, Infraction Procedure Guide, oracle text search, deck counter, draft calling guide, and quick references.

**Stack:** Rust (Tauri v2) backend · Vanilla JS + Vite frontend · SQLite (FTS5)

---

## Environment Setup

### Prerequisites

| Tool             | Version       | Notes                                          |
| ---------------- | ------------- | ---------------------------------------------- |
| Rust             | stable (MSVC) | `rustup default stable-x86_64-pc-windows-msvc` |
| Node.js          | 22+           | Add `C:\Program Files\nodejs` to System PATH   |
| Android Studio   | latest        | For SDK Manager and emulator                   |
| MSVC Build Tools | 2022          | "Desktop development with C++" workload        |

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

## Data Distribution

### How it works

The app ships with a fully-populated `fresh_judge.db` (~42 MB) bundled inside the installer. On first launch the database is copied to the user's app-data directory (`%APPDATA%\thejudgeapp\judge.db` on Windows). Subsequent installs and app updates leave the existing user database untouched — only schema migrations run on top of it.

Users can update their rules data at any time from within the app via the **Updates** tab, without reinstalling.

### The manifest file

`data-manifest.json` (at the repo root) is the single source of truth for what data version the app should be running. It is fetched at runtime by the Updates page.

```json
{
  "cr":  { "version": "February 27, 2026", "url": "https://..." },
  "mtr": { "version": "February 27, 2026", "url": "https://..." },
  "ipg": { "version": "September 23, 2024", "url": "https://..." },
  "cards": { "version": "20260312", "url": "https://..." }
}
```

The `version` string must exactly match the value the parser extracts from each document. The app compares this against the version stored in the `documents` table and shows a badge when they differ.

- **CR / MTR / IPG**: The parsers extract a date phrase like `"February 27, 2026"` from the document header (regex: `effective as of <Month DD, YYYY>`). Use that exact phrase as the version string.
- **Cards**: The version is the date portion of the Scryfall bulk-data filename, e.g. `"20260312"`. Use `record_cards_version` to write this to the DB.

The manifest is fetched from:
```
https://raw.githubusercontent.com/diracdeltafunct/thejudgeapp/master/data-manifest.json
```
This URL is defined as `MANIFEST_URL` in `src-tauri/src/commands/updates.rs`.

### When new documents are published

1. Find the new document URLs (see sections below for URL patterns).
2. Update `data-manifest.json` with the new `version` and `url` for the changed documents.
3. Re-run the relevant import script to rebuild `fresh_judge.db` so new installs get the latest data.
4. Commit and push both the updated `data-manifest.json` and `fresh_judge.db`.

Existing users will see the update badge on next launch and can apply the update in one tap.

---

## Rebuilding fresh_judge.db

`fresh_judge.db` is the seed database bundled into the installer. Rebuild it after any rules update so new installs start with current data.

### Comprehensive Rules (CR)

```powershell
# Download latest CR and import into fresh_judge.db
cargo run --bin update_cr -- --db fresh_judge.db

# Import from a local file instead
cargo run --bin update_cr -- --file path\to\MagicCompRules.txt --db fresh_judge.db
```

**URL pattern:**
```
https://media.wizards.com/<year>/downloads/MagicCompRules%20<YYYYMMDD>.txt
```
Current rules URL listed at: **https://magic.wizards.com/en/rules**

Update `CR_URL` in `src-tauri/src/bin/update_cr.rs` when the URL changes.

### Magic Tournament Rules (MTR)

```powershell
# Download latest MTR PDF and import into fresh_judge.db
cargo run --bin update_mtr -- --db fresh_judge.db

# Import from a locally downloaded PDF
cargo run --bin update_mtr -- --file path\to\MTG_MTR.pdf --db fresh_judge.db

# Dump raw extracted PDF text for debugging (does not import)
cargo run --bin update_mtr -- --dump mtr_extracted.txt
```

**URL pattern:**
```
https://media.wizards.com/ContentResources/WPN/MTG_MTR_<YYYY>_<MonDD>_EN.pdf
```
Latest PDF listed at: **https://magic.wizards.com/en/resources/rules**

Update `MTR_URL` in `src-tauri/src/bin/update_mtr.rs` when the URL changes.

### Infraction Procedure Guide (IPG)

```powershell
# Download latest IPG PDF and import into fresh_judge.db
cargo run --bin update_ipg -- --db fresh_judge.db

# Import from a locally downloaded PDF
cargo run --bin update_ipg -- --file path\to\MTG_IPG.pdf --db fresh_judge.db
```

**URL pattern:**
```
https://media.wizards.com/ContentResources/WPN/MTG_IPG_<YYYY><MonDD>_EN.pdf
```
Update `IPG_URL` in `src-tauri/src/bin/update_ipg.rs` when the URL changes.

### Updating the live user database (dev machine only)

Omit `--db fresh_judge.db` to target the live app database instead:

```powershell
cargo run --bin update_cr
cargo run --bin update_mtr
cargo run --bin update_ipg
```

---

## Updating Card Data

Card data is imported from Scryfall's oracle-cards bulk JSON.

```powershell
# Import into fresh_judge.db (for bundling with installer)
cargo run --bin update_cards -- path\to\oracle-cards-YYYYMMDDhhmmss.json --db fresh_judge.db

# Import into the live user database
cargo run --bin update_cards -- path\to\oracle-cards-YYYYMMDDhhmmss.json
```

Download the latest oracle-cards bulk file from: **https://scryfall.com/docs/api/bulk-data**

This populates the `cards` and `card_rulings` tables used by the Card Search page. Card data updates are not included in the in-app update system (the bulk JSON is ~250 MB) — rebuild `fresh_judge.db` and release a new app version instead.

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

| Table          | Contents                                                         |
| -------------- | ---------------------------------------------------------------- |
| `documents`    | One row per imported document (CR, MTR, IPG) with version        |
| `rules`        | All rules and section headers with pre-rendered hyperlinked HTML |
| `rules_fts`    | FTS5 virtual table for full-text rule search                     |
| `glossary`     | CR glossary terms and definitions                                |
| `glossary_fts` | FTS5 virtual table for glossary search                           |
| `cards`        | Scryfall oracle card data (populated separately)                 |
| `cards_fts`    | FTS5 virtual table for card search                               |

Rules are stored with `body_html` pre-rendered at import time — cross-references like "see rule 704.5k" become `<a href="#R704.5k">` links. The frontend renders this HTML directly.
