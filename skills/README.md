# Castline skills for Claude Code

Six skills that let Claude Code work with a Castline library from any terminal session, in any
folder. Castline's own Agent tab covers researching contacts and enriching profiles from inside the
app. These cover the rest: writing new library items, auditing what is already there, and keeping
variable naming from drifting.

## Install

Copy the whole set into your personal skills directory. The other five read shared schemas, path
resolution and rules from `castline/reference.md`, so installing one on its own will not work.

```bash
git clone https://github.com/Ramonvdo/castline.git
cp -r castline/skills/castline* ~/.claude/skills/
```

On Windows the target is `C:\Users\<you>\.claude\skills\`. To scope them to one project instead, use
that project's `.claude/skills/`. Restart Claude Code and the commands appear.

## The commands

| Command | What it does |
|---|---|
| `/castline "<request>"` | The umbrella. Reads the real library, then writes templates, emails and SOPs as importable blueprints, or creates and enriches profiles through the local endpoint. |
| `/castline-crm-sync` | Pulls contacts from a CRM into profiles: creates the ones that do not exist, enriches the ones that do. Interviews you about the CRM on the first run and remembers the answers. |
| `/castline-vars` | The canonical inventory of every `{{variable}}` in use, near-duplicate spellings that silently split profiles, and variables with no description. |
| `/castline-audit` | Library hygiene: duplicates, email items missing `type: "email"` and a separate subject, inconsistent tags, folder sprawl. |
| `/castline-sop` | Turns a described process into a multi-step SOP, with variables threaded through and delivered as a blueprint. |
| `/castline-create-skill` | Mines the library for clusters of related parameterised prompts worth turning into a skill of their own, then designs its onboarding. |

## What they will and will not do

They never hand-edit `library.json` or `profiles.json`. Castline's Rust core is the only writer,
which is what keeps ids consistent and the UI live. So there are exactly two write paths:

- **Library items** arrive as a `.castline.json` blueprint written somewhere you can reach. You drag
  it into the Castline window to import it.
- **Profiles** go through the local HTTP endpoint. That endpoint is off by default, so anything
  writing profiles will check `settings.json` first and tell you to turn it on rather than failing
  with a connection error. Toggle it in **Connectors → HTTP endpoint (inbound)**, or just open the
  **Agent** tab, which turns it on for you.

Locked variables are never written by anything, in any payload or blueprint. That is the whole point
of locking one.

Nothing here deletes a library item. There is no supported path for it, so `/castline-audit`
proposes and you delete in the app.

## Assumptions

- **Castline is installed.** The skills resolve the data directory rather than hardcoding it, since
  the Microsoft Store build redirects app data. If no `library.json` turns up they say so and stop.
- **Windows or macOS.** Path resolution covers both. Linux is not handled.
- **`/castline-create-skill` hands off to a skill-authoring skill.** It is written for
  `expert-skill`, which is a separate install and not part of this repo. Without it you still get
  the cluster and the onboarding design, which is the part that is hard to recover later.
- **Copy quality is somebody else's job.** If you have a prose-editing skill such as `humanizer`
  installed, the skills will run customer-facing copy through it. Otherwise they follow the tone in
  `settings.json.llm.tone`.
- **`/castline-crm-sync` stores a config, not a credential.** On the first run it writes
  `crm-config.local.json` beside itself, holding the CRM, the mapping and the *name* of the
  environment variable that holds your API key. The key itself stays in your environment. The
  `.local.json` suffix keeps the file out of git.

## Where the facts came from

`castline/reference.md` is the shared spine: data directory resolution, the `library.json` and
`profiles.json` schemas, variable parsing, the endpoint API, and the rules every skill obeys. The
other skills point at it rather than restating it, so there is one place to fix when Castline
changes.

`castline/sources.md` records which source files each fact was read from, and which were not read,
so a future update knows where to look first.
