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
  } from "./api.js";
  import Icon from "./Icon.svelte";

  let { flash, onLibraryData, onProfilesData } = $props();
  let dataDir = $state("");

  onMount(async () => {
    dataDir = await getDataDir();
  });

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
    max-width: 760px;
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
</style>
