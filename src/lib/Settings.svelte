<script>
  import { onMount } from "svelte";
  import {
    getDataDir,
    revealDataDir,
    pickSaveFile,
    pickOpenFile,
    exportLibraryTo,
    importLibraryFrom,
    exportProfilesTo,
    importProfilesFrom,
    profilesSetDescriptions,
    setLlmConfig,
    setSchedules,
    runScheduleNow,
    getSettings,
    getProfiles,
  } from "./api.js";
  import { allLibraryVars } from "./vars.js";
  import Icon from "./Icon.svelte";

  let {
    flash,
    folders = [],
    connectors = [],
    onLibraryData,
    onProfilesData,
    onSettings = () => {},
  } = $props();
  let dataDir = $state("");

  // ── Variables (descriptions = AI context) ──
  let descs = $state({}); // name -> description (editable copy)
  let storedDescs = $state({}); // as loaded from profiles.json

  // Reactive: folders can arrive after mount (App loads the library async).
  let varNames = $derived.by(() => {
    const names = allLibraryVars(folders);
    for (const k of Object.keys(storedDescs)) if (!names.includes(k)) names.push(k);
    return names;
  });
  // Seed editable rows as names appear, without clobbering in-progress edits.
  $effect(() => {
    const d = { ...descs };
    let changed = false;
    for (const n of varNames) {
      if (!(n in d)) {
        d[n] = storedDescs[n] || "";
        changed = true;
      }
    }
    if (changed) descs = d;
  });

  // ── AI workflow (OpenRouter) ──
  let llmKey = $state("");
  let llmModel = $state("google/gemini-2.5-flash");
  let llmWeb = $state(false);

  // ── Scheduled webhooks ──
  let schedules = $state([]);
  let schedDirty = $state(false);

  // Every item across the library, for the "send one item" schedule picker.
  let allItems = $derived.by(() => {
    const out = [];
    for (const f of folders || [])
      for (const i of f.items || []) out.push({ id: i.id, label: `${i.name} · ${f.name}` });
    return out;
  });

  onMount(async () => {
    dataDir = await getDataDir();
    const s = await getSettings();
    llmKey = s.llm?.api_key || "";
    llmModel = s.llm?.model || "google/gemini-2.5-flash";
    llmWeb = !!s.llm?.web_search;
    schedules = (s.schedules || []).map((x) => ({ ...x }));

    const p = await getProfiles();
    storedDescs = p.descriptions || {};
    // Backfill stored descriptions into rows already seeded with "".
    const d = { ...descs };
    for (const [k, v] of Object.entries(storedDescs)) if (!d[k]) d[k] = v;
    descs = d;
  });

  async function saveDescriptions() {
    const data = await profilesSetDescriptions({ ...descs });
    onProfilesData(data);
    flash("Variable descriptions saved");
  }

  async function saveLlm() {
    const s = await setLlmConfig(llmKey, llmModel, llmWeb);
    onSettings(s);
    flash(llmKey.trim() ? "AI workflow saved" : "AI workflow saved (no key — enrich disabled)");
  }

  function addSchedule() {
    schedules = [
      ...schedules,
      {
        id: "",
        kind: "profiles",
        item_id: allItems[0]?.id || "",
        connector_id: connectors[0]?.id || "",
        every: "week",
        last_run: 0,
      },
    ];
    schedDirty = true;
  }
  function removeSchedule(i) {
    schedules = schedules.filter((_, idx) => idx !== i);
    schedDirty = true;
  }
  async function saveSchedules() {
    const s = await setSchedules(schedules.map((x) => ({ ...x })));
    onSettings(s);
    schedules = (s.schedules || []).map((x) => ({ ...x }));
    schedDirty = false;
    flash("Schedules saved");
  }
  async function runNow(sch) {
    try {
      const msg = await runScheduleNow(sch.id);
      flash(msg);
      const s = await getSettings();
      schedules = (s.schedules || []).map((x) => ({ ...x }));
      onSettings(s);
    } catch (e) {
      flash(String(e));
    }
  }
  const lastRunLabel = (ts) =>
    ts ? new Date(ts * 1000).toLocaleString() : "never";

  async function reveal() {
    try {
      await revealDataDir();
    } catch (e) {
      flash(String(e));
    }
  }
  async function exportLibrary() {
    const path = await pickSaveFile("castline-library.json");
    if (!path) return;
    try {
      await exportLibraryTo(path);
      flash("Library exported");
    } catch (e) {
      flash(String(e));
    }
  }
  async function importLibrary(mode) {
    const path = await pickOpenFile();
    if (!path) return;
    if (
      mode === "replace" &&
      !confirm(
        "Replace your entire library with this file? This cannot be undone.",
      )
    )
      return;
    try {
      onLibraryData(await importLibraryFrom(path, mode));
      flash(mode === "replace" ? "Library replaced" : "Library merged");
    } catch (e) {
      flash(String(e));
    }
  }
  async function exportProfiles() {
    const path = await pickSaveFile("castline-profiles.json");
    if (!path) return;
    try {
      await exportProfilesTo(path);
      flash("Profiles exported");
    } catch (e) {
      flash(String(e));
    }
  }
  async function importProfiles(mode) {
    const path = await pickOpenFile();
    if (!path) return;
    if (mode === "replace" && !confirm("Replace all profiles with this file?"))
      return;
    try {
      onProfilesData(await importProfilesFrom(path, mode));
      flash(mode === "replace" ? "Profiles replaced" : "Profiles merged");
    } catch (e) {
      flash(String(e));
    }
  }
</script>

<div class="view">
  <section class="panel">
    <h4>Data & backups</h4>
    <div class="loc">
      <code class="path">{dataDir || "…"}</code>
      <button class="ghost" onclick={reveal}
        ><Icon name="folderOpen" size={15} /> Open folder</button
      >
    </div>
    <div class="grid2">
      <div class="stack">
        <strong>Library</strong>
        <div class="row-btns">
          <button class="ghost" onclick={exportLibrary}>Export</button>
          <button class="ghost" onclick={() => importLibrary("merge")}
            >Import (merge)</button
          >
          <button class="ghost" onclick={() => importLibrary("replace")}
            >Import (replace)</button
          >
        </div>
      </div>
      <div class="stack">
        <strong>Profiles</strong>
        <div class="row-btns">
          <button class="ghost" onclick={exportProfiles}>Export</button>
          <button class="ghost" onclick={() => importProfiles("merge")}
            >Import (merge)</button
          >
          <button class="ghost" onclick={() => importProfiles("replace")}
            >Import (replace)</button
          >
        </div>
      </div>
    </div>
  </section>

  <section class="panel">
    <h4>Variables</h4>
    <p class="hint">
      Every <code>{"{{variable}}"}</code> your library currently uses. Describe what each value must
      look like — the <strong>Castline AI</strong> enrich and the <strong>Agent</strong> follow these
      descriptions to the letter.
    </p>
    {#if varNames.length === 0}
      <p class="hint dim">No variables in the library yet — add {"{{placeholders}}"} to your templates.</p>
    {:else}
      <div class="varlist">
        {#each varNames as n (n)}
          <div class="varrow">
            <code class="vname">{n}</code>
            <input
              class="field"
              bind:value={descs[n]}
              placeholder={n === "companyName"
                ? `e.g. simplified lowercase company name — "RocketFarm Studios LLC" becomes "rocketfarm"`
                : "What should this value contain? Format, casing, an example…"}
            />
          </div>
        {/each}
      </div>
      <div class="row-end">
        <button class="btn" onclick={saveDescriptions}>Save descriptions</button>
      </div>
    {/if}
  </section>

  <section class="panel">
    <h4>AI workflow (Castline AI enrich)</h4>
    <p class="hint">
      Powers the <strong>Enrich → Castline AI</strong> option on profiles: one
      <a class="ext" href="https://openrouter.ai/keys" target="_blank" rel="noreferrer">OpenRouter</a>
      call fills your variables using the profile's current values and the descriptions above.
    </p>
    <div class="llmgrid">
      <label class="fld">
        <span>OpenRouter API key</span>
        <input class="field" type="password" bind:value={llmKey} placeholder="sk-or-…" />
      </label>
      <label class="fld">
        <span>Model</span>
        <input class="field" bind:value={llmModel} placeholder="google/gemini-2.5-flash" />
      </label>
    </div>
    <label class="checkrow">
      <input type="checkbox" bind:checked={llmWeb} />
      <span>Web research — the model searches the web live (OpenRouter <code>:online</code>)</span>
    </label>
    <div class="row-end">
      <button class="btn" onclick={saveLlm}>Save AI workflow</button>
    </div>
  </section>

  <section class="panel">
    <h4>Scheduled webhooks</h4>
    <p class="hint">
      Automatically POST <strong>all profiles</strong> (or one library item) to a connector on a cadence.
      Runs while Castline is open; anything overdue fires on launch.
    </p>
    {#if connectors.length === 0}
      <p class="hint dim">Add a connector first (Connectors tab) — schedules send to connectors.</p>
    {:else}
      {#each schedules as sch, i (i)}
        <div class="schedrow">
          <select class="field sel" bind:value={sch.kind} onchange={() => (schedDirty = true)}>
            <option value="profiles">All profiles</option>
            <option value="item">One item</option>
          </select>
          {#if sch.kind === "item"}
            <select class="field sel wide" bind:value={sch.item_id} onchange={() => (schedDirty = true)}>
              {#each allItems as it (it.id)}<option value={it.id}>{it.label}</option>{/each}
            </select>
          {/if}
          <span class="arrow">→</span>
          <select class="field sel wide" bind:value={sch.connector_id} onchange={() => (schedDirty = true)}>
            {#each connectors as c (c.id)}<option value={c.id}>{c.name || c.url}</option>{/each}
          </select>
          <select class="field sel" bind:value={sch.every} onchange={() => (schedDirty = true)}>
            <option value="day">Every day</option>
            <option value="week">Every week</option>
            <option value="month">Every month</option>
          </select>
          <span class="lastrun" title="Last sent">{lastRunLabel(sch.last_run)}</span>
          <button class="ghost sm" disabled={!sch.id || schedDirty} title={!sch.id || schedDirty ? "Save schedules first" : "Send now"} onclick={() => runNow(sch)}>Run now</button>
          <button class="icon-btn" title="Delete schedule" onclick={() => removeSchedule(i)}><Icon name="trash" size={14} /></button>
        </div>
      {/each}
      <div class="sched-actions">
        <button class="ghost" onclick={addSchedule}><Icon name="plus" size={14} /> Add schedule</button>
        {#if schedules.length || schedDirty}
          <button class="btn" onclick={saveSchedules}>Save schedules</button>
        {/if}
      </div>
    {/if}
  </section>

  <section class="panel about">
    <p>
      <strong>Note: </strong> Your data lives in portable JSON files on this machine.
    </p>
  </section>
</div>

<style>
  .view {
    height: 100%;
    overflow-y: auto;
    padding: 22px 26px;
  }
  .view-head h2 {
    margin: 0 0 14px;
    font-size: 20px;
    font-weight: 700;
  }
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--edge);
    padding: 16px;
    margin-bottom: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h4 {
    margin: 0;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }
  .loc {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .path {
    font-family: var(--font-mono);
    font-size: 12px;
    background: var(--well);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 7px 9px;
    flex: 1;
    min-width: 0;
    overflow-x: auto;
    white-space: nowrap;
    color: var(--text);
  }
  .grid2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 13px;
  }
  .row-btns {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .about p {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
    line-height: 1.6;
    max-width: 60ch;
  }

  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
    line-height: 1.55;
    max-width: 82ch;
  }
  .hint.dim {
    color: var(--faint);
  }
  .hint code,
  .checkrow code {
    font-family: var(--font-mono);
    font-size: 12px;
    background: var(--well);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 4px;
  }
  .ext {
    color: var(--accent-strong);
  }
  .varlist {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .varrow {
    display: grid;
    grid-template-columns: 180px 1fr;
    align-items: center;
    gap: 10px;
  }
  .varrow .vname {
    font-family: var(--font-mono);
    font-size: 12.5px;
    color: var(--accent-strong);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-end {
    display: flex;
    justify-content: flex-end;
  }
  .llmgrid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .fld {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
  }
  .checkrow {
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 13px;
    color: var(--muted);
    cursor: pointer;
    user-select: none;
  }
  .schedrow {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--well);
    padding: 9px 10px;
  }
  .sel {
    width: auto;
    cursor: pointer;
  }
  .sel.wide {
    max-width: 220px;
    flex: 1;
    min-width: 140px;
  }
  .arrow {
    color: var(--faint);
  }
  .lastrun {
    margin-left: auto;
    color: var(--faint);
    font-size: 11.5px;
    font-family: var(--font-mono);
  }
  .ghost.sm {
    padding: 6px 10px;
    font-size: 12px;
  }
  .sched-actions {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
</style>
