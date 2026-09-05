---
name: castline-create-skill
description: Mine a Castline library for content worth turning into a Claude Code skill. Ranks candidates by real usage, spots SOPs that are really workflows and prompt clusters that are really one process, then hands the winner to expert-skill to build properly. Use when the user says /castline-create-skill, or asks which of their Castline templates or prompts could become a skill, or wants to turn a Castline SOP into a skill.
argument-hint: [optional focus, e.g. "look at the Claude Code folder"]
---

# Castline to skill

Your Castline library is a record of the text you actually reuse. That makes it an unusually honest
place to look for skills, because `uses` is evidence rather than intention.

Read `../castline/reference.md` first for the data directory and schemas.

Optional focus: **$ARGUMENTS**

This skill **finds and ranks candidates**. It does not write the skill. Building is handed to
`/expert-skill`, which already does that job well and researches sources before writing.

## What makes library content a good skill candidate

Judge against all four. A candidate failing the first two is not worth proposing.

1. **Repeated.** High `uses`, or an SOP that exists because the process recurs. A template used
   forty times encodes a real workflow. One used twice encodes a guess.
2. **Procedural, not just text.** A skill is a process with judgement in it. A single fixed message
   is a template and should stay one; Castline already does that job better than a skill would.
3. **Has decisions in it.** The best candidates contain branching the user currently holds in their
   head: which variant to use when, what to check first, when to stop. That judgement is what a
   skill can carry and a template cannot.
4. **Would benefit from tools.** If executing it means reading files, calling an API or searching,
   a skill adds something. If it is purely text to paste, it does not.

## How to find them

1. Read `library.json`. Note `uses`, `kind`, `tags` and folder for every item.
2. **Rank by `uses` first.** It is the only objective signal in the library and it is right there.
3. **Look at every SOP.** Multi-step items are workflows by construction, so they are the highest
   density of candidates. An SOP whose steps involve checking, deciding or gathering is a skill
   waiting to happen.
4. **Cluster related templates.** Three or four templates that are variants of one job, or that get
   used in sequence, are often one skill whose job is choosing between them and filling them in.
   Detect this by shared tags, shared variables, adjacent folders and similar names.
5. **Read the folder names.** Folders are the user's own taxonomy of their work, so a folder with
   many high-use items names a domain they operate in repeatedly.

## Present the candidates

For each, in a ranked list:

- **What it is**: the item or cluster, with names and folder.
- **Evidence**: total `uses`, item count, whether it is an SOP.
- **The skill it would become**: one sentence on what `/<name>` would do.
- **Why a skill and not a template**: the specific judgement or tool use that a template cannot
  carry. If you cannot answer this crisply, drop the candidate.
- **What it would need**: any API, file access or research the skill would depend on.

End with a recommendation of one, and say why the others are weaker rather than listing them
neutrally.

## Then hand off

Once the user picks, hand to `/expert-skill` with the library content as the starting material. Say
plainly that the library item is a **starting point, not a source**: `expert-skill` researches real
external sources and stops for source approval before writing, and that checkpoint is the whole
point of it. A skill built only from one's own saved prompt is exactly the recollection-based skill
that `expert-skill` exists to avoid.

Bring to the handoff: the item text, its variables, its `uses`, and what the user says the
judgement steps are.

## Hard rules

1. **Read-only.** Never modify the library.
2. **Never propose a skill for a single fixed message.** That is a template, and Castline is already
   the better home for it. Say so.
3. **Rank by real `uses`**, not by which content sounds most impressive.
4. **Do not propose more than about five candidates.** A long list is a way of avoiding the
   judgement call this skill exists to make.
5. **Do not write the skill here.** Hand to `/expert-skill` so it gets researched sources and a
   `sources.md` rather than being assembled from one saved prompt.
6. **Say when nothing qualifies.** A library of one-off messages with low use counts has no skill in
   it yet, and saying so is more useful than manufacturing a candidate.
