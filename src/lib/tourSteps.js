// The first-run walkthrough, as data. Tour.svelte renders these; nothing here
// knows about the DOM beyond an `anchor` name.
//
// Each step:
//   anchor  — the `data-tour="…"` attribute to spotlight. Null = a centred card
//             with no cut-out (used for the opening and closing beats).
//   side    — preferred placement of the coach-mark; flipped automatically when
//             it wouldn't fit.
//   view    — the tab this step lives on. The tour switches to it on entry, so
//             a step is never explaining something that's off screen.
//   gate    — the step advances when the user really does the thing, rather
//             than on Next. `check(ctx)` is re-evaluated whenever app state
//             changes; returning true advances. Steps without a gate advance on
//             Next. Every gated step still renders Skip, so nobody gets stuck.
//   pad/radius — cut-out padding and corner rounding, in px.
//   optional — the step is skipped silently when its anchor is missing (e.g.
//             the user deleted the seeded cards before replaying the tour).
//   union   — extra selectors merged into the cut-out when they're on screen.
//             App popovers live at z-index 40-116, well under the tour's
//             dimmers, so anything the user opens on a step has to be part of
//             the hole or it's invisible behind the dim.
//   dwell   — auto-advance after N ms, for a beat that's meant to be watched
//             rather than acted on.

// A variable name comes from the user's own library, so it never goes into the
// `body` HTML unescaped.
const esc = (s) =>
  String(s).replace(
    /[&<>"']/g,
    (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c],
  );
const varChip = (name) => `<span class="tour-var">{{${esc(name)}}}</span>`;

export const TOUR_STEPS = [
  {
    id: "welcome",
    anchor: null,
    view: "library",
    title: "Welcome to Castline",
    body: "Castline is a shelf for text you reuse — AI prompts, email templates, notes and multi-step SOPs. Cast it once, fill it in a second, paste it anywhere.",
  },
  {
    id: "rail",
    anchor: "rail",
    side: "right",
    view: "library",
    pad: 6,
    title: "Your folders",
    body: "Everything lives in folders. <b>All</b> spans every folder at once, and <b>Pinned</b> collects the ones you starred.",
    gate: {
      hint: "Click a folder to continue",
      // Any rail selection counts — folder, All or Pinned.
      check: (ctx) => ctx.railClicked,
    },
  },
  {
    id: "card",
    anchor: "card",
    side: "right",
    view: "library",
    pad: 8,
    optional: true,
    title: "An item",
    body: (ctx) =>
      ctx.item?.vars
        ? `This one has ${varChip(ctx.item.firstVar)} in it — anything wrapped in double braces is a blank you fill at copy time. The corner counts how often you've copied it.`
        : "Plain reusable text, ready to copy. Wrap any part of an item in <b>double braces</b> and it becomes a blank you fill at copy time. The corner counts how often you've copied it.",
  },
  {
    id: "card-actions",
    anchor: "card-actions",
    side: "right",
    view: "library",
    pad: 8,
    optional: true,
    title: "Copy, or open it",
    body: (ctx) =>
      ctx.item?.vars
        ? "<b>Copy</b> takes the text as-is. <b>Fill</b> opens it so you can read the whole thing and fill the blanks first."
        : "<b>Copy</b> takes the text as-is. <b>View</b> opens it so you can read the whole thing first — it's on every item, including ones with nothing to fill.",
    gate: {
      hint: "Open this item to continue",
      check: (ctx) => !!ctx.fillItem,
    },
  },
  {
    id: "fill",
    anchor: "fill-modal",
    side: "left",
    view: "library",
    pad: 10,
    radius: 13,
    optional: true,
    // Keyed off the item actually open, not the one the tour pointed at — the
    // user may well have opened a different card.
    title: (ctx) => (ctx.openVars ? "Fill and copy" : "Read it, then copy"),
    body: (ctx) =>
      ctx.openVars
        ? 'Type a value and watch the preview fill in — <span class="tour-filled">filled</span> against <span class="tour-empty">still empty</span>. Then copy the finished text.'
        : "Nothing to fill on this one, so it's just the full text — readable without opening the editor. Every item opens here, blanks or not.",
    gate: {
      hint: "Copy it to continue",
      check: (ctx) => ctx.copied,
    },
  },
  {
    id: "profiles",
    anchor: "nav-profiles",
    side: "bottom",
    view: "library",
    pad: 4,
    title: "Stop retyping the same values",
    body: "A <b>profile</b> is a saved set of variable values — a client, a project, yourself. Fill once, reuse forever.",
    gate: {
      hint: "Open the Profiles tab to continue",
      check: (ctx) => ctx.view === "profiles",
    },
  },
  {
    id: "new-profile",
    // The whole view, not the button: clicking "New profile" swaps the button
    // for the editor, and an anchor that vanishes mid-step would leave the user
    // filling a form they can't reach through the overlay.
    anchor: "profiles-view",
    side: "bottom",
    view: "profiles",
    pad: 6,
    radius: 13,
    title: "Make your first one",
    body: "Hit <b>New profile</b>, give it a name, fill in the values you keep retyping, then head back — it saves itself on the way out.",
    gate: {
      hint: "Create a profile to continue",
      check: (ctx) => ctx.profileCount > 0,
    },
  },
  {
    id: "profsel",
    anchor: "profsel",
    side: "bottom",
    view: "profiles",
    pad: 4,
    // The selector's own dropdown sits far below the tour's dimmers, so it has
    // to join the cut-out or opening it looks broken.
    union: ['[data-tour="profsel"] .menu'],
    title: "Then pick one here",
    body: "With a profile active, every <b>Copy</b> comes out already filled — no dialog at all. Switch profile here any time.",
  },
  {
    id: "connectors",
    anchor: "nav-connectors",
    side: "bottom",
    view: "library",
    pad: 4,
    title: "It's not only a shelf",
    body: "Paste a <b>Make</b> or <b>n8n</b> webhook URL and Castline can POST straight to it — right-click any card to fire that item off, or <b>schedule</b> it to send daily, weekly or monthly on its own. It listens too: a token-gated local endpoint lets an automation write profiles back in.",
  },
  {
    id: "quickfind",
    anchor: "quickfind",
    side: "bottom",
    view: "library",
    pad: 4,
    title: "The fastest way in",
    body: "<kbd>Ctrl</kbd>+<kbd>K</kbd> from anywhere in the app. Type a few letters, hit Enter, it's on your clipboard.",
    gate: {
      hint: "Press Ctrl+K to continue",
      check: (ctx) => ctx.quickOpen,
    },
  },
  {
    id: "palette",
    anchor: "palette",
    side: "bottom",
    view: "library",
    pad: 8,
    radius: 13,
    // Held open and lit for a beat so the palette is actually seen, rather than
    // flashing past on the way to the closing card. Optional so that closing it
    // early just moves the tour along instead of stranding it.
    dwell: 5000,
    optional: true,
    title: "This is it",
    body: "Fuzzy-matches every item in every folder. <kbd>↑</kbd><kbd>↓</kbd> to move, <kbd>Enter</kbd> to copy, <kbd>Esc</kbd> to close.",
  },
  {
    id: "done",
    anchor: null,
    view: "library",
    title: "That's the whole loop",
    body: "Add your own with <b>New item</b>. Still to find: multi-step <b>SOPs</b>, <b>blueprints</b> to share templates as a file, and the <b>Agent</b> tab, which runs Claude Code against your own library. Everything stays in plain JSON on this machine.",
    done: true,
  },
];
