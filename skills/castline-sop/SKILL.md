---
name: castline-sop
description: Turn a described process into a proper Castline multi-step SOP, with ordered steps, variables threaded through, and delivery as an importable blueprint. Matches the house style of the SOPs already in the library. Use when the user says /castline-sop, or asks to turn a process, checklist, runbook or workflow into an SOP in Castline.
argument-hint: [the process, e.g. "our client onboarding from signed contract to kickoff call"]
---

# Castline SOP builder

Turn a process into a runbook someone can actually execute, step by step, with the variable values
filled from a profile.

Read `../castline/reference.md` first for the data directory and schemas, and
`../castline/blueprints.md` for the blueprint format.

The process is: **$ARGUMENTS**

If `$ARGUMENTS` is empty, ask what process to capture, then continue.

## What an SOP is in Castline

`kind: "sop"` with an ordered `steps[]` of `{title, text}`. The preview opens on an overview of all
steps, where hovering a title peeks at that step's filled message, and the user copies them one at a
time as they work.

That copy-per-step behaviour is the whole design constraint: **each step's `text` must be something
worth copying on its own.** A step whose text is narrative description gives the user nothing to
paste, which makes the step a heading pretending to be a step.

## Build it

1. **Read the library first.** There are existing SOPs; match their house style for step
   granularity, title phrasing and tag vocabulary. Read two or three before writing.
2. **Get the process out of the user.** Do not invent steps. Ask for the real sequence, and
   specifically for:
   - what triggers the process and what "done" looks like
   - which steps produce text that gets sent or pasted somewhere
   - which steps are decisions or waits rather than actions
3. **Split into steps.** One action per step. If a step's text would exceed a screen, it is probably
   two steps. If two adjacent steps are always done together and produce one message, they are one.
4. **Write each step's text as the artifact**, not as instructions about the artifact. The step
   "Send the kickoff email" should contain the kickoff email, ready to copy, not the sentence "send
   a kickoff email to the client."
5. **Thread variables through.** Reuse the exact `{{variable}}` names already in the library so one
   profile fills the whole runbook. Use `{{today}}` and `{{now}}` for dates rather than inventing a
   date variable.
6. **Title each step as an action**, short enough to scan in the overview. "Confirm scope in
   writing" beats "Step 3: scope."
7. **Deliver as a blueprint** per `../castline/blueprints.md`, in a folder that matches where the
   user keeps process content.

## Steps that are not steps

Handle these explicitly rather than forcing them into the copy-a-message shape:

- **A decision.** Give the step text the criteria, phrased so the user can read it and decide, and
  name the branches. Castline SOPs are linear, so a branch usually means a second SOP referenced by
  name in the step text.
- **A wait.** State what is being waited for and what to do if it does not arrive. A wait with no
  timeout is where processes stall.
- **A check.** Give a concrete pass condition, not "verify everything looks good."

## Hard rules

1. **Never invent process steps.** If the user's description has a gap, ask. A plausible invented
   step in a runbook someone follows without thinking is worse than an obvious hole.
2. **Every step's `text` must be worth copying.** If it is not, it is a heading, and headings belong
   in the step title or in a preceding step's text.
3. **`kind: "sop"` uses `steps[]`, not `text`.** Leave `text` and `subject` empty for SOP items.
4. **Reuse existing variable spellings exactly.** A new spelling forks every profile.
5. **Deliver as a blueprint.** Never hand-edit `library.json`.
6. **Do not pad.** A four-step process is a four-step SOP. Inventing ceremony to make it look
   thorough makes it less likely to be followed.

## When an SOP is the wrong shape

If the process is really one message with variables, it is a template, not an SOP. Say so and hand
back to `/castline`. A one-step SOP is just a template with extra clicks.
