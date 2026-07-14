<script>
  import { extractVars, applyVars, itemVars, groupVarsByLayout } from "./vars.js";
  import { clipCopy } from "./api.js";

  // props — `layout` is the global variable grouping (presentation only).
  let { item, profiles = [], layout = [], flash, onClose } = $props();

  let values = $state({});
  let stepIdx = $state(0);
  let profileId = $state("");

  // Seed empty keys so every input is controlled. Depends only on `item` (never
  // reads `values`) so typing / loading a profile doesn't retrigger a reset.
  $effect(() => {
    const seed = {};
    for (const v of itemVars(item)) seed[v] = "";
    values = seed;
    stepIdx = 0;
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
  // Group the fill inputs under the same splitters as the profile editor.
  let stepGroups = $derived(groupVarsByLayout(stepVars, layout).filter((g) => g.vars.length));
  let templateGroups = $derived(groupVarsByLayout(templateVars, layout).filter((g) => g.vars.length));
  let preview = $derived.by(() => {
    if (!item) return "";
    if (isSop) return step ? applyVars(step.text, values) : "";
    return applyVars(item.text, values);
  });

  async function copyFilled() {
    const ok = await clipCopy(preview);
    flash(ok ? (isSop ? "Copied this step" : "Copied filled text") : "Copy failed");
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
    <h3>Fill &amp; copy — {item.name}</h3>

    {#if profiles.length}
      <label>
        Load values from a profile
        <select class="field" bind:value={profileId} onchange={applyProfile}>
          <option value="">— pick a profile —</option>
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

    <label>
      Preview
      <textarea class="field preview-box" rows="7" readonly value={preview}></textarea>
    </label>

    <div class="modal-actions">
      <button class="ghost" onclick={onClose}>Close</button>
      {#if isSop}
        <button class="ghost" onclick={prev} disabled={stepIdx === 0}>← Prev</button>
        <button class="btn" onclick={copyFilled}>Copy this step</button>
        <button class="ghost" onclick={next} disabled={stepIdx === item.steps.length - 1}>Next →</button>
      {:else}
        <button class="btn" onclick={copyFilled}>Copy</button>
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
  .preview-box {
    font-family: ui-monospace, "SF Mono", monospace;
    white-space: pre-wrap;
    font-size: 12.5px;
  }
</style>
