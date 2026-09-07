---
name: castline-create-skill
description: Mine a Castline library for prompt clusters worth turning into a Claude Code skill. Finds groups of related parameterised prompts, designs the onboarding interview from their {{variables}}, works out how each variable should be acquired (ask, scrape a link, attach a file, derive), then hands the design to expert-skill to build. Use when the user says /castline-create-skill, or asks which of their Castline prompts or templates could become a skill, or wants to turn a group of Castline prompts into one command.
argument-hint: [optional focus, e.g. "look at the website design folder"]
---

# Castline to skill

A Castline library is full of prompts someone already parameterised. A cluster of related
parameterised prompts is a skill that has not been packaged yet.

Read `../castline/reference.md` first for the data directory, the schemas and the variable syntax.

Optional focus: **$ARGUMENTS**

This skill **finds the cluster and designs the skill**, especially its onboarding. It hands the
design to a skill-authoring skill to build. It does not write the skill itself.

The intended handoff is `/expert-skill`, which researches external sources and stops for approval
before writing. It is a separate, optional install. If the user does not have it, hand the same
design to whatever authoring flow they do have, or write the skill from the design directly and say
that the research step was skipped.

## The shape you are looking for

Several items, usually in one folder, that do the same job for different subjects or in different
styles, each with `{{variables}}` standing in for the subject.

A worked example. A "Website design" folder holding:

```
URL-to-Website         {{URL}}
Restaurant Website     {{URL_google_maps}}, {{attach_a_menu_photo}}
Cinematic Website      {{companyName}}, {{high-level_description_of_what_company_does_and_for_who}}
Tech Website           {{companyName}}, {{high-level_description_of_what_company_does_and_for_who}}
Nature Website         {{companyName}}, {{high-level_description_of_what_company_does_and_for_who}}
```

That is not five templates. That is one `/create-website` skill whose first question is which
variant, and whose remaining questions are the variables.

## Ranking signals, in order

**Do not rank on `uses` alone.** In a real library most items sit at zero, because people paste
from quick-find, add things in batches, or save a prompt before using it. Ranking on `uses` will
hand you the most-copied one-liner and bury the long methodology document that is the actual
skill. Use these instead:

1. **Cluster size.** Several items doing one job with a shared variable vocabulary. This is the
   strongest signal by a distance, because the cluster itself is the evidence of a repeated process.
2. **Parameterisation.** An item with `{{variables}}` was deliberately written to be reused across
   subjects. That is skill-shaped by construction. An item with none is usually a note.
3. **Substance.** Long items encode methodology. A 500-character item is a message; a
   10,000-character item is a process with judgement in it.
4. **Shared variables across folders.** The same variable appearing in several folders means a
   through-line the user has not named yet.
5. **`uses`, as a tiebreaker only.** Real when present. Meaningless when absent. Never read `uses: 0`
   as evidence against an item.

## Designing the onboarding, which is the real work

The generated skill's onboarding **is** the cluster's variables. Do not hand `/expert-skill` a bare
variable list. For each variable, decide how the value should actually be acquired, because that
decision is what separates a good skill from a form.

| Strategy | When | Example |
|---|---|---|
| **Link and fetch** | The variable is a URL, and everything else can be read from it | `{{URL}}`: ask for the site, fetch it, derive company name, offering and tone |
| **Link and extract** | A URL to a structured source with known fields | `{{URL_google_maps}}`: ask for the Google Business Profile link, read the name, address, hours, category, photos and review themes |
| **File** | The value is an artifact that cannot be derived | `{{attach_a_menu_photo}}`: must be asked for. Say what happens without it |
| **Ask directly** | Genuinely only in the user's head | `{{high-level_description_of_what_company_does_and_for_who}}` |
| **Derive, then confirm** | Obtainable from an earlier answer | `{{companyName}}` from the fetched site, shown for confirmation rather than asked cold |

Two rules that make the difference:

- **Never ask for something you can fetch.** If the user has already given a URL, read it and
  confirm what you found. Asking for a company name straight after being handed the company's
  website is the mark of a form pretending to be an interview.
- **Ask in dependency order.** The link comes first, because answers derived from it remove later
  questions.

## Variant selection

When several items in a cluster take **identical** variables, they are styles of one output, not
different jobs. The generated skill's first question is which variant, and that question needs real
descriptions, not just names. "Cinematic, Tech, Nature" means nothing without a line on what each
produces and when to reach for it.

When items take **different** variables, they are different entry paths to the same goal, and the
routing question is about what the user has, not what they want: a URL, a Google Business Profile,
or only a description.

## The workflow

1. Read `library.json`. Extract variables per item with the real parser semantics from
   `../castline/reference.md`, excluding `{{today}}` and `{{now}}`.
2. Group items by folder, shared variables and name similarity.
3. Score the clusters against the signals above.
4. For the best two or three, sketch: the command name, the variant question if there is one, the
   onboarding sequence with an acquisition strategy per variable, and what the skill outputs.
5. Present them ranked, recommend one, and say why the others are weaker.
6. On the user's pick, hand to `/expert-skill`.

## What to hand `/expert-skill`

- The cluster: item names, their full text, and their variables.
- The proposed command name and what it does in one sentence.
- **The onboarding design**, with the acquisition strategy per variable. This is the part that
  cannot be recovered from the prompt text alone, and it is the reason this skill exists.
- The variant question and the descriptions of each variant.
- What the user says the judgement steps are.

Say plainly that the library content is a **starting point, not a source**. `/expert-skill`
researches external sources and stops for approval before writing, and that checkpoint is the whole
point of it. A cluster of good prompts gives the skill its spine and its onboarding; research gives
it the domain rules the prompts assume but never state.

## Hard rules

1. **Read-only.** Never modify the library.
2. **Never rank on `uses` alone**, and never treat `uses: 0` as evidence against an item.
3. **Never propose a skill for a single fixed message.** That is a template, and Castline is already
   the better home for it.
4. **A cluster needs a shared job, not just a shared folder.** Items filed together because they
   were saved the same week are not a cluster.
5. **Never hand over a bare variable list.** Without acquisition strategies, the generated skill
   becomes a form that asks the user for things it could have fetched.
6. **Do not propose more than about three clusters.** A long list avoids the judgement call this
   skill exists to make.
7. **Do not write the skill here.** Hand to `/expert-skill` so it gets researched sources and a
   `sources.md`.
8. **Say when nothing qualifies.** A library of unrelated one-off prompts has no skill in it yet.
