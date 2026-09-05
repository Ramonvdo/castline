# Authoring Castline blueprints

A blueprint is a shareable JSON file describing one or more library items. It is the **only
supported way to add items to the library**, because the HTTP endpoint handles profiles only.

## The format

Verified against `src-tauri/src/blueprint.rs`.

```json
{
  "castline_blueprint": 1,
  "folder": { "name": "Cold outreach", "icon": "", "color": "" },
  "items": [
    {
      "name": "Intro email, agency to SaaS founder",
      "kind": "template",
      "type": "email",
      "subject": "Quick question about {{company}}",
      "text": "Hi {{firstName}},\n\n...",
      "steps": [],
      "tags": ["outreach", "email"]
    }
  ]
}
```

### Rules that matter

- **`castline_blueprint` is required and must be `1`.** It is the only reliable way to tell a
  blueprint from any other JSON, because `folders` is optional on a library export and a bare `{}`
  would otherwise deserialise as an empty blueprint. Omit it and the import fails or misreads.
- **`folder` is optional.** Include it to create or target a named folder. Omit it and items land
  without folder presentation.
- **Never write `id`, `uses`, `favorite`, `created_at` or `updated_at`.** Blueprints deliberately
  carry only what is shareable. Ids are local bookkeeping and a foreign id would collide with the
  importer's library. Use counts are nobody else's business.
- **`variables` is informational only.** If you include it, know that Castline always recomputes it
  on import and never trusts the file's copy.
- `exported_at` and `app_version` are optional and informational.

### Per-kind shape

| Kind | Required | Leave empty |
|---|---|---|
| Template | `name`, `kind: "template"`, `text` | `subject`, `steps` |
| Email | `name`, `kind: "template"`, `type: "email"`, `subject`, `text` | `steps` |
| SOP | `name`, `kind: "sop"`, `steps: [{title, text}]` | `text`, `subject` |

An email item's `subject` is a separate field precisely so webhook payloads can map it
independently from the body. Putting the subject as the first line of `text` throws that away, so
always use the field.

## How to deliver one

1. Write the file with a `.castline.json` extension somewhere the user can reach it, such as their
   Downloads folder or the working directory. Never write it into Castline's data directory: that
   folder is the app's, and dropping stray JSON in it invites confusion with the real stores.
2. Tell the user the exact path and that importing is a drag of the file into the Castline window.
3. Say what it contains: how many items, which folder, and which `{{variables}}` they will need
   values for.

Import is the user's action, not yours. Do not attempt to simulate a drop or poke at the app.

## Writing good items

- **Match the library's existing vocabulary.** Read `library.json` first. Reuse the folder names,
  tag spellings and variable names already in use. A new `{{first_name}}` beside an existing
  `{{firstName}}` splits every profile.
- **Prefer variables over hardcoded specifics.** Any company name, first name, date or number that
  changes per send should be a `{{variable}}`. That is the entire point of the library.
- **Use the auto date tokens** rather than inventing a date variable: `{{today}}`, `{{now}}`, with
  Make-style formats like `{{today:MMM D, YYYY}}`. They resolve at copy time and never need a
  profile value.
- **Do not introduce a variable that duplicates a locked one.** Locked variables must be typed by
  hand every time. If a template needs personal text that must stay human, reuse the locked
  variable rather than adding an unlocked twin, which would defeat the lock.
- **Keep SOP steps genuinely stepwise.** A step is one action with its own copyable text, not a
  paragraph of narrative. If a step has no text worth copying, it is a heading, not a step.

## Writing copy that does not sound like a machine

When the request is email or marketing copy, the user's global instructions and Castline's own LLM
tone setting agree on this, so follow both:

- **No em dashes.** Use commas, colons or periods.
- No clichés, no corporate filler, no hedging that sounds like an assistant.
- Straight to the point, original phrasing.

For anything that will be read by a customer, load the `humanizer` skill and apply it before
writing the blueprint. It is installed and it is the user's stated standard for real copy.
