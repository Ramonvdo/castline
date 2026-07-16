<script>
  import { extractVars, applyVars, itemVars, groupVarsByLayout, VAR_RE, isAutoVar, autoValue } from "./vars.js";
  import { clipCopy, libRecordUse, connectorSend } from "./api.js";
  import Icon from "./Icon.svelte";

  // props
  let { item, mode = "auto", profiles = [], layout = [], activeProfile = null, connectors = [], flash, onClose, onUsed = () => {} } = $props();

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

  // Seed keys; prefill from the active profile (or a picked one) where present.
  $effect(() => {
    const src = activeProfile ? activeProfile.values : {};
    const seed = {};
    for (const v of itemVars(item)) seed[v] = src[v] ?? "";
    values = seed;
    stepIdx = 0;
    profileId = activeProfile ? activeProfile.id : "";
  });

  function applyProfile() {
    const p = profiles.find((p) => p.id === profileId);
    if (!p) return;
    values = { ...values, ...p.values };
  }

  let isSop = $derived(item && item.kind === "sop");
  let step = $derived(isSop ? item.steps[stepIdx] : null);
  let stepVars = $derived(step ? extractVars(step.text) : []);
  let templateVars = $derived(!isSop && item ? extractVars(item.text) : []);
  let stepGroups = $derived(groupVarsByLayout(stepVars, layout).filter((g) => g.vars.length));
  let templateGroups = $derived(groupVarsByLayout(templateVars, layout).filter((g) => g.vars.length));

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
  async function sendTo(c) {
    sendOpen = false;
    sending = true;
    const payload = {
      name: item.name,
      type: item.item_type || "",
      kind: item.kind,
      tags: item.tags || [],
      text: fullText(),
      variables: { ...values },
      profile: profileName(),
    };
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

    {#if isSop}
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

    <div class="preview-wrap">
      <span class="preview-label">Preview — <span class="lg-filled">filled</span> · <span class="lg-empty">empty</span></span>
      <div class="preview-box">{#each previewSegs as s}{#if s.filled}<span class="fv">{s.t}</span>{:else if s.v}<span class="ph">{s.t}</span>{:else}{s.t}{/if}{/each}</div>
    </div>

    <div class="modal-actions">
      <button class="ghost" onclick={onClose}>Close</button>
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
      {#if isSop}
        <button class="ghost" onclick={copyAll}>Copy all</button>
        <button class="ghost" onclick={prev} disabled={stepIdx === 0}>← Prev</button>
        <button class="btn" onclick={copyThis}>{isLastStep ? "Copy & finish" : "Copy & next →"}</button>
      {:else}
        <button class="btn" onclick={copyThis}>Copy</button>
      {/if}
    </div>
  </div>
</div>

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
</style>
