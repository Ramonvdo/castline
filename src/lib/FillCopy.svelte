<script>
  import { extractVars, applyVars, itemVars, groupVarsByLayout } from "./vars.js";
  import { clipCopy } from "./api.js";

  // props
  let { item, mode = "auto", profiles = [], layout = [], activeProfile = null, flash, onClose } = $props();

  let values = $state({});
  let stepIdx = $state(0);
  let profileId = $state("");

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

  // Split the preview so any still-unfilled {{placeholder}} can be accent-highlighted.
  const PH_RE = /\{\{\s*[^{}]+?\s*\}\}/g;
  let previewSegs = $derived.by(() => {
    const text = preview;
    const out = [];
    let last = 0;
    let m;
    PH_RE.lastIndex = 0;
    while ((m = PH_RE.exec(text)) !== null) {
      if (m.index > last) out.push({ t: text.slice(last, m.index), v: false });
      out.push({ t: m[0], v: true });
      last = m.index + m[0].length;
    }
    if (last < text.length) out.push({ t: text.slice(last), v: false });
    return out;
  });

  let isLastStep = $derived(isSop && stepIdx >= item.steps.length - 1);

  async function copyThis() {
    const ok = await clipCopy(preview);
    if (!ok) {
      flash("Copy failed");
      return;
    }
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
    if (ok) onClose();
  }
  function next() {
    if (isSop && stepIdx < item.steps.length - 1) stepIdx += 1;
  }
  function prev() {
    if (stepIdx > 0) stepIdx -= 1;
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="overlay" onclick={(e) => e.target === e.currentTarget && onClose()}>
  <div class="modal wide">
    <h3>{isSop ? "Copy step-by-step" : "Fill & copy"} — {item.name}</h3>

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
      <span class="preview-label">Preview</span>
      <div class="preview-box">{#each previewSegs as s}{#if s.v}<span class="ph">{s.t}</span>{:else}{s.t}{/if}{/each}</div>
    </div>

    <div class="modal-actions">
      <button class="ghost" onclick={onClose}>Close</button>
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
  .ph {
    color: var(--accent-strong);
    background: var(--accent-soft);
    border-radius: 4px;
    padding: 0 3px;
    font-weight: 600;
  }
</style>
