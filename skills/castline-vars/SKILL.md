---
name: castline-vars
description: Audit the {{variables}} across a Castline library. Produces the canonical inventory of every variable in use, flags near-duplicate spellings that silently split profiles, finds variables with no description (which degrades Castline's AI enrich), lists locked variables, and reports templates referencing variables no profile fills. Use when the user says /castline-vars, or asks what variables their Castline templates use, why AI enrich is filling things badly, or wants to clean up variable naming.
---

# Castline variable hygiene

The canonical variable inventory, and the defects that make filling and enriching go wrong.

Read `../castline/reference.md` first for the data directory, the schemas and the variable syntax.

Read-only. This skill reports and proposes; it never writes.

## Build the inventory

1. Resolve the data directory and read `library.json` and `profiles.json`.
2. Extract variables with the real parser semantics:
   `/\{\{\s*([^{}]+?)\s*\}\}/g`, trimming inner whitespace, so `{{ firstName }}` and `{{firstName}}`
   are one variable.
3. **Exclude auto date tokens.** Anything whose head before `:` is `today` or `now`, case
   insensitive, resolves at copy time and is never a fill field or a profile variable. Counting them
   as variables is the most common way this inventory goes wrong.
4. Scan `text`, `subject` and every SOP `steps[].text` and `steps[].title`. A variable that only
   appears in an email subject is easy to miss and just as real.

Report each variable with: how many items use it, which folders, whether it has a description,
whether it is locked, and how many profiles have a non-empty value for it.

## The checks

### 1. Near-duplicate spellings (highest value)

`firstName` beside `first_name` beside `firstname` are three variables to Castline. Every profile
has to fill all three, and any one of them left empty means a template renders a literal
`{{first_name}}` in front of a customer.

Group case-insensitively with separators stripped and flag any group with more than one spelling.
Recommend the spelling with the highest item count as canonical.

### 2. Variables with no description

`profiles.json.descriptions` is per-variable guidance that drives Castline's AI enrich and the
generated agent guide. A variable with no description enriches badly, because the model is guessing
what the field should contain. This is a correctness problem, not tidiness.

List every variable with no description entry, ordered by usage. Draft a description for each,
phrased as what the value must contain, with an example.

### 3. Never filled

Variables used by templates that no profile has a non-empty value for. Each one is a template that
cannot fully render. Distinguish two cases:

- **Locked**, which is correct and expected. Locked variables are meant to be empty and typed on the
  spot. Do not report these as defects.
- **Unlocked and unfilled**, which is a real gap.

### 4. Orphans

Values sitting on profiles for variables no template uses any more. Harmless but noisy, and usually
the residue of a renamed variable. Cross-check against near-duplicates before recommending removal:
an orphan is often the old spelling of a variable that got renamed.

### 5. Locked inventory

List everything in `profiles.json.locked`. This is the set that no enrich path may ever write, so
`/castline-crm-sync` needs it, and it is worth showing the user explicitly so they can confirm the
list is still what they intend.

## Output

A short report in this order: near-duplicates first, then missing descriptions, then never-filled,
then orphans, then the locked list, then the full inventory table.

Lead with the count that matters: how many distinct variables, and how many of those have a
description. That ratio is the health number.

## Hard rules

1. **Read-only.** Never write `library.json` or `profiles.json`. Renaming a variable across a
   library means rewriting templates, which is a blueprint the user imports, not an edit.
2. **Never report auto date tokens as variables.**
3. **Never report a locked variable as an unfilled defect.** Empty is its correct state.
4. **Never propose unlocking a variable** to fix an unfilled warning. The lock is deliberate.
5. **Do not rename anything automatically.** Propose the canonical spelling and let the user decide;
   a rename touches every template and every profile.
