---
name: castline-audit
description: Audit a Castline library for hygiene problems. Finds duplicate and near-duplicate templates, email items missing type email and a separate subject, unused items, inconsistent tags and folder sprawl, then offers the fixes as an importable blueprint. Use when the user says /castline-audit, or asks to clean up, tidy, review or find problems in their Castline library.
---

# Castline library audit

Find what is wrong with the library, ranked by what actually costs the user something.

Read `../castline/reference.md` first for the data directory, the schemas and the hard rules.

Read-only on the library. Fixes are delivered as a blueprint the user imports, never as an edit.

## The checks, in priority order

### 1. Email items not typed as email

An item whose text opens with a subject line, or reads as an email, but which has no
`type: "email"` and no separate `subject` field.

This one matters more than it looks. The `subject` field exists so webhook payloads map subject and
body independently, which is what lets a Make or n8n scenario send the email directly. An email
with its subject buried in the first line of `text` cannot be sent that way, so the automation path
is quietly unavailable for that item.

Report each candidate with the line you believe is the subject.

### 2. Duplicate and near-duplicate text

Exact duplicate `text` across items, and near-duplicates. Compare on normalised text: whitespace
collapsed, variables replaced with a placeholder so `{{firstName}}` and `{{first_name}}` versions of
the same template match each other.

Rank duplicates by combined `uses`. When one copy has real usage and the other has none, the
recommendation is obvious. When both are used, they may be deliberate variants and are worth asking
about, not merging.

### 3. Variables with no description

Cross-reference every `{{variable}}` in the library against `profiles.json.descriptions`. Missing
descriptions degrade Castline's AI enrich, because the model has to guess what the field means.

Summarise the count here and hand the detail to `/castline-vars`, which owns this properly. Do not
duplicate that skill's full report.

### 4. Dead weight

**Check the distribution before running this check at all.** In most real libraries the majority of
items sit at `uses: 0`, because people paste from quick-find, add things in batches, or save a
prompt before they need it. If most of the library is at zero, `uses: 0` is the norm and listing it
is noise, not a finding. Say so and skip to the next check.

Only report dead weight when zero-use items are a genuine minority, and even then rank by age and
require a `created_at` well in the past. A template can be new, seasonal, or a reference note that
is read in the app rather than copied.

Never recommend deleting on the count alone. Present candidates with their age and folder and let
the user judge.

### 5. Tag inconsistency

Tags differing only by case, separator or plural: `email` beside `Email` beside `emails`. Same
failure as variable near-duplicates, cheaper to fix. Recommend the most-used spelling.

### 6. Folder sprawl

Folders holding one or two items, and folders whose names overlap. Report them; do not propose a
reorganisation unless asked. Folder structure is personal and the user's ordering usually encodes
something you cannot see from the JSON.

## Output

Lead with a one-line health summary: item count, folder count, and the number of findings by
severity. Then the findings in the order above, each with the specific items named.

Offer, but do not write unprompted, a `.castline.json` blueprint containing the corrected versions
of the items you propose changing.

## Delivering fixes

Fixes are new items in a blueprint the user imports. Note the consequence honestly: importing a
corrected copy does not remove the original, so the user deletes the old item in the app. There is
no supported way for a skill to delete a library item, and hand-editing `library.json` to do it is
exactly what the hard rules forbid.

For anything that is only a rename or a retag, it is usually less work for the user to fix it in
the app directly. Say so rather than generating a blueprint that creates duplicates.

## Hard rules

1. **Read-only on `library.json`.** No edits, ever. Blueprint or nothing.
2. **Never delete anything**, and never imply the skill can.
3. **Never recommend deleting on `uses: 0` alone.** Present, do not prune.
4. **Never merge items that both have real usage** without asking. They are probably deliberate
   variants.
5. **Do not restate `/castline-vars`.** Summarise the variable finding and hand off.
