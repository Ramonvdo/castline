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
  Also: **Send all profiles** in one payload, send any library item to a webhook from its right-click
  menu, and **schedule** recurring sends (daily/weekly/monthly) in Settings.
- **Inbound HTTP endpoints** — flip it around: enable a token-gated local endpoint and paste its exact
  config into a Make **HTTP** module (or n8n **HTTP Request** node) to **Create** or **Update/enrich**
  a profile from the outside.
- **Castline AI enrich** — one OpenRouter call fills a profile's variables from what's already known
  (optionally with live web research). Guided by per-variable **descriptions** you write in Settings.
- **AI agent** — an embedded terminal running your own `claude` CLI in the data folder, with a generated
  `CLAUDE.md`, so it can research contacts and create/enrich profiles for you (via the local endpoint).
- **Date tokens** — `{{today}}` and `{{now}}` fill themselves at copy time, with Make-style formats:
  `{{today:YYYY-MM-DD}}`, `{{now:HH:mm}}`, `{{today:MMM D, YYYY}}`.
- **Usage counts** — every card shows how often you've copied it; sort any view by **Most used**.
- **Starts with Windows & lives in the tray** — autostart is on by default (toggle in Settings);
  closing the window keeps Castline running in the system tray, so schedules, the HTTP endpoint and
  the agent stay available. Reopen from the tray icon; **Quit** is in its right-click menu.

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
| `settings.json` | outbound connector URLs, HTTP-endpoint + AI-agent config |
| `CLAUDE.md` / `MEMORY.md` | agent context (generated) + the agent's durable notes |

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

**Outbound extras:** the Profiles header has **Send all ▾** (POSTs `{ "profiles": [ { name, values } ] }`
to a connector in one click), and any library item has a **plug button** (and right-click entry) to send
it to a connector. **Settings → Scheduled jobs** automates it on a daily/weekly/monthly cadence: all
profiles, one item, a whole folder — or a local **backup** of your data files. Schedules run while
Castline is open (the tray keeps it open); **missed runs are skipped** and the cadence re-anchors at
launch, unless a job opts into *Catch up*, which fires it exactly once.

## Castline AI enrich (OpenRouter)

Each profile's **Enrich ▾** menu offers three routes: **Castline AI** (structured, one call), any
**webhook connector** (your Make/n8n scenario), or **Ask the Agent** (open-ended, in the terminal).

For Castline AI, add an [OpenRouter](https://openrouter.ai/keys) API key in **Settings → AI workflow**
and pick a model. Choosing **Castline AI** opens a small dialog where you can add **extra context**
(notes from a call, a LinkedIn blurb), **attach a `.txt`/`.md` file**, and toggle **web research** for
that run — OpenRouter's `:online` mode works with *any* model, so no separate research model is needed.
The call sends the profile's current values plus the **variable descriptions** you write in
**Settings → Variables** — e.g. describing `{{companyName}}` as *"simplified lowercase company name:
'RocketFarm Studios LLC' → 'rocketfarm'"* makes every enrichment come back in exactly that shape. The
same descriptions are baked into the agent's `CLAUDE.md`.

## Inbound HTTP endpoints → push profiles in (Make / n8n HTTP module)

The reverse direction. In **Connectors → HTTP endpoint (inbound)**, flip it on to get a token and two
copy-paste-ready actions to drop into a Make **HTTP** module (n8n **HTTP Request** node):

| | Create profile | Update / enrich profile |
| --- | --- | --- |
| Method | `POST` | `POST` |
| URL | `http://127.0.0.1:8787/api/create-profile` | `http://127.0.0.1:8787/api/update-profile` |
| Headers | `Authorization: Bearer <token>` · `Content-Type: application/json` | same |
| Body | JSON — every key becomes a variable | JSON with a `name` **or** `email` to match, plus the fields to merge |

Create makes a brand-new profile; Update merges into the profile matched by `name` (case-insensitive) or
`email` (`404` if none match). The UI's **Test locally** button fires each action so you can see it work.

The endpoint binds `127.0.0.1` and is token-gated. A **self-hosted n8n** on the same machine/LAN reaches
it directly; **Make cloud** (or any internet scenario) can't see localhost — run a tunnel
(**ngrok** / **Cloudflare Tunnel**) and use that URL in place of `127.0.0.1:8787`.

## AI agent → research & enrich profiles

The **Agent** tab embeds your own **Claude Code** CLI (`claude`) in a real terminal, launched in
Castline's data folder. The app generates a `CLAUDE.md` there describing your `library.json` /
`profiles.json` and the local write endpoint, so the agent can read your data, research contacts with
whatever tools you've given it, and **create or enrich profiles** by POSTing to the endpoint above (Rust
stays the only writer, so the JSON never gets corrupted and the UI updates live). Durable notes it keeps
go in `MEMORY.md`, which Castline never overwrites.

Requires Claude Code installed (`npm install -g @anthropic-ai/claude-code`, or from **claude.ai/code**).
Starting the Agent turns the HTTP endpoint on automatically so the agent has a write path.

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
    settings.rs        app settings (connectors, HTTP endpoint, AI, schedules)
    connectors.rs      outbound POST (ureq) + JSON→profile passthrough
    receiver.rs        inbound HTTP endpoint (tiny_http): create / update profile
    llm.rs             Castline AI enrich (OpenRouter chat completions)
    scheduler.rs       recurring webhook sends (60s ticker)
    ai.rs              embedded claude PTY (portable-pty) + reader/emitter threads
    agent.rs           generates the agent's CLAUDE.md / MEMORY.md
    lib.rs             Tauri commands + app setup + store file-watcher
```

## Tech stack

Tauri v2 · Rust · Svelte 5 + Vite · plain JSON storage · `ureq` (outbound) + `tiny_http` (inbound) ·
`portable-pty` + `xterm.js` for the embedded agent.

## License

[MIT](LICENSE) © Castline contributors
