---
name: castline-crm-sync
description: Sync contacts from a CRM into Castline profiles. Reads the existing Castline profiles and the library's {{variables}}, pulls contacts from the configured CRM, then creates profiles that do not exist yet and enriches those that do, through Castline's local HTTP endpoint. Remembers which CRM you use after the first run. Use when the user says /castline-crm-sync, or asks to sync, import or refresh clients, contacts or leads from a CRM into Castline.
argument-hint: [optional filter, e.g. "only contacts tagged client" or "dry run"]
---

# Castline CRM sync

Pull contacts out of a CRM and land them in Castline as profiles, with the library's
`{{variables}}` filled in.

Read `../castline/reference.md` first for the data directory, the schemas and the endpoint API.
Read [adapters.md](adapters.md) for the CRM adapter contract and the remembered config.

Scope filter, if given: **$ARGUMENTS**

## First run: the interview

On first run there is no `crm-config.local.json` beside this skill. Ask, then save, so later runs
skip straight to syncing.

1. **Which CRM?** Name it.
2. **How is it reached?** API base URL and the docs URL. Ask for the docs and read them. Never
   guess endpoint paths or field names for a CRM you have not read.
3. **How does auth work, and what is the env var?** The config stores the **name** of an
   environment variable, never the key itself. If the user has no env var set, tell them to set one
   and name it in the config.
4. **Which contacts are in scope?** All, or a segment, list, tag or pipeline stage. Syncing an
   entire CRM into a template library is rarely what someone wants.
5. **Field mapping.** This is the real work, so do it last and do it against reality:
   - Run the variable inventory (`/castline-vars`, or read the library yourself) to get the exact
     `{{variable}}` names in use.
   - Pull one sample contact from the CRM and show its actual fields.
   - Propose a mapping from CRM field to Castline variable, and confirm it.
   - **Exclude every variable in `profiles.json.locked` from the mapping entirely.**

Then write the config per [adapters.md](adapters.md) and say where it went.

## Later runs

Load `crm-config.local.json` and go. Re-interview only when the config is missing, the mapping
references a variable that no longer exists, or the user asks to reconfigure.

Say which CRM you loaded from config before you start. The user should never have to guess which
system you are about to touch.

## The sync

1. **Preflight the endpoint.** Read `settings.json`. If `http.enabled` is false, stop. The user can
   toggle **Connectors → HTTP endpoint (inbound)**, or just open the **Agent** tab, which turns the
   endpoint on automatically. Nothing can be written until it is listening, and attempting the POST
   produces a confusing connection error instead of a clear instruction. Re-read the setting before
   continuing.
2. **Read the current state.** `profiles.json` for existing profiles, `locked` and `descriptions`.
3. **Fetch contacts** through the adapter, honouring the scope filter.
4. **Match.** Castline matches on `name` case-insensitively or on a shared `email`. Prefer email;
   it is the stable key. Treat a name-only match as a candidate to confirm, not a certainty, since
   two people can share a name.
5. **Classify** each contact as create, enrich or skip. Show the counts and a sample before writing
   anything.
6. **Write.** `POST /api/create-profile` for new, `/api/update-profile` for existing. One request
   per contact. A `200 {"ok":true}` means it is already visible in the app.
7. **Report.** Created, enriched, skipped, and any that failed with the reason.

## Hard rules

1. **Never write a locked variable.** Strip every name in `profiles.json.locked` from every payload,
   even when the CRM has a value for it. Locked means the user types it by hand, and no enrich path
   may fill it. Filtering at the mapping stage is not enough; filter at the payload too.
2. **Never hand-edit `profiles.json`.** The endpoint is the only write path.
3. **Never store an API key** in the config, in a blueprint, or in the transcript. The config names
   an environment variable; the value stays in the environment.
4. **Never echo the Castline bearer token.**
5. **Dry run by default when the change is large.** More than about 25 writes, or any run that would
   modify existing profiles, gets a summary and a confirmation first. This is a local library, but
   a bad mapping silently overwrites good data across hundreds of profiles.
6. **Never invent a CRM field.** If the mapping needs a field the sample contact does not have, say
   so and ask, rather than guessing a plausible key name.
7. **Do not sync empty values over good ones.** An empty or missing CRM field should skip that
   variable, not blank an existing profile value.

## When the CRM has no usable API

If the CRM cannot be read programmatically, say so plainly and offer the CSV route: the user
exports contacts, and the skill reads that file with the same mapping and the same write path. That
is a real fallback, not a consolation prize, and it works with every CRM immediately.
