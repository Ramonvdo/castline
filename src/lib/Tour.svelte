<script>
  // The first-run walkthrough: a welcome prompt, then a spotlight tour.
  //
  // The cut-out is four dimmer rects tiling the viewport *around* the target
  // rather than one overlay with a hole. That's deliberate: the target then has
  // nothing covering it, so real clicks reach it natively — which is what the
  // gated steps need — without touching any app DOM or z-index.
  import { onMount } from "svelte";
  import { TOUR_STEPS } from "./tourSteps.js";
  import { itemVars } from "./vars.js";
  import Icon from "./Icon.svelte";

  let {
    // Live app state the gates watch.
    view = "library",
    quickOpen = false,
    fillItem = null,
    copyCount = 0,
    profileCount = 0,
    viewMode = "full",
    firstItem = null, // { name, kind, vars, firstVar } — the spotlit card
    // Drives the app from the tour.
    onView = () => {},
    onViewMode = () => {},
    onCloseFill = () => {},
    onCloseQuickOpen = () => {},
    onFinish = () => {},
  } = $props();

  // "welcome" → the yes/no prompt; "running" → the spotlight; null → nothing.
  let phase = $state(null);
  let idx = $state(0);
  let rect = $state(null); // spotlight box, viewport coords
  let cardPos = $state({ left: 0, top: 0, side: "bottom" });
  let missing = $state(false); // anchor not on screen (yet)

  let step = $derived(TOUR_STEPS[idx] || null);
  let total = TOUR_STEPS.length;

  // ── Gate context ──
  // Gates are pure functions of app state plus a couple of things only the tour
  // observes (a rail click, a copy). Tracked here so a gate can't be satisfied
  // by something that happened before its step opened.
  let railClicked = $state(false);
  let copyBaseline = 0;
  let ctx = $derived({
    view,
    quickOpen,
    fillItem,
    profileCount,
    railClicked,
    copied: copyCount > copyBaseline,
    // What the tour is looking at right now, so its wording can match it.
    item: firstItem,
    openVars: fillItem ? itemVars(fillItem).length : 0,
  });

  // A step's title/body may be a function of that context.
  const resolve = (v) => (typeof v === "function" ? v(ctx) : v);

  // The tour explains the Copy/Fill buttons and the {{variables}} in a card's
  // preview text — none of which exist in the compact densities, where the
  // footer is a hover-only flap positioned outside the card. So it runs in full
  // cards and hands the user's setting back afterwards. Deliberately not
  // persisted, so a crash mid-tour can't overwrite their choice.
  let savedViewMode = null;

  export function start() {
    idx = 0;
    phase = "running";
    if (viewMode !== "full") {
      savedViewMode = viewMode;
      onViewMode("full");
    }
    enterStep();
  }
  export function offerWelcome() {
    phase = "welcome";
  }

  function finish() {
    phase = null;
    rect = null;
    if (savedViewMode !== null) {
      onViewMode(savedViewMode);
      savedViewMode = null;
    }
    onFinish();
  }

  // ── Step lifecycle ──
  let dir = 1; // travel direction, so a skipped step is skipped the same way back
  let scrolled = false; // anchor pulled into view once per step, not per measure

  function enterStep() {
    const s = TOUR_STEPS[idx];
    if (!s) return finish();
    railClicked = false;
    copyBaseline = copyCount;
    scrolled = false;
    // Never leave a modal the previous step opened sitting behind the overlay —
    // unless this step is the one that explains it.
    if (fillItem && s.anchor !== "fill-modal") onCloseFill();
    if (quickOpen && s.anchor !== "palette") onCloseQuickOpen();
    if (s.view && s.view !== view) onView(s.view);
    startTracking();
  }

  function next() {
    dir = 1;
    if (idx >= total - 1) return finish();
    idx += 1;
    enterStep();
  }
  function prev() {
    dir = -1;
    if (idx === 0) return;
    idx -= 1;
    enterStep();
  }

  // A gate is satisfied → advance, but let the user see the result first.
  $effect(() => {
    if (phase !== "running" || !step?.gate) return;
    if (!step.gate.check(ctx)) return;
    const t = setTimeout(next, 420);
    return () => clearTimeout(t);
  });

  // A step that's watched rather than acted on moves along by itself.
  $effect(() => {
    if (phase !== "running" || !step?.dwell) return;
    const t = setTimeout(next, step.dwell);
    return () => clearTimeout(t);
  });

  // ── Tracking ──
  // The spotlight follows its anchor every frame rather than re-measuring on a
  // few known events. Menus opening, grids reflowing and modals animating all
  // move the target, and each missed one leaves the cut-out somewhere the user
  // isn't looking. Only an actual change is written to state, so the cost is a
  // couple of getBoundingClientRect calls per frame and no re-render.
  let raf = 0;
  let loopId = 0;
  let missTicks = 0;
  let lastKey = "";
  const MAX_MISS = 40; // ~0.6s, comfortably longer than a view switch

  function startTracking() {
    const id = ++loopId;
    cancelAnimationFrame(raf);
    missTicks = 0;
    lastKey = "";
    const run = () => {
      if (phase !== "running" || id !== loopId) return;
      measure();
      // measure() may have advanced the step, which starts its own loop.
      if (id === loopId) raf = requestAnimationFrame(run);
    };
    raf = requestAnimationFrame(run);
  }
  function stopTracking() {
    loopId += 1;
    cancelAnimationFrame(raf);
  }

  // Viewport size lives in state so the dimmers re-render on resize even when
  // the spotlight box itself happens to be unchanged.
  let vw = $state(window.innerWidth);
  let vh = $state(window.innerHeight);

  function measure() {
    const s = TOUR_STEPS[idx];
    if (!s) return;
    vw = window.innerWidth;
    vh = window.innerHeight;

    if (!s.anchor) return apply(null, s);

    const el = document.querySelector(`[data-tour="${s.anchor}"]`);
    // A hidden anchor (an `opacity: 0` hover flap, a collapsed panel) measures
    // as a real box in the wrong place, so treat invisible as absent.
    if (!el || !isVisible(el)) {
      missing = true;
      missTicks += 1;
      apply(null, s);
      // It's not coming. A step marked `optional` describes something that may
      // genuinely not exist — an empty library has no card to point at — so
      // skip it rather than spotlighting nothing.
      if (missTicks > MAX_MISS && s.optional) {
        if (dir < 0 && idx > 0) prev();
        else next();
      }
      return;
    }

    // Pull it into view once on entry — a card further down the grid would
    // otherwise be spotlit off screen. Not on every frame, or it would fight
    // the user's own scrolling.
    if (!scrolled) {
      scrolled = true;
      el.scrollIntoView({ block: "nearest", inline: "nearest" });
    }
    missing = false;
    missTicks = 0;

    const r = el.getBoundingClientRect();
    if (r.width === 0 && r.height === 0) return apply(null, s);

    // Merge in anything the step declared, so a popover the user opens on this
    // step lands inside the hole instead of behind the dim.
    let { left, top, right, bottom } = r;
    for (const sel of s.union || []) {
      const ex = document.querySelector(sel);
      if (!ex || !isVisible(ex)) continue;
      const er = ex.getBoundingClientRect();
      if (er.width === 0 && er.height === 0) continue;
      left = Math.min(left, er.left);
      top = Math.min(top, er.top);
      right = Math.max(right, er.right);
      bottom = Math.max(bottom, er.bottom);
    }

    const pad = s.pad ?? 6;
    apply(
      {
        left: Math.max(0, left - pad),
        top: Math.max(0, top - pad),
        width: Math.min(vw, right - left + pad * 2),
        height: Math.min(vh, bottom - top + pad * 2),
        radius: s.radius ?? 9,
      },
      s,
    );
  }

  function isVisible(el) {
    if (!el.isConnected) return false;
    const cs = getComputedStyle(el);
    return cs.visibility !== "hidden" && cs.display !== "none" && Number(cs.opacity) > 0.05;
  }

  // Write to state only when something actually moved — this runs every frame.
  function apply(r, s) {
    const key = r
      ? `${Math.round(r.left)}|${Math.round(r.top)}|${Math.round(r.width)}|${Math.round(r.height)}|${r.radius}|${cardH}|${vw}|${vh}`
      : `none|${cardH}|${vw}|${vh}`;
    if (key === lastKey) return;
    lastKey = key;
    rect = r;
    placeCard(r, s);
  }

  // Card geometry. The width is fixed in CSS; the height is whatever the copy
  // needs, so it's measured — an estimate here puts the card through the
  // spotlight or off the bottom edge on the longer steps.
  const CARD_W = 320;
  const GAP = 14;
  const EDGE = 12;
  let cardEl = $state(null);
  let cardH = $state(190);

  $effect(() => {
    if (!cardEl) return;
    const ro = new ResizeObserver(() => {
      const h = cardEl?.offsetHeight;
      // The tracking loop re-places on the next frame — cardH is part of its
      // change key, so it doesn't need poking here.
      if (h && Math.abs(h - cardH) > 1) cardH = h;
    });
    ro.observe(cardEl);
    return () => ro.disconnect();
  });

  function placeCard(r, s) {
    const CARD_H = cardH;
    if (!r) {
      cardPos = {
        left: (vw - CARD_W) / 2,
        top: (vh - CARD_H) / 2,
        side: "center",
      };
      return;
    }
    const fits = {
      right: vw - (r.left + r.width) >= CARD_W + GAP + EDGE,
      left: r.left >= CARD_W + GAP + EDGE,
      bottom: vh - (r.top + r.height) >= CARD_H + GAP + EDGE,
      top: r.top >= CARD_H + GAP + EDGE,
    };
    // Preferred side first, then anything that fits, then bottom regardless —
    // the clamp below keeps it on screen even in the worst case.
    const order = [s.side || "bottom", "bottom", "right", "left", "top"];
    const side = order.find((k) => fits[k]) || "bottom";

    let left, top;
    if (side === "right") {
      left = r.left + r.width + GAP;
      top = r.top + r.height / 2 - CARD_H / 2;
    } else if (side === "left") {
      left = r.left - CARD_W - GAP;
      top = r.top + r.height / 2 - CARD_H / 2;
    } else if (side === "top") {
      left = r.left + r.width / 2 - CARD_W / 2;
      top = r.top - CARD_H - GAP;
    } else {
      left = r.left + r.width / 2 - CARD_W / 2;
      top = r.top + r.height + GAP;
    }
    cardPos = {
      left: clamp(left, EDGE, vw - CARD_W - EDGE),
      top: clamp(top, EDGE, vh - CARD_H - EDGE),
      side,
    };
  }

  // When the viewport is narrower than the card, `hi` goes below `lo` and this
  // pins to the near edge rather than flipping the card off screen.
  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));

  onMount(() => {
    // Resize and scroll need no listeners — the tracking loop already sees them.

    // Capture phase so Escape means "leave the tour" and doesn't also close the
    // very thing the current step just asked the user to open.
    const onKey = (e) => {
      if (phase !== "running") return;
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        finish();
      }
    };
    window.addEventListener("keydown", onKey, true);

    // The rail step's gate. Picking a folder, All or Pinned counts; "New folder"
    // does not — that opens a modal over the next step rather than showing what
    // this one is about.
    const onClick = (e) => {
      if (phase !== "running") return;
      const btn = e.target.closest?.('[data-tour="rail"] button');
      if (btn && !btn.classList.contains("newfolder")) railClicked = true;
    };
    window.addEventListener("click", onClick, true);

    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("click", onClick, true);
      stopTracking();
    };
  });

  // Nothing to track once the tour is over.
  $effect(() => {
    if (phase !== "running") stopTracking();
  });
</script>

{#if phase === "welcome"}
  <div class="overlay tour-welcome">
    <div class="modal">
      <h3>Hey, welcome to Castline</h3>
      <p class="hint">
        Castline is a shelf for text you reuse — prompts, email templates, notes and multi-step SOPs.
        Want a quick tutorial walkthrough? It takes about a minute.
      </p>
      <div class="modal-actions">
        <button class="ghost" onclick={finish}>No thanks</button>
        <button class="btn" onclick={start}>Yes, show me</button>
      </div>
    </div>
  </div>
{/if}

{#if phase === "running" && step}
  {#if rect}
    <!-- Four dimmers tiling everything except the target. -->
    <div class="tour-block" style:left="0px" style:top="0px" style:width="{vw}px" style:height="{rect.top}px"></div>
    <div
      class="tour-block"
      style:left="0px"
      style:top="{rect.top + rect.height}px"
      style:width="{vw}px"
      style:height="{Math.max(0, vh - rect.top - rect.height)}px"
    ></div>
    <div class="tour-block" style:left="0px" style:top="{rect.top}px" style:width="{rect.left}px" style:height="{rect.height}px"></div>
    <div
      class="tour-block"
      style:left="{rect.left + rect.width}px"
      style:top="{rect.top}px"
      style:width="{Math.max(0, vw - rect.left - rect.width)}px"
      style:height="{rect.height}px"
    ></div>

    <div
      class="tour-ring"
      class:pulse={!!step.gate}
      style:left="{rect.left}px"
      style:top="{rect.top}px"
      style:width="{rect.width}px"
      style:height="{rect.height}px"
      style:border-radius="{rect.radius}px"
    ></div>
  {:else}
    <!-- No anchor, or it hasn't appeared yet. Dim, but never block: with
         nothing spotlit there's nothing to protect, and swallowing clicks here
         would strand the user on exactly the steps that ask them to act. -->
    <div class="tour-block full"></div>
  {/if}

  <div
    class="tour-card"
    bind:this={cardEl}
    style:left="{cardPos.left}px"
    style:top="{cardPos.top}px"
  >
    <div class="tc-head">
      <span class="tc-count">{idx + 1} / {total}</span>
      <button class="tc-exit" onclick={finish} title="Exit tour">
        <Icon name="close" size={14} />
      </button>
    </div>
    <h4>{resolve(step.title)}</h4>
    <!-- Copy comes from tourSteps.js. The only user-derived value it can
         interpolate is a {{variable}} name, escaped at the point of use. -->
    <p>{@html resolve(step.body)}</p>

    {#if step.gate}
      <div class="tc-gate">
        <span class="tc-dot"></span>
        {step.gate.hint}
      </div>
    {/if}

    <div class="tc-actions">
      {#if idx > 0}
        <button class="tc-back" onclick={prev}>Back</button>
      {/if}
      <span class="tc-spacer"></span>
      <button class="tc-next" onclick={next}>
        {step.done ? "Finish" : step.gate ? "Skip this step" : "Next"}
      </button>
    </div>
  </div>
{/if}

<style>
  /* Above every existing layer — the app's ladder tops out at 116. */
  .tour-block {
    position: fixed;
    z-index: 130;
    background: rgba(3, 5, 9, 0.62);
    backdrop-filter: blur(1.5px);
    transition:
      left 0.24s var(--ease),
      top 0.24s var(--ease),
      width 0.24s var(--ease),
      height 0.24s var(--ease);
  }
  .tour-block.full {
    inset: 0;
    width: 100vw;
    height: 100vh;
    pointer-events: none;
  }
  /* Purely decorative — never intercepts the click the step is waiting for. */
  .tour-ring {
    position: fixed;
    z-index: 130;
    pointer-events: none;
    border: 1px solid var(--accent-strong);
    box-shadow:
      0 0 0 1px rgba(3, 5, 9, 0.5),
      0 0 22px -4px color-mix(in srgb, var(--accent) 70%, transparent);
    transition:
      left 0.24s var(--ease),
      top 0.24s var(--ease),
      width 0.24s var(--ease),
      height 0.24s var(--ease);
  }
  .tour-ring.pulse {
    animation: tour-pulse 2s var(--ease) infinite;
  }
  @keyframes tour-pulse {
    0%,
    100% {
      box-shadow:
        0 0 0 1px rgba(3, 5, 9, 0.5),
        0 0 22px -4px color-mix(in srgb, var(--accent) 70%, transparent);
    }
    50% {
      box-shadow:
        0 0 0 1px rgba(3, 5, 9, 0.5),
        0 0 30px 2px color-mix(in srgb, var(--accent) 85%, transparent);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .tour-block,
    .tour-ring {
      transition: none;
    }
    .tour-ring.pulse {
      animation: none;
    }
  }

  .tour-card {
    position: fixed;
    z-index: 131;
    width: 320px;
    background: var(--sheen), var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-modal), var(--edge);
    padding: 13px 15px 12px;
    display: flex;
    flex-direction: column;
    gap: 7px;
    transition:
      left 0.24s var(--ease),
      top 0.24s var(--ease);
  }
  .tc-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .tc-count {
    font-size: 11px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  .tc-exit {
    display: flex;
    background: none;
    border: none;
    color: var(--faint);
    cursor: pointer;
    padding: 2px;
    border-radius: 5px;
  }
  .tc-exit:hover {
    color: var(--text);
    background: var(--elevated);
  }
  .tour-card h4 {
    margin: 0;
    font-size: 14.5px;
    font-weight: 600;
  }
  .tour-card p {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.55;
    color: var(--muted);
  }
  .tour-card :global(b) {
    color: var(--text);
    font-weight: 600;
  }
  .tour-card :global(kbd) {
    font-family: inherit;
    font-size: 10.5px;
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    padding: 1px 5px;
    color: var(--text);
  }
  .tour-card :global(.tour-var) {
    font-family: var(--font-mono);
    font-size: 11.5px;
    color: var(--accent-strong);
    background: var(--accent-soft);
    border-radius: 5px;
    padding: 1px 5px;
  }
  .tour-card :global(.tour-filled) {
    color: var(--accent-strong);
  }
  .tour-card :global(.tour-empty) {
    color: var(--faint);
  }

  .tc-gate {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    color: var(--accent-strong);
    background: var(--accent-soft);
    border-radius: var(--radius-sm);
    padding: 6px 9px;
    margin-top: 1px;
  }
  .tc-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-strong);
    flex: none;
    animation: tour-blink 1.4s var(--ease) infinite;
  }
  @keyframes tour-blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.25;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .tc-dot {
      animation: none;
    }
  }

  .tc-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 3px;
  }
  .tc-spacer {
    flex: 1;
  }
  .tc-back,
  .tc-next {
    background: none;
    border: 1px solid transparent;
    color: var(--muted);
    cursor: pointer;
    font-size: 12.5px;
    padding: 5px 10px;
    border-radius: var(--radius-sm);
    transition:
      color 0.12s var(--ease),
      border-color 0.12s var(--ease),
      background 0.12s var(--ease);
  }
  .tc-back:hover,
  .tc-next:hover {
    color: var(--text);
    border-color: var(--border);
    background: var(--elevated);
  }
  .tc-next {
    color: var(--accent-strong);
  }

  /* The welcome prompt sits above the tour layer too, so a replay can't be
     covered by a stale dimmer. */
  .tour-welcome {
    z-index: 132;
  }
</style>
