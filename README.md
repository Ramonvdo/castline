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
- **Connectors (Make / n8n)** — paste a webhook URL and Castline POSTs a profile's fields to it, then
  builds/enriches the profile from the JSON your scenario returns. No tunnel, no open port (see below).

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
| `profiles.json` | `{{variable}}` value sets (+ global variable grouping) |
| `settings.json` | outbound connector URLs |

- **Windows:** `%APPDATA%\Castline\`
- **macOS:** `~/Library/Application Support/Castline/`

## Connectors → enrich & create profiles (Make / n8n)

Rather than run a server you'd have to expose to the internet, Castline calls **out** to a webhook URL
you paste from Make / n8n (or any HTTP endpoint). It POSTs a profile's fields and reads the JSON your
scenario returns — a single request/response round-trip on the connection Castline opens, so there's
**no tunnel and no open port**.

1. In Make add a **Custom webhook** trigger (n8n: a **Webhook** node) and copy its URL.
2. Add your lookup/enrichment steps, then a **Webhook response** module (n8n: **Respond to Webhook**)
   that returns JSON.
3. In Castline → **Connectors**, paste the URL. **Test** shows exactly which fields Castline sends
   (all your `{{variables}}`) so you can map them in Make/n8n, and shows what comes back.
4. Use it from **Profiles**: **Enrich** a profile (send its fields, merge the returned ones) or **New
   from connector** (send a seed like an email, build a profile from the response).

Response keys become variables of the same name — all mapping lives in your Make/n8n scenario, so
changing fields never means reconfiguring Castline. No integration? **Paste JSON…** in Profiles still
works.

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
    settings.rs        app settings + connector list
    connectors.rs      outbound POST (ureq) + JSON→profile passthrough
    lib.rs             Tauri commands + app setup
```

## Tech stack

Tauri v2 · Rust · Svelte 5 + Vite · plain JSON storage · `ureq` for outbound connectors.

## License

[MIT](LICENSE) © Castline contributors
