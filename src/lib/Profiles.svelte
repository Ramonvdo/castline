<script>
  import { allLibraryVars } from "./vars.js";
  import {
    profilesSave,
    profilesDelete,
    profilesSetLayout,
    profileFromJson,
    connectorSend,
  } from "./api.js";
  import Icon from "./Icon.svelte";

  // props — `layout` is the GLOBAL variable grouping (splitters + ordering),
  // shared by every profile. Presentation-only: never affects `values`, so
  // webhook / n8n / Make mappings keyed on variable names keep working.
  let {
    profiles = [],
    layout = [],
    folders = [],
    connectors = [],
    flash,
    onData,
  } = $props();

  let editingId = $state(null); // null = list, "" = new, <id> = editing
  let name = $state("");
  let slots = $state([]);
  let valueMap = $state({});

  let showPaste = $state(false);
  let pasteText = $state("");

  let sid = 0;
  const nextId = () => `s${++sid}`;

  function buildSlots(profileValues) {
    const known = new Set();
    const out = [];
    for (const e of layout) {
      if (e.type === "splitter")
        out.push({ _id: nextId(), type: "splitter", label: e.label || "" });
      else if (e.type === "var" && !known.has(e.name)) {
        out.push({ _id: "v:" + e.name, type: "var", name: e.name });
        known.add(e.name);
      }
    }
    for (const n of allLibraryVars(folders))
      if (!known.has(n)) {
        out.push({ _id: "v:" + n, type: "var", name: n });
        known.add(n);
      }
    for (const n of Object.keys(profileValues || {}))
      if (!known.has(n)) {
        out.push({ _id: "v:" + n, type: "var", name: n });
        known.add(n);
      }
    return out;
  }
  function seedValues(sl, base) {
    const vm = { ...base };
    for (const s of sl)
      if (s.type === "var" && !(s.name in vm)) vm[s.name] = "";
    return vm;
  }

  function newProfile() {
    editingId = "";
    name = "";
    slots = buildSlots({});
    valueMap = seedValues(slots, {});
  }
  function editProfile(p) {
    editingId = p.id;
    name = p.name;
    slots = buildSlots(p.values);
    valueMap = seedValues(slots, p.values);
  }
  function backToList() {
    editingId = null;
  }

  function toLayout(sl) {
    return sl.map((s) =>
      s.type === "splitter"
        ? { type: "splitter", label: s.label, name: "" }
        : { type: "var", label: "", name: s.name },
    );
  }
  async function persistLayout() {
    const data = await profilesSetLayout(toLayout(slots));
    onData(data);
  }

  function addSplitter() {
    slots = [...slots, { _id: nextId(), type: "splitter", label: "New group" }];
    persistLayout();
  }
  function addVariable() {
    const n = prompt("Variable name (no braces), e.g. firstName:");
    if (!n || !n.trim()) return;
    const nm = n.trim();
    if (!slots.some((s) => s.type === "var" && s.name === nm))
      slots = [...slots, { _id: "v:" + nm, type: "var", name: nm }];
    if (!(nm in valueMap)) valueMap = { ...valueMap, [nm]: "" };
    persistLayout();
  }
  function removeSlot(i) {
    const s = slots[i];
    slots = slots.filter((_, idx) => idx !== i);
    if (s.type === "var") {
      const v = { ...valueMap };
      delete v[s.name];
      valueMap = v;
    }
    persistLayout();
  }
  function setLabel(i, label) {
    slots[i] = { ...slots[i], label };
    slots = [...slots];
  }

  let dragIndex = $state(-1);
  let overIndex = $state(-1);
  function onDrop(i) {
    if (dragIndex >= 0 && dragIndex !== i) {
      const next = [...slots];
      const [moved] = next.splice(dragIndex, 1);
      next.splice(dragIndex < i ? i - 1 : i, 0, moved);
      slots = next;
      persistLayout();
    }
    dragIndex = -1;
    overIndex = -1;
  }

  async function save() {
    const nm = name.trim();
    if (!nm) {
      flash("Profile name is required");
      return;
    }
    const values = {};
    for (const s of slots) {
      if (s.type !== "var") continue;
      const val = valueMap[s.name];
      if (val !== undefined && val !== "") values[s.name] = val;
    }
    await persistLayout();
    const data = await profilesSave({
      id: editingId || "",
      name: nm,
      values,
      source: "manual",
    });
    onData(data);
    editingId = null;
    flash("Profile saved");
  }
  async function remove(p) {
    if (!confirm(`Delete profile “${p.name}”?`)) return;
    const data = await profilesDelete(p.id);
    onData(data);
    if (editingId === p.id) editingId = null;
  }
  async function createFromPaste() {
    if (!pasteText.trim()) {
      flash("Paste some JSON first");
      return;
    }
    try {
      const data = await profileFromJson(pasteText);
      onData(data);
      pasteText = "";
      showPaste = false;
      flash("Profile created from JSON");
    } catch (e) {
      flash(String(e));
    }
  }

  // ── Outbound connectors: enrich a profile / create from a connector ──
  let showConnector = $state(false);
  let connId = $state("");
  let connSeed = $state("");
  let connBusy = $state(false);
  let enrichForId = $state(null);
  let enrichBusy = $state(false);

  function parseObj(body) {
    try {
      const o = JSON.parse(body);
      return o && typeof o === "object" && !Array.isArray(o) ? o : null;
    } catch {
      return null;
    }
  }
  function strval(v) {
    return typeof v === "object" && v !== null ? JSON.stringify(v) : String(v);
  }

  function openConnectorPanel() {
    showConnector = !showConnector;
    if (showConnector && !connId && connectors.length)
      connId = connectors[0].id;
  }
  async function newFromConnector() {
    const c = connectors.find((x) => x.id === connId) || connectors[0];
    if (!c) {
      flash("Add a connector first (Connectors tab)");
      return;
    }
    let seedObj = {};
    if (connSeed.trim()) {
      try {
        seedObj = JSON.parse(connSeed);
      } catch {
        flash('Seed must be valid JSON, e.g. { "email": "sam@x.com" }');
        return;
      }
    }
    connBusy = true;
    try {
      const res = await connectorSend(c.url, JSON.stringify(seedObj));
      const obj = parseObj(res.body);
      if (!obj) {
        flash(
          `Connector returned no JSON to build a profile (status ${res.status})`,
        );
      } else {
        const data = await profileFromJson(JSON.stringify(obj));
        onData(data);
        showConnector = false;
        connSeed = "";
        flash("Profile created from connector");
      }
    } catch (e) {
      flash(String(e));
    }
    connBusy = false;
  }
  async function enrich(p, c) {
    enrichForId = null;
    enrichBusy = true;
    try {
      const res = await connectorSend(c.url, JSON.stringify(p.values));
      const obj = parseObj(res.body);
      if (!obj) {
        flash(`Connector returned no JSON (status ${res.status})`);
      } else {
        const merged = { ...p.values };
        for (const [k, v] of Object.entries(obj)) merged[k] = strval(v);
        const data = await profilesSave({
          id: p.id,
          name: p.name,
          values: merged,
          source: p.source || "manual",
        });
        onData(data);
        flash(`Enriched “${p.name}” (+${Object.keys(obj).length} fields)`);
      }
    } catch (e) {
      flash(String(e));
    }
    enrichBusy = false;
  }
</script>

<div class="view">
  {#if editingId === null}
    <div class="view-head">
      <h2></h2>
      <div class="head-actions">
        {#if connectors.length}<button
            class="ghost"
            onclick={openConnectorPanel}>New from connector</button
          >{/if}
        <button class="ghost" onclick={() => (showPaste = !showPaste)}
          >Paste JSON…</button
        >
        <button class="btn" onclick={newProfile}
          ><Icon name="plus" size={14} /> New profile</button
        >
      </div>
    </div>

    {#if showPaste}
      <div class="panel paste">
        <label
          >Create from pasted JSON
          <textarea
            class="field"
            rows="5"
            placeholder={'{ "first_name": "Sam", "email": "sam@example.com" }'}
            bind:value={pasteText}
          ></textarea>
        </label>
        <div class="row-end">
          <button class="ghost" onclick={() => (showPaste = false)}
            >Cancel</button
          >
          <button class="btn" onclick={createFromPaste}>Create profile</button>
        </div>
      </div>
    {/if}

    {#if showConnector}
      <div class="panel paste">
        <label
          >Connector
          <select class="field" bind:value={connId}>
            {#each connectors as c (c.id)}<option value={c.id}
                >{c.name || c.url}</option
              >{/each}
          </select>
        </label>
        <label
          >Seed to send (optional JSON)
          <textarea
            class="field"
            rows="3"
            placeholder={'{ "email": "sam@example.com" }'}
            bind:value={connSeed}
          ></textarea>
        </label>
        <p class="tiny">
          Castline POSTs this to the connector and builds a profile from the
          JSON it returns.
        </p>
        <div class="row-end">
          <button class="ghost" onclick={() => (showConnector = false)}
            >Cancel</button
          >
          <button class="btn" onclick={newFromConnector} disabled={connBusy}
            >{connBusy ? "Running…" : "Run"}</button
          >
        </div>
      </div>
    {/if}

    {#if profiles.length === 0}
      <p class="empty">No profiles yet.</p>
    {:else}
      <ul class="plist">
        {#each profiles as p (p.id)}
          <li class="prow">
            <span class="pname">{p.name}</span>
            {#if p.source && p.source !== "manual"}<span class="srcbadge"
                >{p.source}</span
              >{/if}
            <span class="muted">{Object.keys(p.values).length} value(s)</span>
            {#if connectors.length}
              <div class="enrich-wrap">
                <button
                  class="link"
                  disabled={enrichBusy}
                  onclick={() =>
                    (enrichForId = enrichForId === p.id ? null : p.id)}
                  >Enrich ▾</button
                >
                {#if enrichForId === p.id}
                  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
                  <div
                    class="backdrop"
                    onclick={() => (enrichForId = null)}
                  ></div>
                  <div class="enrich-menu">
                    {#each connectors as c (c.id)}
                      <button class="emi" onclick={() => enrich(p, c)}
                        >{c.name || c.url}</button
                      >
                    {/each}
                  </div>
                {/if}
              </div>
            {/if}
            <button class="link" onclick={() => editProfile(p)}>Edit</button>
            <button class="link danger" onclick={() => remove(p)}>Delete</button
            >
          </li>
        {/each}
      </ul>
    {/if}
  {:else}
    <div class="view-head">
      <button class="ghost" onclick={backToList}
        ><Icon name="arrowLeft" size={15} /> Profiles</button
      >
      <button class="btn" onclick={save}>Save profile</button>
    </div>

    <div class="panel">
      <label class="wlabel"
        >Profile name<input
          class="field"
          bind:value={name}
          placeholder="e.g. Client ACME"
        /></label
      >

      <div class="editor">
        {#each slots as s, i (s._id)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="slot {s.type}"
            class:over={i === overIndex}
            draggable="true"
            ondragstart={() => (dragIndex = i)}
            ondragover={(e) => {
              e.preventDefault();
              overIndex = i;
            }}
            ondrop={() => onDrop(i)}
            ondragend={() => {
              dragIndex = -1;
              overIndex = -1;
            }}
          >
            <span class="grip" title="Drag to reorder"
              ><Icon name="grip" size={16} fill={true} /></span
            >
            {#if s.type === "splitter"}
              <input
                class="slabel"
                value={s.label}
                oninput={(e) => setLabel(i, e.target.value)}
                onchange={persistLayout}
                placeholder="Group name"
              />
              <span class="sline"></span>
            {:else}
              <span class="vname" title={s.name}>{s.name}</span>
              <input
                class="field vval"
                bind:value={valueMap[s.name]}
                placeholder="value"
              />
            {/if}
            <button
              class="icon-btn rm"
              title="Remove"
              onclick={() => removeSlot(i)}
              ><Icon name="close" size={14} /></button
            >
          </div>
        {/each}
      </div>

      <div class="editor-actions">
        <button class="ghost" onclick={addSplitter}
          ><Icon name="divider" size={15} /> Add splitter</button
        >
        <button class="ghost" onclick={addVariable}
          ><Icon name="plus" size={14} /> Add variable</button
        >
      </div>
    </div>
  {/if}
</div>

<style>
  .view {
    height: 100%;
    overflow-y: auto;
    padding: 22px 26px;
    max-width: 860px;
  }
  .view-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .view-head h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .head-actions {
    display: flex;
    gap: 8px;
  }
  .sub {
    color: var(--muted);
    font-size: 13px;
    margin: 6px 0 16px;
  }
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--edge);
    padding: 16px;
    margin-top: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .paste {
    margin-top: 12px;
  }
  .panel label,
  .wlabel {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
  }
  .row-end {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .empty {
    color: var(--muted);
    font-size: 14px;
    margin-top: 12px;
  }
  .plist {
    list-style: none;
    margin: 14px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .plist li {
    display: flex;
    align-items: center;
    gap: 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    padding: 11px 13px;
  }
  .prow {
    position: relative;
  }
  .enrich-wrap {
    position: relative;
  }
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
  }
  .enrich-menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 41;
    min-width: 180px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-modal), var(--edge);
    padding: 5px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .emi {
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
  }
  .emi:hover {
    background: var(--elevated);
  }
  .tiny {
    font-size: 12px;
    color: var(--faint);
    margin: 0;
  }
  .pname {
    font-weight: 600;
    flex: 1;
  }
  .srcbadge {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--accent-strong);
    border: 1px solid color-mix(in srgb, var(--accent) 40%, var(--border));
    border-radius: 5px;
    padding: 1px 6px;
  }
  .muted {
    color: var(--muted);
    font-size: 12px;
  }
  .link {
    background: none;
    border: none;
    color: var(--accent-strong);
    cursor: pointer;
    font-size: 13px;
    padding: 0;
  }
  .link.danger {
    color: #d98a8a;
  }

  .editor {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .slot {
    display: flex;
    align-items: center;
    gap: 8px;
    border-radius: var(--radius-sm);
    padding: 2px;
  }
  .slot.over {
    box-shadow: inset 0 2px 0 var(--accent);
  }
  .grip {
    display: flex;
    color: var(--faint);
    cursor: grab;
    flex-shrink: 0;
  }
  .grip:active {
    cursor: grabbing;
  }
  .slot.var .vname {
    width: 160px;
    flex-shrink: 0;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--accent-strong);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .vval {
    flex: 1;
  }
  .slot.splitter {
    margin-top: 6px;
  }
  .slabel {
    border: none;
    background: none;
    color: var(--text);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    padding: 2px 0;
    width: 150px;
    flex-shrink: 0;
  }
  .slabel:focus {
    outline: none;
    color: var(--accent-strong);
  }
  .slabel::placeholder {
    color: var(--faint);
  }
  .sline {
    flex: 1;
    height: 1px;
    background: var(--border);
  }
  .rm {
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 0.12s var(--ease);
  }
  .slot:hover .rm {
    opacity: 1;
  }
  .editor-actions {
    display: flex;
    gap: 8px;
  }
</style>
