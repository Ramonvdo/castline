<script>
  import { onMount } from "svelte";
  import {
    setAccent,
    applyAccent,
    getDataDir,
    revealDataDir,
    pickSaveFile,
    pickOpenFile,
    exportLibraryTo,
    importLibraryFrom,
    exportProfilesTo,
    importProfilesFrom,
    setWebhookConfig,
    webhookStatus,
    clipCopy,
  } from "./api.js";
  import Icon from "./Icon.svelte";

  // props
  let { settings, flash, onLibraryData, onProfilesData, onSettings, onClose } = $props();

  let accent = $state(settings.accent || "#4f8cff");
  let dataDir = $state("");
  let activePort = $state(null);

  // editable copy of the webhook config
  let wh = $state({
    enabled: false,
    port: 8787,
    token: "",
    name_template: "{{first_name}} {{last_name}}",
    mappings: [],
    passthrough: true,
    ...structuredClone($state.snapshot(settings.webhook || {})),
  });

  onMount(async () => {
    dataDir = await getDataDir();
    activePort = await webhookStatus();
  });

  // ── Appearance ──
  async function pickAccent(e) {
    accent = e.target.value;
    applyAccent(accent);
    const s = await setAccent(accent);
    onSettings(s);
  }

  // ── Data location ──
  async function reveal() {
    try {
      await revealDataDir();
    } catch (e) {
      flash(String(e));
    }
  }

  // ── Import / export ──
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
    if (mode === "replace" && !confirm("Replace your entire library with this file? This cannot be undone.")) return;
    try {
      const data = await importLibraryFrom(path, mode);
      onLibraryData(data);
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
    if (mode === "replace" && !confirm("Replace all profiles with this file?")) return;
    try {
      const data = await importProfilesFrom(path, mode);
      onProfilesData(data);
      flash(mode === "replace" ? "Profiles replaced" : "Profiles merged");
    } catch (e) {
      flash(String(e));
    }
  }

  // ── Webhook ──
  function addMapping() {
    wh.mappings = [...wh.mappings, { from: "", to: "" }];
  }
  function removeMapping(i) {
    wh.mappings = wh.mappings.filter((_, idx) => idx !== i);
  }
  async function saveWebhook() {
    const cfg = {
      enabled: !!wh.enabled,
      port: Number(wh.port) || 8787,
      token: wh.token || "",
      name_template: wh.name_template || "",
      mappings: wh.mappings.filter((m) => m.from.trim()),
      passthrough: !!wh.passthrough,
    };
    const s = await setWebhookConfig(cfg);
    wh = { ...wh, ...structuredClone($state.snapshot(s.webhook)) };
    onSettings(s);
    activePort = await webhookStatus();
    flash(cfg.enabled ? "Webhook receiver running" : "Webhook receiver stopped");
  }
  let endpoint = $derived(
    wh.enabled && wh.token ? `http://127.0.0.1:${wh.port}/hook?token=${wh.token}` : "",
  );
  async function copyEndpoint() {
    if (endpoint) {
      await clipCopy(endpoint);
      flash("Endpoint URL copied");
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="overlay" onclick={(e) => e.target === e.currentTarget && onClose()}>
  <div class="modal wide">
    <h3>Settings</h3>

    <!-- Appearance -->
    <section>
      <h4>Appearance</h4>
      <div class="line">
        <span>Accent colour</span>
        <input class="color" type="color" value={accent} onchange={pickAccent} />
        <span class="muted">{accent}</span>
      </div>
    </section>

    <!-- Data -->
    <section>
      <h4>Data & backups</h4>
      <div class="line wrap">
        <code class="path">{dataDir || "…"}</code>
        <button class="ghost" onclick={reveal}><Icon name="folderOpen" size={15} /> Open folder</button>
      </div>
      <div class="grid2">
        <div class="stack">
          <strong>Library</strong>
          <div class="row-btns">
            <button class="ghost" onclick={exportLibrary}>Export</button>
            <button class="ghost" onclick={() => importLibrary("merge")}>Import (merge)</button>
            <button class="ghost" onclick={() => importLibrary("replace")}>Import (replace)</button>
          </div>
        </div>
        <div class="stack">
          <strong>Profiles</strong>
          <div class="row-btns">
            <button class="ghost" onclick={exportProfiles}>Export</button>
            <button class="ghost" onclick={() => importProfiles("merge")}>Import (merge)</button>
            <button class="ghost" onclick={() => importProfiles("replace")}>Import (replace)</button>
          </div>
        </div>
      </div>
    </section>

    <!-- Webhook -->
    <section>
      <h4>Incoming webhook</h4>

      <label class="toggle">
        <input type="checkbox" bind:checked={wh.enabled} />
        Enable local receiver
      </label>

      <div class="line">
        <label class="inline">Port<input class="field port" type="number" bind:value={wh.port} min="1024" max="65535" /></label>
        <label class="inline grow">Name template<input class="field" bind:value={wh.name_template} placeholder="{'{{first_name}} {{last_name}}'}" /></label>
      </div>

      <label class="toggle">
        <input type="checkbox" bind:checked={wh.passthrough} />
        Pass unmapped fields through as variables of the same name
      </label>

      <div class="maps">
        <span class="maps-head">Field mapping — incoming JSON key → <code>{"{{variable}}"}</code></span>
        {#each wh.mappings as m, i (i)}
          <div class="map-row">
            <input class="field" bind:value={m.from} placeholder="first_name" />
            <span class="arrow"><Icon name="arrowRight" size={15} /></span>
            <input class="field" bind:value={m.to} placeholder="firstName" />
            <button class="icon-btn" title="Remove" onclick={() => removeMapping(i)}><Icon name="close" size={14} /></button>
          </div>
        {/each}
        <button class="ghost" onclick={addMapping}><Icon name="plus" size={14} /> Add mapping</button>
      </div>

      {#if endpoint}
        <div class="endpoint">
          <span class="dot" class:live={activePort}></span>
          <code class="url">{endpoint}</code>
          <button class="ghost" onclick={copyEndpoint}>Copy URL</button>
        </div>
        <p class="hint">{activePort ? `Listening on 127.0.0.1:${activePort}.` : "Save to start the receiver."}</p>
      {/if}

      <div class="modal-actions">
        <button class="btn" onclick={saveWebhook}>Save & apply</button>
      </div>
    </section>

    <div class="modal-actions">
      <button class="ghost" onclick={onClose}>Close</button>
    </div>
  </div>
</div>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
    border-top: 1px solid var(--border);
    padding-top: 14px;
  }
  section:first-of-type {
    border-top: none;
    padding-top: 0;
  }
  h4 {
    margin: 0;
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .line.wrap {
    flex-wrap: wrap;
  }
  .color {
    width: 40px;
    height: 28px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: none;
    cursor: pointer;
  }
  .muted {
    color: var(--muted);
    font-size: 12px;
  }
  .path,
  .url {
    font-family: ui-monospace, monospace;
    font-size: 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 9px;
    flex: 1;
    min-width: 0;
    overflow-x: auto;
    white-space: nowrap;
    color: var(--text);
  }
  .grid2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: 7px;
    font-size: 13px;
  }
  .row-btns {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text);
    cursor: pointer;
  }
  .inline {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
    color: var(--muted);
  }
  .inline.grow {
    flex: 1;
  }
  .port {
    width: 100px;
  }
  .maps {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .maps-head {
    font-size: 12px;
    color: var(--muted);
  }
  .map-row {
    display: grid;
    grid-template-columns: 1fr auto 1fr auto;
    align-items: center;
    gap: 8px;
  }
  .arrow {
    color: var(--muted);
  }
  .endpoint {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--muted);
    flex-shrink: 0;
  }
  .dot.live {
    background: #46b980;
    box-shadow: 0 0 0 3px color-mix(in srgb, #46b980 22%, transparent);
  }
</style>
