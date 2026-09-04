# Accord

Accord is a local application for creating and organizing markdown notes with support for folders, tags, attachments, and search. It runs as a desktop client built with Tauri, SvelteKit, and SQLite, so all data is stored locally on the user's machine.

## Features

- Organize notes in a hierarchy of folders and subfolders
- Markdown-based notes with content preview
- Automatic detection of tags like ::tag inside note content
- Global tag list with tag-based filtering
- Full-text search across the workspace
- Trash with restore and permanent delete actions
- Attachments for notes: copy, move, and link support
- Export a folder to a Markdown file
- Local SQLite storage in a dedicated workspace directory

## Architecture

- Frontend: SvelteKit + TypeScript + Vite
- Desktop shell: Tauri 2
- Backend: Rust + SQLx + SQLite
- Workspace configuration: saved in the system config directory

## Requirements

Before running the app, make sure you have installed:

- Node.js 18+ or newer
- npm
- Rust stable
- Tauri prerequisites for your operating system

On Linux, an additional set of GUI and system libraries may be required.

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

3. Start the app in development mode:

```bash
npm run tauri dev
```

## Scripts

From the project root:

```bash
npm run dev
npm run build
npm run preview
npm run check
npm run tauri dev
```

From the `src-tauri` directory:

```bash
cargo check
cargo clippy -- -D warnings
cargo fmt
cargo sqlx database create
cargo sqlx migrate run
```

## Project structure

```text
accord/
├── src/                     # SvelteKit frontend
│   ├── lib/                 # app components and UI state
│   └── routes/              # application views
├── src-tauri/               # Tauri backend and Rust logic
│   ├── src/                 # application modules: notes, folders, tags, search, trash
│   ├── migrations/          # SQLite migrations
│   ├── capabilities/        # Tauri permissions
│   ├── Cargo.toml           # Rust configuration
│   └── tauri.conf.json      # Tauri configuration
├── static/                  # static frontend assets
├── package.json             # npm scripts and frontend dependencies
├── svelte.config.js
├── vite.config.js
├── tsconfig.json
├── README.md
└── .gitignore
```

## How it works

On first launch, the user selects a working folder where Accord stores:

- the SQLite database with notes,
- the attachments directory,
- the workspace configuration.

Then the user can create channels (folders), add notes, attach tags, and import files. The app uses a local backend, so data is not dependent on a cloud service or external server.

## Notes and tags

Tags are automatically detected from patterns such as:

```text
::work
::important
::ideas
```

Tags are normalized and stored in the database, and can then be filtered from the right-side panel.

## Attachments

Files can be attached to notes in three modes:

- `COPY` — copy the file into the workspace directory
- `MOVE` — move the file into the workspace directory
- `LINK` — keep a link to the original file

## Search

The search feature uses SQLite FTS (Full Text Search), allowing fast retrieval of note content by phrase or keyword. Results show a snippet of the text along with the folder name and creation date.

## Trash

Instead of deleting items immediately, they are moved to the trash. From there, users can:

- restore a folder or note,
- permanently empty the trash,
- remove associated attachment files physically.

## Export

It is possible to export an entire folder to a Markdown file. The file includes the folder header, note content, and a list of attachments as links.

## Development

To start working on the project:

```bash
npm install
npm run tauri dev
```

Useful tools for further development:

- VS Code
- Svelte and Tauri extensions
- Rust Analyzer

## License

This project is distributed under the MIT license.

## Author

Accord is a personal / experimental project developed locally in a desktop environment.

## Status

The project is actively being developed and currently includes core functionality for managing notes, tags, folders, and workspace data.
