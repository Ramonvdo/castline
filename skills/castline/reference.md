# Castline reference

The shared factual spine for every `castline-*` skill. Everything here was read from the source of
`Ramonvdo/castline` and from a live install. Do not restate it in the other skills; point here.

Castline is a local-first Tauri desktop app: a library of reusable text (prompts, email templates,
notes, multi-step SOPs) with `{{variables}}`, plus reusable **profiles** (named sets of variable
values). No account, no cloud, plain JSON on disk.

## 1. Finding the data directory

**Resolve it, never hardcode.** The Microsoft Store build redirects app data, so the obvious path is
empty and a skill that assumes it silently finds nothing.

Check in this order and use the first that contains `library.json`:

```
1. MSIX (Store build), Windows:
   %LOCALAPPDATA%\Packages\Ravando.Castline_<pubhash>\LocalCache\Roaming\Castline\
   The publisher hash is stable per publisher. On this machine:
   Ravando.Castline_36ex7sfbaqfcj

2. Plain install, Windows:
   %APPDATA%\Castline\

3. macOS:
   ~/Library/Application Support/Castline/
```

A reliable Windows resolver is a glob for `Packages/Ravando.Castline_*/LocalCache/Roaming/Castline`
falling back to `%APPDATA%/Castline`. If neither has `library.json`, stop and say Castline does not
appear to be installed. Do not create the directory.

Three files live there:

| File | Contents |
|---|---|
| `library.json` | the text library |
| `profiles.json` | profiles, layout, variable descriptions, locked variables |
| `settings.json` | theme, connectors, schedules, the HTTP endpoint config, LLM settings |

## 2. Schemas

### `library.json`

```json
{ "folders": [ { "id": "", "name": "", "color": "", "icon": "",
                 "items": [ { "id": "", "name": "", "kind": "template|sop",
                              "type": "email or absent", "text": "", "subject": "",
                              "steps": [], "tags": [], "favorite": false,
                              "uses": 0, "created_at": "", "updated_at": "" } ] } ] }
```

- `kind` is `template` (single body) or `sop` (ordered `steps[]` of `{title, text}`).
- `type: "email"` gives the item a separate `subject` line that webhook payloads map independently
  from the body. Anything else leaves `type` absent.
- `uses` counts copies. It is the honest signal of which templates actually earn their place.

### `profiles.json`

```json
{ "profiles": [ { "id": "", "name": "", "values": { "variableName": "value" },
                  "source": "manual|webhook|import", "tone": "", "locked": [] } ],
  "layout": [ { "type": "splitter|var", "label": "", "name": "" } ],
  "descriptions": { "variableName": "what this value must contain" },
  "locked": [ "variableName" ] }
```

- `values` keys are the `{{variable}}` names used across the library.
- `layout` is **presentation only**. It never affects values. Ignore it when writing.
- `descriptions` is per-variable guidance that drives the AI enrich workflow and the agent's
  generated `CLAUDE.md`. A variable with no description enriches badly, so missing descriptions are
  a correctness problem, not tidiness.
- `locked` is the **global** locked-empty list. `Profile.locked` is legacy and migrated into it.
- `source` records provenance. A profile written through the inbound endpoint is `webhook`.

### `settings.json`

```json
{ "http": { "enabled": false, "port": 8787, "token": "..." }, "llm": { "tone": "..." }, ... }
```

## 3. Variables

The parser is `VAR_RE = /\{\{\s*([^{}]+?)\s*\}\}/g`. Inner whitespace is allowed and trimmed, so
`{{ firstName }}` and `{{firstName}}` are the same variable.

**Auto date tokens resolve at copy time and are never fill fields or profile variables.** Exclude
them from any variable inventory. The head is the part before `:`, case-insensitive:

- `{{today}}` defaults to `YYYY-MM-DD`
- `{{now}}` defaults to `HH:mm`
- Format tokens: `YYYY YY MMMM MMM MM M DD D HH mm ss`, for example `{{today:MMM D, YYYY}}`

When filling, a variable with no value is left as its literal `{{name}}` rather than blanked.

## 4. Writing to Castline: two paths, and neither is editing JSON

Castline's own agent guide states the rule: Rust is the only writer, so the JSON never gets
hand-corrupted, ids stay consistent, and the UI refreshes live. Hand-editing `library.json` or
`profiles.json` while the app runs risks the app overwriting your change from memory.

### Path A: profiles, via the local HTTP endpoint

Loopback only, bearer token, from `settings.json.http`.

```
POST http://127.0.0.1:<port>/api/create-profile
POST http://127.0.0.1:<port>/api/update-profile
Authorization: Bearer <token>
Content-Type: application/json
```

- **Create**: every JSON key becomes a variable of the same name. `name`, or `first_name` plus
  `last_name`, sets the display name.
- **Update**: merges into the profile matched by `name` case-insensitively, or by a shared `email`.
  404 if nothing matches.
- `200 {"ok":true,...}` means it worked and the profile is already visible in the app.

**Preflight.** If `settings.json.http.enabled` is false, the endpoint is not listening. Do not
attempt the POST and report a confusing connection error. Give the user both ways to turn it on:

- **Connectors → HTTP endpoint (inbound)**, the explicit toggle.
- **Opening the Agent tab**, which turns the endpoint on automatically so the embedded agent has a
  write path. Often the faster answer if they were heading there anyway.

Re-read `settings.json` after they say it is on, rather than assuming.

**Never print the token.** Read it, use it in the request, and keep it out of transcripts, files
and commits. Redact by key path, not by top-level key name.

### Path B: library items, via a blueprint

The endpoint cannot create library items. To add a template, email or SOP, author a
`.castline.json` blueprint and have the user drag it into the window. Format and rules are in
[blueprints.md](blueprints.md).

## 5. Hard rules

1. **Never hand-edit `library.json` or `profiles.json`.** Endpoint for profiles, blueprint for
   library items.
2. **Never write a locked variable.** Anything in `profiles.json.locked` must stay empty and be
   typed by the user on the spot. No enrich path (AI, webhook, inbound API, profile save) may fill
   it. This is a deliberate privacy boundary. Exclude locked names from every mapping and payload.
3. **Respect safe mode.** Nothing with unfilled `{{variables}}` may be sent to an external webhook.
   Fill first or do not send.
4. **Never echo the bearer token.**
5. **Never invent a field, endpoint or item kind.** The two endpoints, the blueprint keys and the
   schemas above are the entire surface.
6. **Read before you write.** Load the library first so new items match existing folder names, tag
   vocabulary and variable spellings. A near-duplicate variable like `firstname` beside `firstName`
   is a defect.

## 6. Relationship to Castline's built-in agent

Castline writes its own `CLAUDE.md` into the data directory for the agent embedded in the app
(Settings → AI agent). That file is regenerated on every agent start, covers the schemas and the
profile-writing endpoint, and is scoped to gathering data and enriching profiles.

These skills are the complement: they work in any Claude Code session anywhere on the machine, and
they cover what the embedded agent's guide does not, namely authoring library items, auditing the
library, variable hygiene, and mining the library for skills.

**Never edit Castline's generated `CLAUDE.md`.** It is overwritten on every agent start. Durable
notes belong in `MEMORY.md` in the same folder, which Castline writes once and never touches again.
