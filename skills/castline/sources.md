# Sources

Bibliography for the whole `castline-*` suite: `castline`, `castline-crm-sync`,
`castline-create-skill`, `castline-audit`, `castline-sop`, `castline-vars`. One file for six skills,
because six near-identical bibliographies would be noise.

Everything was read from Castline's source at the state of the default branch on 2026-09-05
(last pushed 2026-08-17) and from a live Microsoft Store install.

## Included

### `src-tauri/src/agent.rs`
Read 2026-09-05. The single most useful file.

Castline writes its own `CLAUDE.md` into the data directory for the agent embedded in the app. That
file contributed the schemas for `library.json` and `profiles.json`, the curl shape for the profile
endpoint, and, most importantly, **the rule these skills are built around**: create and enrich
profiles by POSTing to the local endpoint, do not hand-edit the JSON, because Rust stays the only
writer, ids stay consistent, and the UI refreshes live only for changes made through the endpoint.

It also defined the boundary. That guide is scoped to gathering data and enriching profiles, and it
only exists inside Castline's own terminal. The suite deliberately covers what it does not: authoring
library items, auditing, variable hygiene, and mining the library for skills.

### `src-tauri/src/receiver.rs`
Read 2026-09-05. Contributed the entire profile write path: `POST /api/create-profile` and
`/api/update-profile` on `127.0.0.1:<port>`, bearer auth via header or `?token=`, update matching by
`name` case-insensitively or by `email`, loopback-only binding. Also the preflight requirement,
since the controller reports a bind failure rather than silently listening.

### `src-tauri/src/blueprint.rs`
Read 2026-09-05. Contributed the `.castline.json` format and its rules: `castline_blueprint` is
required and is the only reliable discriminator, because `folders` is optional on a library export
so a bare `{}` would otherwise deserialise as an empty blueprint. Also the deliberate exclusions,
since ids, use counts, pins and timestamps are personal bookkeeping and a foreign id would collide
with the importer's library, and the fact that a blueprint's `variables` list is always recomputed
on import and never trusted.

### `src-tauri/src/profiles.rs`
Read 2026-09-05. Contributed the exact `ProfilesData` shape: `profiles`, `layout`, `descriptions`
as a per-variable map, and `locked` as the **global** locked-empty list, with `Profile.locked`
noted as legacy and migrated on load. The comment on `locked` is the basis for the hard rule
repeated in every skill: locked variables must be filled on the spot and are never written by any
enrich path (AI, webhook, inbound API) or profile save.

It also contributed `layout` being presentation-only, which is why the skills ignore it when
writing, and `source` being `manual`, `webhook` or `import`.

### `src/lib/vars.js`
Read 2026-09-05. Contributed the exact variable semantics: the parser
`/\{\{\s*([^{}]+?)\s*\}\}/g` with trimmed inner whitespace, and the auto date tokens `{{today}}` and
`{{now}}` with Make-style formats `YYYY YY MMMM MMM MM M DD D HH mm ss`, defaulting to `YYYY-MM-DD`
and `HH:mm`. Crucially, auto tokens are resolved at copy time and are never fill fields or profile
variables, which is why `/castline-vars` excludes them from the inventory. Also that an unfilled
variable is left as its literal `{{name}}` rather than blanked.

### `README.md`
Read 2026-09-05. Contributed the feature model and two constraints that became hard rules:
**safe mode**, which blocks anything with unfilled `{{variables}}` from being sent to an external
webhook, and **locked variables**, described there as perfect for personal notes that must stay
human. Also the email item type having a separate subject that webhook payloads map independently,
which is why `/castline-audit` treats an untyped email as a real defect rather than cosmetics.

### A live install
Inspected 2026-09-05, structure only.

Contributed the **MSIX path redirection**, which is the fact most likely to break a naive skill:
`%APPDATA%\Castline` is empty on a Store install and the real store is under
`%LOCALAPPDATA%\Packages\Ravando.Castline_36ex7sfbaqfcj\LocalCache\Roaming\Castline\`.

Also two defaults the skills are designed around. The inbound HTTP endpoint ships disabled, which is
why every profile write starts with a preflight instead of a POST that fails confusingly. And `type`
is opt-in per item, so most items in a real library never get it, which is why the untyped-email
check leads the audit rather than sitting at the bottom as a footnote.

## Not read, and therefore not relied on

`library.rs`, `connectors.rs`, `scheduler.rs`, `llm.rs`, `storage.rs`, `settings.rs`, `startup.rs`,
and the Svelte components other than `vars.js`. The library item schema in `reference.md` comes from
`agent.rs` plus the live `library.json`, not from `library.rs` directly. If a future change makes an
item field behave differently, that file is the place to check first.

`connectors.rs` in particular is unread, so the suite says nothing about outbound webhooks and
scheduled sends beyond what the README states. Anything the skills need to do with connectors should
start by reading it.

## Deliberately out of scope

- **Castline's generated `CLAUDE.md` is never edited by these skills.** It is overwritten on every
  agent start. Durable notes belong in `MEMORY.md` beside it, which Castline writes once and never
  touches again.
- **Deleting library items.** There is no supported path, and hand-editing `library.json` to do it
  is what the hard rules forbid. `/castline-audit` proposes and the user deletes in the app.
- **Skill authoring.** `/castline-create-skill` finds and ranks candidates, then hands to
  `/expert-skill`, which researches real sources and stops for approval before writing.
- **Customer-facing copy standards.** Out of scope. The suite defers to whatever prose-editing skill
  is installed, such as `humanizer`, and to the tone configured in `settings.json.llm.tone`.
