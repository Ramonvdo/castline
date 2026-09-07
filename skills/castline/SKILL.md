---
name: castline
description: Work with Castline, the local-first desktop library of reusable text (AI prompts, email templates, notes, multi-step SOPs) with {{variables}} and profiles. Reads the real library, writes new templates, emails and SOPs as importable blueprints, and creates or enriches profiles through Castline's local HTTP endpoint. Use when the user says /castline, or asks to add a template, prompt, email or SOP to Castline, look something up in their Castline library, or work with Castline profiles and variables.
argument-hint: [request, e.g. "make some cold email copy and add it to Castline"]
---

# Castline

Castline is a local-first library of reusable text with `{{variables}}` and reusable profiles. This
skill gives you the real context and the two correct write paths.

Read [reference.md](reference.md) first, every time. It carries the data-directory resolution, the
schemas, the endpoint API and the hard rules. Read [blueprints.md](blueprints.md) before authoring
any library item. Provenance is in [sources.md](sources.md).

The request is: **$ARGUMENTS**

If `$ARGUMENTS` is empty, ask what the user wants to do with Castline, then continue.

## Always start here

1. **Resolve the data directory.** Never hardcode it. The Microsoft Store build redirects app data,
   so `%APPDATA%\Castline` is empty on a Store install and the real store sits under
   `%LOCALAPPDATA%\Packages\Ravando.Castline_*\LocalCache\Roaming\Castline\`. Resolution order is in
   [reference.md](reference.md). If no `library.json` is found anywhere, say Castline does not
   appear to be installed and stop.
2. **Read `library.json` before writing anything.** You need the existing folder names, tag
   vocabulary and variable spellings so new work matches instead of forking them.
3. **Read `profiles.json` for `locked` and `descriptions`** whenever profiles or variables are in
   play.

## Routing

| The request is about | Do this |
|---|---|
| Adding a template, prompt, email or SOP | Author a blueprint, per [blueprints.md](blueprints.md). The endpoint cannot create library items. |
| Creating or enriching a profile | POST to the local endpoint. Preflight `settings.json.http.enabled` first. |
| Syncing profiles from a CRM | Hand off to `/castline-crm-sync`. |
| Building a multi-step SOP | Hand off to `/castline-sop`. |
| Library cleanliness, duplicates, missing types | Hand off to `/castline-audit`. |
| What variables exist, what lacks descriptions | Hand off to `/castline-vars`. |
| Turning library content into a Claude skill | Hand off to `/castline-create-skill`. |
| Finding or reading something in the library | Read `library.json` directly and answer. |

## Adding an item

The common case, for example "make some cold email copy and add it to Castline":

1. Read the library. Find the folder this belongs in, or propose a new one. Note the tag vocabulary
   and the variable names already in use.
2. Decide the kind. Email copy is `kind: "template"` with `type: "email"` and a **separate
   `subject`**, because webhook payloads map subject and body independently. A process is
   `kind: "sop"` with ordered `steps[]`.
3. Write the copy. Read `settings.json.llm.tone` and follow the tone set there. Castline ships with
   a starter tone that rules out em dashes, so prefer commas, colons and periods unless the
   configured tone says otherwise.
4. Variabilise it. Anything that changes per send becomes a `{{variable}}`, reusing existing names
   exactly. Use `{{today}}` and `{{now}}` rather than inventing date variables.
5. Write the `.castline.json` blueprint somewhere the user can reach, never inside Castline's data
   directory.
6. Tell them the path, that importing is a drag into the Castline window, and which variables they
   will need values for.

## Creating or enriching a profile

1. **Preflight.** Read `settings.json`. If `http.enabled` is false, stop. Tell the user they can
   either toggle **Connectors → HTTP endpoint (inbound)**, or simply open the **Agent** tab, which
   turns the endpoint on automatically. Re-read the setting before continuing; do not assume.
2. Read `profiles.json.locked`. **Strip every locked variable from the payload.** They must be
   typed by hand and no enrich path may write them.
3. POST to `/api/create-profile` for a new profile, or `/api/update-profile` to merge into one
   matched by `name` case-insensitively or by `email`.
4. Keep the bearer token out of the transcript, out of any file you write, and out of any command
   you echo back.
5. A `200 {"ok":true}` means it is already live in the app. Say so rather than telling the user to
   refresh.

## Hard rules

1. **Never hand-edit `library.json` or `profiles.json`.** Castline's Rust core is the only writer;
   that is what keeps ids consistent, the JSON uncorrupted and the UI live. Blueprint for library
   items, endpoint for profiles.
2. **Never write a locked variable**, in any payload, mapping or blueprint.
3. **Respect safe mode.** Nothing with unfilled `{{variables}}` goes to an external webhook.
4. **Never echo the bearer token.** Redact by key path, not by top-level key name.
5. **Never invent a field, endpoint or item kind.** The surface is exactly what
   [reference.md](reference.md) documents.
6. **Never write into Castline's data directory.** Blueprints go somewhere the user can reach.
7. **Never edit Castline's generated `CLAUDE.md`.** It is overwritten on every agent start. Durable
   notes go in `MEMORY.md` beside it.

## Boundaries

- **Castline ships its own agent** (Settings → AI agent) with a generated `CLAUDE.md` scoped to
  profile enrichment inside the app's own terminal. This skill is the complement: it works in any
  session, anywhere, and covers authoring, auditing and variable hygiene that the built-in guide
  does not.
- **Copy quality is not this skill's job.** If a prose-editing skill such as `humanizer` is
  installed, run customer-facing copy through it before the blueprint gets written.
- **This skill does not send anything.** Castline sends, via its connectors and schedules. You
  author and you populate.
