<div align="center">

# Castline

**A lightweight, local-first library for prompts, templates, notes & SOPs — with live `{{variables}}` and reusable profiles.**

Cast any block of text once, fill it in a second, paste it anywhere.

[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS-1f6feb)](https://github.com/Ramonvdo/castline/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202%20%2B%20Rust-1f6feb)](https://tauri.app)
[![License: MIT](https://img.shields.io/badge/license-MIT-1f6feb)](LICENSE)
[![Release](https://img.shields.io/github/v/release/Ramonvdo/castline?display_name=tag&color=1f6feb)](https://github.com/Ramonvdo/castline/releases/latest)

</div>

---

## What it is

Castline is a fast desktop shelf for **any reusable text** — AI prompts, `CLAUDE.md`-style notes,
message and email templates, and multi-step SOPs. Everything is copy-pasteable text at the end of the
day, so there's no artificial split between "prompts" and "templates": one unified item, with folders,
tags, colours and icons to keep a big library tidy.

It's **local-first** (plain JSON files on your machine — no account, no cloud, no telemetry) and
**tiny** (a Tauri + Rust binary, single-digit-MB installer).

## Features

- **`{{variables}}` + Fill & copy** — write `Hi {{firstName}}, …` once; fill the blanks and copy in one step.
- **Profiles** — save named sets of variable values (a client, a project, yourself) and auto-load them
  when filling. Stored in a **separate file** from your library, so prompts and people back up independently.
- **SOPs (multi-step)** — chain prompts into an ordered runbook and copy them step-by-step.
- **Pick & combine** — `Ctrl`-click any cards to select them in order (each shows its number), then
  **Copy combined**, spin them into a **New SOP**, or **export to `.md`** — perfect for assembling a
  custom SOP to hand a client.
- **Quick find (`Ctrl`+`K`)** — fuzzy-search every item across all folders and copy instantly.
- **Beat the mess** — global search, cross-folder tag filters, folder icons/colours, an "All items"
  view and Favourites keep large libraries scannable.
- **Import / export** — your library and profiles are portable JSON. Back them up, share them, open the
  folder, or edit by hand.
- **Incoming webhooks → auto profiles** — turn a form submission (e.g. a Calendly booking) into a
  ready-to-use profile automatically (see below).

## Install

### Prebuilt (recommended)
Download the latest installer from the [**Releases**](https://github.com/Ramonvdo/castline/releases/latest) page:

- **Windows** — `Castline_x.y.z_x64-setup.exe` (NSIS, installs for the current user)
- **macOS** — `Castline_x.y.z_universal.dmg`

### Build from source
Requires [Rust](https://www.rust-lang.org/tools/install), [Node 18+](https://nodejs.org), and the Tauri CLI.

```bash
git clone https://github.com/Ramonvdo/castline.git
cd castline
npm install
npm run tauri dev      # run in development
npm run tauri build    # produce an installer in src-tauri/target/release/bundle
```

## Where your data lives

Two portable JSON files (plus settings) in your OS app-data folder — reachable from
**Settings → Data & backups → Open folder**:

| File | Contents |
| --- | --- |
| `library.json` | folders, prompts, templates, notes, SOPs |
| `profiles.json` | `{{variable}}` value sets (incl. webhook-created ones) |
| `settings.json` | accent colour + webhook configuration |

- **Windows:** `%APPDATA%\Castline\`
- **macOS:** `~/Library/Application Support/Castline/`

## Incoming webhooks → auto-create profiles

Castline can run a small **local HTTP receiver** that turns an inbound JSON payload into a profile,
using a field mapping you define in **Settings → Incoming webhook**.

Because a desktop app on `localhost` isn't reachable from the public internet, point one of these at
the endpoint:

- a tunnel — **ngrok** or **Cloudflare Tunnel**
- a relay/automation — **Make**, **n8n** or **Zapier** forwarding the payload

```
POST http://127.0.0.1:8787/hook?token=<your-secret>
Content-Type: application/json

{ "first_name": "Sam", "last_name": "Rivera", "email": "sam@example.com" }
```

→ creates a profile **"Sam Rivera"** with `firstName`, `lastName`, `email` ready to fill your templates.

The receiver binds to `127.0.0.1` only, requires the secret `token`, and runs only while the app is
open. No tunnel? The same field mapping powers **Paste JSON…** in the Profiles panel.

## Cutting a release

Version lives in three files — keep them in sync, then tag:

1. `package.json` → `version`
2. `src-tauri/Cargo.toml` → `version`
3. `src-tauri/tauri.conf.json` → `version`

```bash
git commit -am "release: v1.0.1"
git tag v1.0.1
git push origin v1.0.1
```

The tag triggers `.github/workflows/release.yml`, which builds Windows + universal-macOS installers and
attaches them to a **draft** GitHub Release for you to review and publish.

## Project structure

```
castline/
  src/                 Svelte 5 frontend (Library, QuickOpen, FillCopy, Profiles, Settings, Icon)
  src-tauri/src/
    library.rs         folders + items store (library.json)
    profiles.rs        variable profiles store (profiles.json)
    settings.rs        app settings + webhook config
    webhook.rs         local HTTP receiver + JSON→profile mapping
    lib.rs             Tauri commands + app setup
```

## Tech stack

Tauri v2 · Rust · Svelte 5 + Vite · plain JSON storage · `tiny_http` for the webhook receiver.

## License

[MIT](LICENSE) © Castline contributors
