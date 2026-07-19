<script>
  import { onMount, onDestroy } from "svelte";
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
    setAutostart,
    pickDirectory,
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
  let lockedVars = $state([]); // GLOBAL locked-empty variables

  // Long-description hover: a floating, editable preview beside the cursor
  // (same pattern as the profile editor's long-value panel).
  const LONG_DESC = 60;
  let hoverVar = $state(null); // { name, x, y }
  let hoverTimer;
  function descEnter(e, varName) {
    if ((descs[varName] || "").length <= LONG_DESC) return;
    clearTimeout(hoverTimer);
    const x = Math.min(e.clientX + 18, window.innerWidth - 400);
    const y = Math.max(10, Math.min(e.clientY - 30, window.innerHeight - 280));
    hoverVar = { name: varName, x, y };
  }
  function descLeave() {
    clearTimeout(hoverTimer);
    hoverTimer = setTimeout(() => (hoverVar = null), 250);
  }
  function hoverKeep() {
    clearTimeout(hoverTimer);
  }

  // ── Auto-save: every change persists on its own (debounced) — no Save
  // buttons. A small "Saved" tick confirms each write. ──
  let savedNote = $state("");
  let savedTimer;
  function noteSaved(which) {
    savedNote = which;
    clearTimeout(savedTimer);
    savedTimer = setTimeout(() => (savedNote = ""), 1600);
  }

  // Reactive: folders can arrive after mount (App loads the library async).
  let varNames = $derived.by(() => {
    const names = allLibraryVars(folders);
    for (const k of Object.keys(storedDescs))
      if (!names.includes(k)) names.push(k);
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
  let llmTone = $state("");

  // ── Startup & tray ──
  let autostart = $state(true);
  async function toggleAutostart() {
    const s = await setAutostart(!autostart);
    autostart = !!s.autostart;
    onSettings(s);
    flash(autostart ? "Castline starts with Windows" : "Autostart off");
  }

  // ── Scheduled jobs ──
  let schedules = $state([]);
  let schedDirty = $state(false);
  let schedTimer;
  function schedChanged() {
    schedDirty = true;
    clearTimeout(schedTimer);
    schedTimer = setTimeout(saveSchedules, 700);
  }

  async function browseDir(sch) {
    const dir = await pickDirectory();
    if (dir) {
      sch.dir = dir;
      schedChanged();
    }
  }

  // Every item across the library, for the "send one item" schedule picker.
  let allItems = $derived.by(() => {
    const out = [];
    for (const f of folders || [])
      for (const i of f.items || [])
        out.push({ id: i.id, label: `${i.name} · ${f.name}` });
    return out;
  });

  onMount(async () => {
    dataDir = await getDataDir();
    const s = await getSettings();
    autostart = s.autostart !== false;
    llmKey = s.llm?.api_key || "";
    llmModel = s.llm?.model || "google/gemini-2.5-flash";
    llmWeb = !!s.llm?.web_search;
    llmTone = s.llm?.tone || "";
    schedules = (s.schedules || []).map((x) => ({ ...x }));

    const p = await getProfiles();
    storedDescs = p.descriptions || {};
    // Backfill stored descriptions into rows already seeded with "".
    const d = { ...descs };
    for (const [k, v] of Object.entries(storedDescs)) if (!d[k]) d[k] = v;
    descs = d;
    // Globally locked-empty variables — shown as a padlock so it's clear the
    // value stays empty everywhere while the description still guides AI
    // enrichment of the other variables.
    lockedVars = p.locked || [];
  });

  let descDirty = false;
  let descTimer;
  function descChanged() {
    descDirty = true;
    clearTimeout(descTimer);
    descTimer = setTimeout(saveDescriptions, 800);
  }
  async function saveDescriptions() {
    clearTimeout(descTimer);
    descDirty = false;
    const data = await profilesSetDescriptions({ ...descs });
    onProfilesData(data);
    noteSaved("desc");
  }

  let llmDirty = false;
  let llmTimer;
  function llmChanged() {
    llmDirty = true;
    clearTimeout(llmTimer);
    llmTimer = setTimeout(saveLlm, 800);
  }
  async function saveLlm() {
    clearTimeout(llmTimer);
    llmDirty = false;
    const s = await setLlmConfig(llmKey, llmModel, llmWeb, llmTone);
    onSettings(s);
    noteSaved("llm");
  }

  // Leaving the tab flushes anything still pending, so nothing is ever lost.
  onDestroy(() => {
    clearTimeout(descTimer);
    clearTimeout(llmTimer);
    clearTimeout(schedTimer);
    if (descDirty)
      profilesSetDescriptions({ ...descs })
        .then(onProfilesData)
        .catch(() => {});
    if (llmDirty)
      setLlmConfig(llmKey, llmModel, llmWeb, llmTone)
        .then(onSettings)
        .catch(() => {});
    if (schedDirty)
      setSchedules(schedules.map((x) => ({ ...x })))
        .then(onSettings)
        .catch(() => {});
  });

  function addSchedule() {
    schedules = [
      ...schedules,
      {
        id: "",
        kind: "profiles",
        item_id: allItems[0]?.id || "",
        folder_id: folders[0]?.id || "",
        dir: "",
        connector_id: connectors[0]?.id || "",
        every: "week",
        last_run: 0,
        catch_up: false,
      },
    ];
    schedChanged();
  }
  function removeSchedule(i) {
    schedules = schedules.filter((_, idx) => idx !== i);
    schedChanged();
  }
  async function saveSchedules() {
    clearTimeout(schedTimer);
    const s = await setSchedules(schedules.map((x) => ({ ...x })));
    onSettings(s);
    schedules = (s.schedules || []).map((x) => ({ ...x }));
    schedDirty = false;
    noteSaved("sched");
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
  // In-app confirm for destructive imports (no native confirm()).
  let pendingConfirm = $state(null); // { text, run }

  async function importLibrary(mode) {
    const path = await pickOpenFile();
    if (!path) return;
    const run = async () => {
      try {
        onLibraryData(await importLibraryFrom(path, mode));
        flash(mode === "replace" ? "Library replaced" : "Library merged");
      } catch (e) {
        flash(String(e));
      }
    };
    if (mode === "replace") {
      pendingConfirm = {
        text: "Replace your entire library with this file? This cannot be undone.",
        run,
      };
    } else {
      await run();
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
    const run = async () => {
      try {
        onProfilesData(await importProfilesFrom(path, mode));
        flash(mode === "replace" ? "Profiles replaced" : "Profiles merged");
      } catch (e) {
        flash(String(e));
      }
    };
    if (mode === "replace") {
      pendingConfirm = {
        text: "Replace all profiles with this file? This cannot be undone.",
        run,
      };
    } else {
      await run();
    }
  }
</script>

<div class="view">
  <section class="panel">
    <h4>Data Backups</h4>
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
    <label class="checkrow">
      <input type="checkbox" checked={autostart} onchange={toggleAutostart} />
      <span><strong>Enable auto-start on PC startup</strong></span>
    </label>
  </section>

  <section class="panel">
    <h4>
      <code>Profle {"{{variable}}"} Configuration</code>
      <span class="saved" class:show={savedNote === "desc"}>✓ Saved</span>
    </h4>
    <p class="hint">
      Describe what each <code>{"{{variable}}"}</code> must look like for a more
      accurate enrichment via <strong>Castline AI</strong>. Changes save
      automatically.
    </p>
    {#if varNames.length === 0}
      <p class="hint dim">
        No variables in the library yet — add {"{{placeholders}}"} to your templates.
      </p>
    {:else}
      <div class="varlist">
        {#each varNames as n (n)}
          <div class="varrow">
            <span class="vcell">
              <code class="vname">{n}</code>
              {#if lockedVars.includes(n)}
                <span
                  class="vlock"
                  title="Locked empty in every profile — always filled on the spot, no enrich can write it. The description still guides the AI for the other variables."
                  ><Icon name="lock" size={12} /></span
                >
              {/if}
            </span>
            <input
              class="field"
              bind:value={descs[n]}
              oninput={descChanged}
              onmouseenter={(e) => descEnter(e, n)}
              onmouseleave={descLeave}
              placeholder={n === "companyName"
                ? `e.g. simplified lowercase company name ("Acme Studios LLC" becomes "acme")`
                : "What should this value contain? Format, casing, an example…"}
            />
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <section class="panel">
    <h4>
      Castline AI enrichment
      <span class="saved" class:show={savedNote === "llm"}>✓ Saved</span>
    </h4>
    <p class="hint">
      <a
        class="ext"
        href="https://openrouter.ai/keys"
        target="_blank"
        rel="noreferrer">OpenRouter</a
      > is used to enrich your variables using the profile's current values and descriptions.
      Changes save automatically.
    </p>
    <div class="llmgrid">
      <label class="fld">
        <span>OpenRouter API key</span>
        <input
          class="field"
          type="password"
          bind:value={llmKey}
          oninput={llmChanged}
          placeholder="sk-or-…"
        />
      </label>
      <label class="fld">
        <span>Model</span>
        <input
          class="field"
          bind:value={llmModel}
          oninput={llmChanged}
          placeholder="google/gemini-2.5-flash"
        />
      </label>
    </div>
    <label class="checkrow">
      <input type="checkbox" bind:checked={llmWeb} onchange={llmChanged} />
      <span
        >Web research (OpenRouter <code>:online</code>, works with any model)</span
      >
    </label>
    <label class="fld">
      <span>Tone of voice (optional)</span>
      <textarea
        class="field tone"
        rows="2"
        bind:value={llmTone}
        oninput={llmChanged}
        placeholder="Empty = no tone is applied at all"
      ></textarea>
    </label>
  </section>

  <section class="panel">
    <h4>
      Scheduled jobs
      <span class="saved" class:show={savedNote === "sched"}>✓ Saved</span>
    </h4>
    <p class="hint">
      Missed runs are <strong>skipped</strong> unless a job opts into
      <em>Catch up</em>, which fires it once. Changes save automatically.
    </p>
    {#each schedules as sch, i (i)}
      <div class="schedrow">
        <select
          class="field sel"
          bind:value={sch.kind}
          onchange={schedChanged}
        >
          <option value="profiles">All profiles</option>
          <option value="item">One item</option>
          <option value="folder">One folder</option>
          <option value="backup">Backup data</option>
        </select>
        {#if sch.kind === "item"}
          {#if allItems.length}
            <select
              class="field sel wide"
              bind:value={sch.item_id}
              onchange={schedChanged}
            >
              {#each allItems as it (it.id)}<option value={it.id}
                  >{it.label}</option
                >{/each}
            </select>
          {:else}
            <span class="hint dim">No items in the library yet</span>
          {/if}
        {:else if sch.kind === "folder"}
          <select
            class="field sel wide"
            bind:value={sch.folder_id}
            onchange={schedChanged}
          >
            {#each folders as f (f.id)}<option value={f.id}>{f.name}</option
              >{/each}
          </select>
        {/if}
        <span class="arrow">→</span>
        {#if sch.kind === "backup"}
          <input
            class="field sel wide"
            bind:value={sch.dir}
            placeholder="Backup folder…"
            onchange={schedChanged}
          />
          <button class="ghost sm" onclick={() => browseDir(sch)}
            >Browse…</button
          >
        {:else if connectors.length}
          <select
            class="field sel wide"
            bind:value={sch.connector_id}
            onchange={schedChanged}
          >
            {#each connectors as c (c.id)}<option value={c.id}
                >{c.name || c.url}</option
              >{/each}
          </select>
        {:else}
          <span class="hint dim">Add a connector first (Connectors tab)</span>
        {/if}
        <select
          class="field sel"
          bind:value={sch.every}
          onchange={schedChanged}
        >
          <option value="day">Every day</option>
          <option value="week">Every week</option>
          <option value="month">Every month</option>
        </select>
        <label
          class="catchup"
          title="If a run was missed while the app was closed, fire it once at launch (off = skip missed runs)"
        >
          <input
            type="checkbox"
            bind:checked={sch.catch_up}
            onchange={schedChanged}
          />
          <span>Catch up</span>
        </label>
        <span class="lastrun" title="Last run"
          >{lastRunLabel(sch.last_run)}</span
        >
        <button
          class="ghost sm"
          disabled={!sch.id || schedDirty}
          title={!sch.id || schedDirty
            ? "Waiting for auto-save…"
            : "Run now"}
          onclick={() => runNow(sch)}>Run now</button
        >
        <button
          class="icon-btn"
          title="Delete schedule"
          onclick={() => removeSchedule(i)}
          ><Icon name="trash" size={14} /></button
        >
      </div>
    {/each}
    <div class="sched-actions">
      <button class="ghost" onclick={addSchedule}
        ><Icon name="plus" size={14} /> Add schedule</button
      >
    </div>
  </section>

  <section class="panel about">
    <p>
      <strong>Note: </strong> Your data lives in portable JSON files on this machine.
    </p>
  </section>
</div>

{#if hoverVar}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="hoverpanel"
    style:left="{hoverVar.x}px"
    style:top="{hoverVar.y}px"
    onmouseenter={hoverKeep}
    onmouseleave={descLeave}
  >
    <span class="hp-name">{hoverVar.name}</span>
    <textarea
      class="field hp-text"
      rows="8"
      bind:value={descs[hoverVar.name]}
      oninput={descChanged}
    ></textarea>
    <span class="hp-hint">Edits apply live and save automatically.</span>
  </div>
{/if}

{#if pendingConfirm}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="confirm-overlay"
    onclick={(e) => e.target === e.currentTarget && (pendingConfirm = null)}
  >
    <div class="confirm-modal">
      <h3>Are you sure?</h3>
      <p>{pendingConfirm.text}</p>
      <div class="confirm-actions">
        <button class="ghost" onclick={() => (pendingConfirm = null)}
          >Cancel</button
        >
        <button
          class="btn"
          onclick={() => {
            const c = pendingConfirm;
            pendingConfirm = null;
            c.run();
          }}>Yes, replace</button
        >
      </div>
    </div>
  </div>
{/if}

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
  .varrow .vcell {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
  }
  .varrow .vname {
    font-family: var(--font-mono);
    font-size: 12.5px;
    color: var(--accent-strong);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .vlock {
    display: inline-flex;
    flex-shrink: 0;
    color: var(--accent-strong);
    background: var(--accent-soft);
    border-radius: 4px;
    padding: 2px 3px;
    cursor: help;
  }
  /* "✓ Saved" tick beside a panel title, fades in after each auto-save. */
  h4 {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .saved {
    text-transform: none;
    letter-spacing: 0;
    font-size: 11.5px;
    font-weight: 600;
    color: #86c7a4;
    opacity: 0;
    transition: opacity 0.25s var(--ease);
  }
  .saved.show {
    opacity: 1;
  }
  /* Long-description hover editor (same panel as the profile editor's). */
  .hoverpanel {
    position: fixed;
    z-index: 65;
    width: 380px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-modal), var(--edge);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .hp-name {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--accent-strong);
  }
  .hp-text {
    resize: vertical;
    font-family: inherit;
    font-size: 13px;
    line-height: 1.55;
    min-height: 110px;
  }
  .hp-hint {
    font-size: 11px;
    color: var(--faint);
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
  .tone {
    resize: vertical;
    font-family: inherit;
    line-height: 1.5;
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
  .catchup {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
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
  .confirm-overlay {
    position: fixed;
    inset: 0;
    z-index: 70;
    background: rgba(4, 7, 13, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }
  .confirm-modal {
    width: min(420px, 100%);
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-modal), var(--edge);
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .confirm-modal h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 700;
  }
  .confirm-modal p {
    margin: 0;
    color: var(--muted);
    font-size: 13.5px;
    line-height: 1.55;
  }
  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 6px;
  }
</style>
