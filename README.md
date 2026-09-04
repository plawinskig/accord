# Accord

Accord is a local, private knowledge base and markdown note-taking application inspired by chat interfaces. Built as a high-performance desktop client with Tauri, SvelteKit, and SQLite, it ensures all your data, configuration, and attachments are stored securely on your local machine.

## Features

* **Nested Channels:** Organize notes in a recursive hierarchy of folders and subfolders.
* **Markdown Chat Interface:** Chat-like note creation with full Markdown support and DOMpurify XSS protection.
* **Auto-detected Tags:** Automatic extraction and normalization of tags (e.g., `::tag`) directly from note content.


* **Global Tag Filtering:** A dedicated sidebar for global tag management and real-time cross-filtering.
* **Advanced Search:** Lightning-fast full-text search (FTS5) across the workspace with context snippets.
* **Robust Attachments:** Support for Drag & Drop and native clipboard pasting (Ctrl+V), including a native OS bypass for reliable image parsing on Linux.


* **Smart File Handling:** Attach files in three modes (`COPY`, `MOVE`, `LINK`) rendered securely via a custom `accord://` protocol.


* **Safe Trash System:** Soft-delete functionality with restore capabilities and a permanent hard-delete that cascades through the database and physical drive.


* **Markdown Export:** Export entire folders to `.md` files with correctly mapped attachment links.

## Architecture

* **Frontend:** Svelte 5 (Runes) + TypeScript + Tailwind CSS v4 + Vite
* **Desktop Shell:** Tauri 2
* **Backend:** Rust + SQLx
* **Database:** SQLite running in Write-Ahead Logging (WAL) mode with foreign key enforcement for data integrity.



## Requirements

Ensure the following prerequisites are met before running the application:

* Node.js 18+ or newer
* npm
* Rust stable
* Tauri prerequisites for your operating system

*Note for Linux users: An additional set of GUI and system libraries (such as `webkit2gtk` and GTK dependencies) may be required.*

## Installation

1. Clone the repository:

```bash
git clone https://github.com/plawinskig/accord.git
cd accord

```

2. Install JavaScript dependencies:

```bash
npm install

```

3. Start the application in development mode:

```bash
npm run tauri dev

```

## Scripts

**From the project root (Frontend & Tauri):**

```bash
npm run dev         # Start frontend-only dev server
npm run build       # Build frontend for production
npm run preview     # Preview production build
npm run check       # Run Svelte sync and typechecking
npm run tauri dev   # Start full desktop app in dev mode
npm run tauri build # Compile the final executable/bundle

```

**From the `src-tauri` directory (Rust Backend):**

```bash
cargo check
cargo clippy -- -D warnings
cargo fmt
cargo sqlx database setup # Create database and run migrations
cargo sqlx prepare        # Freeze SQLx queries for offline compile-time verification

```

## Project Structure

```text
accord/
├── src/                     # SvelteKit frontend
│   ├── lib/                 # App components and UI state (Runes)
│   └── routes/              # Application views (+page.svelte)
├── src-tauri/               # Tauri backend and Rust logic
│   ├── src/                 # Rust modules: notes, folders, tags, attachments, protocol, trash
│   ├── migrations/          # SQLite schema migrations
│   ├── Cargo.toml           # Rust configuration
│   └── tauri.conf.json      # Tauri application configuration
├── static/                  # Static frontend assets (e.g., logos)
├── package.json             # npm scripts and frontend dependencies
├── svelte.config.js
├── vite.config.js
└── README.md

```

## Core Concepts

### Workspace

On the first launch, the user selects a working directory. Accord automatically provisions this folder with the SQLite database file and a hidden `attachments/` directory. Because the backend is entirely local, data is never dependent on cloud services.

### Notes and Tags

Tags are automatically detected from trigger patterns within the text (e.g., `::work`, `::important`). The backend normalizes these tags, stores them uniquely, and binds them to the note via a many-to-many relationship. Users can filter notes dynamically using the right-side panel.

### Attachments

Files dropped into the chat or selected via the picker can be handled in three ways:

* **COPY** — Duplicates the original file into the workspace's attachments folder.
* **MOVE** — Relocates the original file directly into the workspace.
* **LINK** — Retains the file in its original location, saving only an absolute reference in the database.

### Trash System

Instead of immediate deletion, deleted folders and notes are flagged as `is_deleted = 1`. From the Trash panel, users can restore items or permanently empty the trash. Hard deletion utilizes recursive Common Table Expressions (CTEs) and `ON DELETE CASCADE` constraints to safely wipe orphaned data and physical files.
