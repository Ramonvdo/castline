<script>
  import { extractVars, applyVars, itemVars, groupVarsByLayout, VAR_RE, isAutoVar, autoValue, itemPayload } from "./vars.js";
  import { clipCopy, libRecordUse, connectorSend, llmEnrich } from "./api.js";
  import Icon from "./Icon.svelte";

  // props
  let { item, mode = "auto", profiles = [], layout = [], activeProfile = null, safeMode = true, llm = {}, connectors = [], flash, onClose, onUsed = () => {} } = $props();

  let values = $state({});
  let stepIdx = $state(0);
  let profileId = $state("");

  // Count the item as "used" once per fill session, on the first real copy.
  let counted = false;
  function countUse() {
    if (counted || !item) return;
    counted = true;
    libRecordUse(item.id).then(onUsed).catch(() => {});
  }

  // SOPs open on an OVERVIEW of all steps (hover a title → preview popup),
  // then step through; templates go straight to the single view.
  let stage = $state("single"); // "overview" | "steps" | "single"

  // Hover-preview for a step in the overview (same pattern as the profile
  // editor's long-value panel: grace timer, fixed-position popup).
  let hoverStep = $state(null); // { idx, x, y }
  let hoverTimer;
  function stepEnter(e, idx) {
    clearTimeout(hoverTimer);
    const r = e.currentTarget.getBoundingClientRect();
    const x = Math.min(r.right + 12, window.innerWidth - 420);
    const y = Math.max(10, Math.min(r.top - 10, window.innerHeight - 300));
    hoverStep = { idx, x, y };
  }
  function stepLeave() {
    clearTimeout(hoverTimer);
    hoverTimer = setTimeout(() => (hoverStep = null), 200);
  }
  function stepHoverKeep() {
    clearTimeout(hoverTimer);
  }

  // Seed keys; prefill from the active profile (or a picked one) where present.
  $effect(() => {
    const src = activeProfile ? activeProfile.values : {};
    const seed = {};
    for (const v of itemVars(item)) seed[v] = src[v] ?? "";
    values = seed;
    stepIdx = 0;
    profileId = activeProfile ? activeProfile.id : "";
    stage = item && item.kind === "sop" ? "overview" : "single";
    hoverStep = null;
  });

  function applyProfile() {
    const p = profiles.find((p) => p.id === profileId);
    if (!p) return;
    values = { ...values, ...p.values };
  }

  let isSop = $derived(item && item.kind === "sop");
  let step = $derived(isSop ? item.steps[stepIdx] : null);
  let stepVars = $derived(step ? extractVars(step.text) : []);
  let templateVars = $derived(!isSop && item ? itemVars(item) : []);
  let stepGroups = $derived(groupVarsByLayout(stepVars, layout).filter((g) => g.vars.length));
  let templateGroups = $derived(groupVarsByLayout(templateVars, layout).filter((g) => g.vars.length));
  // The overview shows every variable across all steps, filled once up front.
  let allSopVars = $derived(isSop && item ? itemVars(item) : []);
  let allSopGroups = $derived(groupVarsByLayout(allSopVars, layout).filter((g) => g.vars.length));
  let isEmail = $derived(item && item.item_type === "email" && (item.subject || "").length > 0);

  let preview = $derived.by(() => {
    if (!item) return "";
    if (isSop) return step ? applyVars(step.text, values) : "";
    return applyVars(item.text, values);
  });

  // Segment the RAW text on {{tokens}} so the preview can highlight BOTH
  // filled variables (accent — "this came from a variable") and still-empty
  // placeholders. Live edits re-render immediately.
  function segment(raw) {
    const out = [];
    let last = 0;
    let m;
    // A LOCAL regex instance: the module-level VAR_RE is a shared /g regex and
    // any nested applyVars() call would reset its lastIndex mid-loop.
    const re = new RegExp(VAR_RE.source, "g");
    while ((m = re.exec(raw || "")) !== null) {
      if (m.index > last) out.push({ t: raw.slice(last, m.index) });
      const name = m[1].trim();
      const val = isAutoVar(name) ? autoValue(name) : values[name];
      if (val !== undefined && val !== null && val !== "") {
        out.push({ t: val, filled: true });
      } else {
        out.push({ t: m[0], v: true });
      }
      last = m.index + m[0].length;
    }
    if (last < (raw || "").length) out.push({ t: raw.slice(last) });
    return out;
  }
  let previewSegs = $derived.by(() => {
    if (!item) return [];
    return segment(isSop ? step?.text || "" : item.text);
  });

  let isLastStep = $derived(isSop && stepIdx >= item.steps.length - 1);

  async function copyThis() {
    const ok = await clipCopy(preview);
    if (!ok) {
      flash("Copy failed");
      return;
    }
    countUse();
    if (isSop) {
      if (isLastStep) {
        flash(`Copied step ${stepIdx + 1} · done`);
        onClose();
      } else {
        flash(`Copied step ${stepIdx + 1}`);
        stepIdx += 1;
      }
    } else {
      flash("Copied");
      onClose();
    }
  }
  async function copyAll() {
    const text = (item.steps || []).map((s) => applyVars(s.text, values)).join("\n\n");
    const ok = await clipCopy(text);
    flash(ok ? "Copied all steps" : "Copy failed");
    if (ok) {
      countUse();
      onClose();
    }
  }
  function next() {
    if (isSop && stepIdx < item.steps.length - 1) stepIdx += 1;
  }
  function prev() {
    if (stepIdx > 0) stepIdx -= 1;
  }

  // ── Per-step actions (SOP overview): direct copy + send one step ──
  let stepSend = $state(null); // { idx, x, y }
  async function copyStep(i) {
    const ok = await clipCopy(applyVars(item.steps[i].text, values));
    flash(ok ? `Copied step ${i + 1}` : "Copy failed");
    if (ok) countUse();
  }
  async function sendStepTo(i, c) {
    stepSend = null;
    const s = item.steps[i];
    if (safeMode) {
      const missing = stillUnfilled(s.text);
      if (missing.length) {
        flash(`Safe mode: fill ${missing.slice(0, 3).join(", ")}${missing.length > 3 ? "…" : ""} first`);
        return;
      }
    }
    const payload = {
      name: item.name,
      kind: "sop-step",
      step: s.title || `Step ${i + 1}`,
      index: i + 1,
      of: item.steps.length,
      text: applyVars(s.text, values),
      profile: profileName(),
      variables: { ...values },
    };
    try {
      const res = await connectorSend(c.url, JSON.stringify(payload));
      flash(
        res.status >= 200 && res.status < 300
          ? `Sent step ${i + 1} → ${c.name || "webhook"}`
          : `Webhook answered ${res.status}`,
      );
    } catch (e) {
      flash(String(e));
    }
  }

  // ── AI fill (ephemeral): fill ONLY the empty variables from this template's
  // context — never saved to any profile, just for this copy/send. ──
  let aiBusy = $state(false);
  let llmReady = $derived(!!(llm && llm.api_key));
  function rawItemContext() {
    const parts = [];
    if (item.subject) parts.push(`Subject: ${item.subject}`);
    if (item.kind === "sop") {
      for (const s of item.steps || []) parts.push(`## ${s.title}\n${s.text}`);
    } else {
      parts.push(item.text || "");
    }
    return `### ${item.name}\n${parts.join("\n\n")}`;
  }
  async function aiFill() {
    const empty = itemVars(item).filter((v) => !values[v]);
    if (!empty.length) {
      flash("Nothing to fill — all variables have values");
      return;
    }
    aiBusy = true;
    try {
      const tone = profiles.find((p) => p.id === profileId)?.tone || activeProfile?.tone || "";
      const body = await llmEnrich(JSON.stringify(values), "", null, tone, false, false, rawItemContext());
      const obj = JSON.parse(body);
      let n = 0;
      for (const [k, v] of Object.entries(obj)) {
        if (empty.includes(k) && v) {
          values[k] = String(v);
          n += 1;
        }
      }
      flash(n ? `AI filled ${n} variable${n === 1 ? "" : "s"} — not saved to the profile` : "The AI returned nothing usable");
    } catch (e) {
      flash(String(e));
    }
    aiBusy = false;
  }

  // ── Send the previewed (filled) message to a connector ──
  // Payload carries the current variable values too, so an automation can use
  // e.g. {{email}} as the destination in one click.
  let sendOpen = $state(false);
  let sending = $state(false);
  const profileName = () =>
    profiles.find((p) => p.id === profileId)?.name || activeProfile?.name || null;
  function fullText() {
    // SOP → all steps filled and joined; template → the live preview.
    return isSop ? (item.steps || []).map((s) => applyVars(s.text, values)).join("\n\n") : preview;
  }
  async function copySubject() {
    const ok = await clipCopy(applyVars(item.subject || "", values));
    flash(ok ? "Subject copied" : "Copy failed");
  }

  // Safe mode: names still unfilled in `text` after applying current values.
  function stillUnfilled(text) {
    return extractVars(applyVars(text, values));
  }

  async function sendTo(c) {
    sendOpen = false;
    if (safeMode) {
      const missing = stillUnfilled((item.subject || "") + "\n" + (isSop ? (item.steps || []).map((s) => s.text).join("\n") : item.text || ""));
      if (missing.length) {
        flash(`Safe mode: fill ${missing.slice(0, 3).join(", ")}${missing.length > 3 ? "…" : ""} first`);
        return;
      }
    }
    sending = true;
    // Shared shape: subject mapped separately, text (stacked), text_pages
    // (--- separated), steps[] — filled with the CURRENT (live-edited) values.
    const payload = itemPayload(item, { ...values }, profileName());
    try {
      const res = await connectorSend(c.url, JSON.stringify(payload));
      flash(
        res.status >= 200 && res.status < 300
          ? `Sent “${item.name}” → ${c.name || "webhook"}`
          : `Webhook answered ${res.status}`,
      );
      if (res.status >= 200 && res.status < 300) countUse();
    } catch (e) {
      flash(String(e));
    }
    sending = false;
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="overlay" onclick={(e) => e.target === e.currentTarget && onClose()}>
  <div class="modal wide">
    <h3>{activeProfile ? "Preview" : isSop ? "Copy step-by-step" : "Fill & copy"} — {item.name}</h3>

    {#if profiles.length}
      <label>
        Fill from a profile
        <select class="field" bind:value={profileId} onchange={applyProfile}>
          <option value="">— none —</option>
          {#each profiles as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
        </select>
      </label>
    {/if}

    {#if isSop && stage === "overview"}
      <!-- Overview: fill every variable once, scan the steps, hover to peek. -->
      {#if allSopVars.length}
        <div class="fills">
          {#each allSopGroups as g}
            {#if g.label}<div class="fgroup"><span>{g.label}</span><span class="fline"></span></div>{/if}
            {#each g.vars as v (v)}
              <label class="fill-row"><span class="vchip">{v}</span><input class="field" bind:value={values[v]} /></label>
            {/each}
          {/each}
        </div>
      {/if}

      <div class="ov-wrap">
        <span class="preview-label">{item.steps.length} steps — hover to preview, click to jump in</span>
        <div class="ov-steps">
          {#each item.steps as s, i (s.id || i)}
            <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
            <div
              class="ov-step"
              onmouseenter={(e) => stepEnter(e, i)}
              onmouseleave={stepLeave}
              onclick={() => {
                hoverStep = null;
                stepIdx = i;
                stage = "steps";
              }}
            >
              <span class="ov-n">{i + 1}</span>
              <span class="ov-title">{s.title || `Step ${i + 1}`}</span>
              <span class="ov-acts">
                <button
                  class="icon-btn xs"
                  title="Copy this step"
                  onclick={(e) => {
                    e.stopPropagation();
                    copyStep(i);
                  }}><Icon name="copy" size={14} /></button
                >
                {#if connectors.length}
                  <button
                    class="icon-btn xs"
                    title="Send this step to a webhook"
                    onclick={(e) => {
                      e.stopPropagation();
                      hoverStep = null;
                      stepSend = { idx: i, x: e.clientX, y: e.clientY };
                    }}><Icon name="plug" size={14} /></button
                  >
                {/if}
              </span>
              <Icon name="chevronRight" size={14} />
            </div>
          {/each}
        </div>
      </div>
    {:else if isSop}
      <div class="stepper">
        <span class="step-pos">Step {stepIdx + 1} / {item.steps.length}</span>
        <strong>{step?.title}</strong>
      </div>
      {#if stepVars.length}
        <div class="fills">
          {#each stepGroups as g}
            {#if g.label}<div class="fgroup"><span>{g.label}</span><span class="fline"></span></div>{/if}
            {#each g.vars as v (v)}
              <label class="fill-row"><span class="vchip">{v}</span><input class="field" bind:value={values[v]} /></label>
            {/each}
          {/each}
        </div>
      {/if}
    {:else if templateVars.length}
      <div class="fills">
        {#each templateGroups as g}
          {#if g.label}<div class="fgroup"><span>{g.label}</span><span class="fline"></span></div>{/if}
          {#each g.vars as v (v)}
            <label class="fill-row"><span class="vchip">{v}</span><input class="field" bind:value={values[v]} /></label>
          {/each}
        {/each}
      </div>
    {/if}

    {#if !isSop && isEmail}
      <div class="subj-row">
        <span class="preview-label">Subject <span class="dim-note">— mapped separately in webhooks</span></span>
        <div class="subj-line">
          <div class="subj-box">{#each segment(item.subject) as s}{#if s.filled}<span class="fv">{s.t}</span>{:else if s.v}<span class="ph">{s.t}</span>{:else}{s.t}{/if}{/each}</div>
          <button class="icon-btn" title="Copy subject" onclick={copySubject}><Icon name="copy" size={14} /></button>
        </div>
      </div>
    {/if}

    {#if !(isSop && stage === "overview")}
      <div class="preview-wrap">
        <span class="preview-label">Preview — <span class="lg-filled">filled</span> · <span class="lg-empty">empty</span></span>
        <div class="preview-box">{#each previewSegs as s}{#if s.filled}<span class="fv">{s.t}</span>{:else if s.v}<span class="ph">{s.t}</span>{:else}{s.t}{/if}{/each}</div>
      </div>
    {/if}

    <div class="modal-actions">
      <button class="ghost" onclick={onClose}>Close</button>
      {#if llmReady && itemVars(item).length}
        <button
          class="ghost aifill"
          disabled={aiBusy}
          title="One AI call fills ONLY the empty variables using this template as context — nothing is saved to the profile"
          onclick={aiFill}
        >
          <Icon name="sparkle" size={13} /> {aiBusy ? "Filling…" : "AI fill"}
        </button>
      {/if}
      {#if connectors.length}
        <div class="send-wrap">
          <button class="ghost" disabled={sending} onclick={() => (sendOpen = !sendOpen)}>
            <Icon name="plug" size={13} /> {sending ? "Sending…" : "Send webhook ▾"}
          </button>
          {#if sendOpen}
            <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
            <div class="send-backdrop" onclick={() => (sendOpen = false)}></div>
            <div class="send-menu">
              {#each connectors as c (c.id)}
                <button class="smi" onclick={() => sendTo(c)}>{c.name || c.url}</button>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
      {#if isSop && stage === "overview"}
        <button class="ghost" onclick={copyAll}>Copy all</button>
        <button
          class="btn"
          onclick={() => {
            hoverStep = null;
            stepIdx = 0;
            stage = "steps";
          }}>Step-by-step →</button
        >
      {:else if isSop}
        <button class="ghost" onclick={() => { hoverStep = null; stage = "overview"; }}>☰ Overview</button>
        <button class="ghost" onclick={copyAll}>Copy all</button>
        <button class="ghost" onclick={prev} disabled={stepIdx === 0}>← Prev</button>
        <button class="btn" onclick={copyThis}>{isLastStep ? "Copy & finish" : "Copy & next →"}</button>
      {:else}
        <button class="btn" onclick={copyThis}>Copy</button>
      {/if}
    </div>
  </div>
</div>

{#if stepSend}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="ss-backdrop" onclick={() => (stepSend = null)}></div>
  <div class="ss-menu" style:left="{Math.min(stepSend.x, window.innerWidth - 240)}px" style:top="{stepSend.y}px">
    <div class="ss-label"><Icon name="plug" size={12} /> Send step {stepSend.idx + 1} to</div>
    {#each connectors as c (c.id)}
      <button class="smi" onclick={() => sendStepTo(stepSend.idx, c)}>{c.name || c.url}</button>
    {/each}
  </div>
{/if}

{#if hoverStep && item.steps[hoverStep.idx]}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="step-pop"
    style:left="{hoverStep.x}px"
    style:top="{hoverStep.y}px"
    onmouseenter={stepHoverKeep}
    onmouseleave={stepLeave}
  >
    <span class="sp-title">{item.steps[hoverStep.idx].title || `Step ${hoverStep.idx + 1}`}</span>
    <div class="sp-body">{#each segment(item.steps[hoverStep.idx].text) as s}{#if s.filled}<span class="fv">{s.t}</span>{:else if s.v}<span class="ph">{s.t}</span>{:else}{s.t}{/if}{/each}</div>
  </div>
{/if}

<style>
  .stepper {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .step-pos {
    font-size: 12px;
    color: var(--muted);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 2px 8px;
  }
  .fills {
    display: flex;
    flex-direction: column;
    gap: 9px;
  }
  .fgroup {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 4px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
  }
  .fgroup .fline {
    flex: 1;
    height: 1px;
    background: var(--border);
  }
  .fill-row {
    display: grid;
    grid-template-columns: 150px 1fr;
    align-items: center;
    gap: 10px;
  }
  .fill-row :global(.vchip) {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .preview-wrap {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .preview-label {
    font-size: 12px;
    color: var(--muted);
  }
  .preview-box {
    font-family: var(--font-mono);
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 12.5px;
    line-height: 1.55;
    background: var(--well);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.28);
    padding: 11px 12px;
    max-height: 220px;
    overflow-y: auto;
  }
  /* Still-empty placeholder — quiet, "needs a value". */
  .ph {
    color: var(--faint);
    background: transparent;
    border: 1px dashed var(--border-strong);
    border-radius: 4px;
    padding: 0 3px;
  }
  /* Filled variable — accent, "this came from your profile". */
  .fv {
    color: var(--accent-strong);
    background: var(--accent-soft);
    border-radius: 4px;
    padding: 0 3px;
    font-weight: 600;
  }
  .preview-label .lg-filled {
    color: var(--accent-strong);
    background: var(--accent-soft);
    border-radius: 3px;
    padding: 0 4px;
  }
  .preview-label .lg-empty {
    color: var(--faint);
    border: 1px dashed var(--border-strong);
    border-radius: 3px;
    padding: 0 4px;
  }
  .send-wrap {
    position: relative;
    margin-right: auto;
  }
  .send-backdrop {
    position: fixed;
    inset: 0;
    z-index: 90;
  }
  .send-menu {
    position: absolute;
    bottom: calc(100% + 6px);
    left: 0;
    z-index: 91;
    min-width: 190px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-modal), var(--edge);
    padding: 5px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .smi {
    text-align: left;
    background: none;
    border: none;
    color: var(--text);
    cursor: pointer;
    font-size: 13px;
    padding: 7px 9px;
    border-radius: var(--radius-sm);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 260px;
  }
  .smi:hover {
    background: var(--elevated);
  }

  /* ── SOP overview ── */
  .ov-wrap {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .ov-steps {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .ov-step {
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
    background: var(--well);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    cursor: pointer;
    font-size: 13.5px;
    padding: 10px 12px;
    transition: border-color 0.12s var(--ease), background 0.12s var(--ease);
  }
  .ov-step:hover {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
    background: var(--elevated);
  }
  .ov-n {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent-strong);
    font-size: 11.5px;
    font-weight: 700;
    flex-shrink: 0;
  }
  .ov-title {
    flex: 1;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ov-step :global(.ic) {
    color: var(--faint);
  }
  .ov-acts {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.12s var(--ease);
  }
  .ov-step:hover .ov-acts {
    opacity: 1;
  }
  .icon-btn.xs {
    padding: 5px;
  }
  .aifill {
    color: var(--accent-strong);
  }
  .ss-backdrop {
    position: fixed;
    inset: 0;
    z-index: 115;
  }
  .ss-menu {
    position: fixed;
    z-index: 116;
    min-width: 200px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-modal), var(--edge);
    padding: 5px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .ss-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10.5px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--faint);
    padding: 5px 9px 2px;
  }

  /* Hover popup: the step's filled message, beside the row. */
  .step-pop {
    position: fixed;
    z-index: 110;
    width: 400px;
    max-height: 320px;
    overflow-y: auto;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-modal), var(--edge);
    padding: 12px 13px;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .sp-title {
    font-size: 12px;
    font-weight: 700;
    color: var(--accent-strong);
  }
  .sp-body {
    font-family: var(--font-mono);
    font-size: 12.5px;
    line-height: 1.55;
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--text);
  }

  /* ── Email subject line ── */
  .subj-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .dim-note {
    color: var(--faint);
    font-weight: 400;
  }
  .subj-line {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .subj-box {
    flex: 1;
    font-family: var(--font-mono);
    font-size: 12.5px;
    line-height: 1.5;
    background: var(--well);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 8px 10px;
    overflow-x: auto;
    white-space: nowrap;
  }
</style>
