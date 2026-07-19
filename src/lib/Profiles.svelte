<script>
  import { onDestroy } from "svelte";
  import { allLibraryVars } from "./vars.js";
  import {
    profilesSave,
    profilesDelete,
    profilesSetLayout,
    profileFromJson,
    connectorSend,
    llmEnrich,
    readTextFile,
    pickContextFile,
  } from "./api.js";
  import Icon from "./Icon.svelte";

  // props — `layout` is the GLOBAL variable grouping (splitters + ordering),
  // shared by every profile. Presentation-only: never affects `values`, so
  // webhook / n8n / Make mappings keyed on variable names keep working.
  // `llm` = the AI-workflow settings (key presence gates Castline AI; its
  // web_search seeds the dialog's per-run toggle); `onAgent(text)` hands an
  // instruction to the Agent tab.
  let {
    profiles = [],
    layout = [],
    folders = [],
    connectors = [],
    llm = { api_key: "", web_search: false },
    flash,
    onData,
    onAgent = () => {},
  } = $props();

  let llmReady = $derived(!!(llm && llm.api_key));

  let editingId = $state(null); // null = list, "" = new, <id> = editing
  let name = $state("");
  let pTone = $state(""); // per-profile tone-of-voice override
  let slots = $state([]);
  let valueMap = $state({});
  // Locked-empty variables: always empty, filled on the spot, never enriched.
  let pLocked = $state([]);
  function toggleLock(varName) {
    if (pLocked.includes(varName)) {
      pLocked = pLocked.filter((n) => n !== varName);
    } else {
      pLocked = [...pLocked, varName];
      valueMap = { ...valueMap, [varName]: "" };
    }
  }

  // ── Long-value hover: a floating, editable preview beside the cursor ──
  const LONG_VALUE = 60;
  let hoverVar = $state(null); // { name, x, y }
  let hoverTimer;
  function varEnter(e, varName) {
    if ((valueMap[varName] || "").length <= LONG_VALUE) return;
    clearTimeout(hoverTimer);
    const x = Math.min(e.clientX + 18, window.innerWidth - 400);
    const y = Math.max(10, Math.min(e.clientY - 30, window.innerHeight - 280));
    hoverVar = { name: varName, x, y };
  }
  function varLeave() {
    clearTimeout(hoverTimer);
    hoverTimer = setTimeout(() => (hoverVar = null), 250);
  }
  function hoverKeep() {
    clearTimeout(hoverTimer);
  }

  let showPaste = $state(false);
  let pasteText = $state("");

  let sid = 0;
  const nextId = () => `s${++sid}`;

  function buildSlots(profileValues) {
    // Only variables that are live: used somewhere in the library right now, or
    // already holding a value in this profile. Stale layout entries (vars whose
    // prompts were deleted) keep their stored ordering but don't render.
    const live = new Set([
      ...allLibraryVars(folders),
      ...Object.keys(profileValues || {}),
    ]);
    const known = new Set();
    const out = [];
    for (const e of layout) {
      if (e.type === "splitter")
        out.push({ _id: nextId(), type: "splitter", label: e.label || "" });
      else if (e.type === "var" && live.has(e.name) && !known.has(e.name)) {
        out.push({ _id: "v:" + e.name, type: "var", name: e.name });
        known.add(e.name);
      }
    }
    for (const n of live)
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
    pTone = "";
    pLocked = [];
    slots = buildSlots({});
    valueMap = seedValues(slots, {});
  }
  function editProfile(p) {
    editingId = p.id;
    name = p.name;
    pTone = p.tone || "";
    pLocked = [...(p.locked || [])];
    slots = buildSlots(p.values);
    valueMap = seedValues(slots, p.values);
  }
  async function backToList() {
    hoverVar = null; // never let the hover panel outlive the editor
    addingVar = false;
    // Auto-save on the way out — leaving the editor must never lose edits.
    if (editingId !== null && name.trim()) {
      await save();
      return;
    }
    if (editingId === "" && !name.trim()) flash("Discarded — no profile name");
    editingId = null;
  }

  // Tab switches unmount this view: flush an open editor the same way.
  onDestroy(() => {
    if (editingId !== null && name.trim()) {
      persistLayout().catch(() => {});
      profilesSave({
        id: editingId || "",
        name: name.trim(),
        values: collectValues(),
        source: "manual",
        tone: pTone.trim(),
        locked: pLocked,
      })
        .then(onData)
        .catch(() => {});
    }
  });

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
  // Inline add-variable (no native prompt): a small input in the actions row.
  let addingVar = $state(false);
  let newVarName = $state("");
  function addVariable() {
    const nm = newVarName.trim().replace(/[{}]/g, "");
    if (!nm) {
      addingVar = false;
      return;
    }
    if (!slots.some((s) => s.type === "var" && s.name === nm))
      slots = [...slots, { _id: "v:" + nm, type: "var", name: nm }];
    if (!(nm in valueMap)) valueMap = { ...valueMap, [nm]: "" };
    newVarName = "";
    addingVar = false;
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

  function collectValues() {
    const values = {};
    for (const s of slots) {
      if (s.type !== "var") continue;
      if (pLocked.includes(s.name)) continue; // locked = always empty
      const val = valueMap[s.name];
      if (val !== undefined && val !== "") values[s.name] = val;
    }
    return values;
  }
  async function save() {
    const nm = name.trim();
    if (!nm) {
      flash("Profile name is required");
      return;
    }
    await persistLayout();
    const data = await profilesSave({
      id: editingId || "",
      name: nm,
      values: collectValues(),
      source: "manual",
      tone: pTone.trim(),
      locked: pLocked,
    });
    onData(data);
    editingId = null;
    flash("Profile saved");
  }
  // In-app delete confirmation (no native confirm()).
  let pendingDelete = $state(null); // a profile
  async function confirmRemove() {
    const p = pendingDelete;
    pendingDelete = null;
    if (!p) return;
    const data = await profilesDelete(p.id);
    onData(data);
    if (editingId === p.id) editingId = null;
    flash("Profile deleted");
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
      const res = await connectorSend(
        c.url,
        JSON.stringify(seedObj),
        "New profile from connector",
      );
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
  // Freshly AI-generated values this session: profileId -> [varNames]. Shown
  // green in the editor (and as a badge in the list) until the user edits —
  // i.e. vets — each one.
  let aiFresh = $state({});
  function clearAiMark(varName) {
    const id = editingId;
    if (!id || !(aiFresh[id] || []).includes(varName)) return;
    aiFresh = { ...aiFresh, [id]: aiFresh[id].filter((n) => n !== varName) };
  }

  // Merge an enrichment result (object of name → value) into a profile.
  // Locked-empty variables are never written by any enrich path.
  async function mergeEnriched(p, obj, label, markAi = false) {
    const locked = p.locked || [];
    const merged = { ...p.values };
    const written = [];
    for (const [k, v] of Object.entries(obj)) {
      if (locked.includes(k)) continue;
      merged[k] = strval(v);
      written.push(k);
    }
    const n = written.length;
    if (markAi) aiFresh = { ...aiFresh, [p.id]: written };
    const data = await profilesSave({
      id: p.id,
      name: p.name,
      values: merged,
      source: p.source || "manual",
      tone: p.tone || "",
      locked,
    });
    onData(data);
    flash(`Enriched “${p.name}” (+${n} fields${label ? ` · ${label}` : ""})`);
  }

  async function enrich(p, c) {
    enrichForId = null;
    enrichBusy = true;
    try {
      const res = await connectorSend(
        c.url,
        JSON.stringify(p.values),
        `Enrich · ${p.name}`,
      );
      const obj = parseObj(res.body);
      if (!obj) {
        flash(`Connector returned no JSON (status ${res.status})`);
      } else {
        await mergeEnriched(p, obj, c.name || "webhook");
      }
    } catch (e) {
      flash(String(e));
    }
    enrichBusy = false;
  }

  // "Castline AI": a small dialog first — extra context, an optional .txt/.md
  // attachment, and a per-run web-research toggle — then one OpenRouter call.
  let aiPanel = $state(null); // { profile }
  let aiContext = $state("");
  let aiFile = $state(null); // { name, text }
  let aiWeb = $state(false);
  let aiTone = $state(false); // opt-in: apply the tone of voice
  let aiLib = $state(false); // opt-in: give the model the library templates

  function openAiPanel(p) {
    enrichForId = null;
    aiPanel = { profile: p };
    aiContext = "";
    aiFile = null;
    // Auto-checked: without web research the model invents company facts.
    // Only an explicit Settings opt-out starts it unticked.
    aiWeb = llm?.web_search !== false;
    // Off by default — with nothing ticked the generation stays simple
    // (values + variable descriptions only).
    aiTone = false;
    aiLib = false;
  }

  // What tone WOULD apply if the checkbox is ticked (profile → Settings).
  let aiToneText = $derived(
    (aiPanel?.profile?.tone || "").trim() || (llm?.tone || "").trim(),
  );
  async function attachAiFile() {
    const path = await pickContextFile();
    if (!path) return;
    try {
      const text = await readTextFile(path);
      aiFile = { name: path.split(/[\\/]/).pop(), text };
    } catch (e) {
      flash(String(e));
    }
  }
  async function runAiEnrich() {
    const p = aiPanel?.profile;
    if (!p) return;
    enrichBusy = true;
    try {
      const ctx = [
        aiContext.trim(),
        aiFile ? `--- Attached file: ${aiFile.name} ---\n${aiFile.text}` : "",
      ]
        .filter(Boolean)
        .join("\n\n");
      const body = await llmEnrich(
        JSON.stringify(p.values),
        ctx,
        aiWeb,
        p.tone || "",
        aiTone,
        aiLib,
      );
      const obj = parseObj(body);
      if (!obj || !Object.keys(obj).length) {
        flash("The AI returned no fields");
      } else {
        await mergeEnriched(p, obj, "Castline AI", true);
        aiPanel = null;
      }
    } catch (e) {
      flash(String(e));
    }
    enrichBusy = false;
  }

  // Hand the profile to the Agent tab with a ready-made instruction.
  function enrichViaAgent(p) {
    enrichForId = null;
    onAgent(
      `Enrich the Castline profile "${p.name}": research the missing variables (see the Variables section in CLAUDE.md) and update it via the local update-profile endpoint. Current values: ${JSON.stringify(p.values)}`,
    );
  }

  // ── Send all profiles to a connector (one payload) ──
  let sendAllOpen = $state(false);
  async function sendAll(c) {
    sendAllOpen = false;
    const payload = {
      profiles: profiles.map((p) => ({ name: p.name, values: p.values })),
    };
    try {
      const res = await connectorSend(
        c.url,
        JSON.stringify(payload),
        `Send all profiles (${profiles.length})`,
      );
      flash(
        res.status >= 200 && res.status < 300
          ? `Sent ${profiles.length} profile${profiles.length === 1 ? "" : "s"} → ${c.name || "webhook"}`
          : `Webhook answered ${res.status}`,
      );
    } catch (e) {
      flash(String(e));
    }
  }
</script>

<div class="view">
  {#if editingId === null}
    <div class="view-head">
      <h2>Profiles</h2>
      <div class="head-actions">
        {#if connectors.length && profiles.length}
          <div class="enrich-wrap">
            <button class="ghost" onclick={() => (sendAllOpen = !sendAllOpen)}
              >Send all ▾</button
            >
            {#if sendAllOpen}
              <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
              <div class="backdrop" onclick={() => (sendAllOpen = false)}></div>
              <div class="enrich-menu">
                <div class="emi-label">POST all profiles to</div>
                {#each connectors as c (c.id)}
                  <button class="emi" onclick={() => sendAll(c)}
                    >{c.name || c.url}</button
                  >
                {/each}
              </div>
            {/if}
          </div>
        {/if}
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
            {#if (aiFresh[p.id] || []).length}
              <span
                class="aibadge"
                title="Fields just generated by Castline AI — open Edit to review the green-highlighted values"
                >✦ {aiFresh[p.id].length} AI</span
              >
            {/if}
            <span class="muted">{Object.keys(p.values).length} value(s)</span>
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
                  <button
                    class="emi ai"
                    disabled={!llmReady}
                    title={llmReady
                      ? "One AI call fills the library's variables"
                      : "Add an OpenRouter key in Settings → AI workflow"}
                    onclick={() => openAiPanel(p)}
                  >
                    <span style="color:#9095d6">Castline AI</span>{llmReady
                      ? ""
                      : " (no API key)"}
                  </button>
                  {#if connectors.length}
                    <div class="emi-label">Webhooks</div>
                    {#each connectors as c (c.id)}
                      <button class="emi" onclick={() => enrich(p, c)}
                        >{c.name || c.url}</button
                      >
                    {/each}
                  {/if}
                  <div class="emi-sep"></div>
                  <button class="emi" onclick={() => enrichViaAgent(p)}>
                    <Icon name="terminal" size={13} /> Ask the Agent…
                  </button>
                </div>
              {/if}
            </div>
            <button class="link" onclick={() => editProfile(p)}>Edit</button>
            <button class="link danger" onclick={() => (pendingDelete = p)}
              >Delete</button
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
      <label class="wlabel"
        >Tone of voice — optional, overrides Settings for this profile's AI text<input
          class="field"
          bind:value={pTone}
          placeholder="e.g. Formal and precise; write in Dutch; never use exclamation marks"
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
                class:lockedv={pLocked.includes(s.name)}
                class:aifresh={editingId &&
                  (aiFresh[editingId] || []).includes(s.name)}
                bind:value={valueMap[s.name]}
                disabled={pLocked.includes(s.name)}
                placeholder={pLocked.includes(s.name)
                  ? "locked — fill on the spot, never enriched"
                  : "value"}
                oninput={() => clearAiMark(s.name)}
                onmouseenter={(e) => varEnter(e, s.name)}
                onmouseleave={varLeave}
                title={editingId &&
                (aiFresh[editingId] || []).includes(s.name)
                  ? "Just generated by Castline AI — check it; editing clears the highlight"
                  : undefined}
              />
              <button
                class="icon-btn lockbtn"
                class:on={pLocked.includes(s.name)}
                title={pLocked.includes(s.name)
                  ? "Locked empty — this variable must be filled on the spot and no enrich can write it. Click to unlock."
                  : "Lock empty — always fill this on the spot; AI/webhook enrich will never touch it"}
                onclick={() => toggleLock(s.name)}
                ><Icon name="lock" size={14} /></button
              >
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
        {#if addingVar}
          <div class="addvar">
            <!-- svelte-ignore a11y_autofocus -->
            <input
              class="field avname"
              bind:value={newVarName}
              placeholder="variableName (no braces)"
              autofocus
              onkeydown={(e) => {
                if (e.key === "Enter") addVariable();
                else if (e.key === "Escape") {
                  addingVar = false;
                  newVarName = "";
                }
              }}
            />
            <button class="btn sm" onclick={addVariable}>Add</button>
            <button
              class="ghost sm"
              onclick={() => {
                addingVar = false;
                newVarName = "";
              }}>Cancel</button
            >
          </div>
        {:else}
          <button class="ghost" onclick={() => (addingVar = true)}
            ><Icon name="plus" size={14} /> Add variable</button
          >
        {/if}
      </div>
    </div>
  {/if}
</div>

{#if pendingDelete}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="ai-overlay"
    onclick={(e) => e.target === e.currentTarget && (pendingDelete = null)}
  >
    <div class="ai-modal confirm">
      <div class="ai-head"><h3>Delete profile</h3></div>
      <p class="ai-sub">
        Are you sure you want to delete <strong>“{pendingDelete.name}”</strong>?
        Its
        {Object.keys(pendingDelete.values || {}).length} value(s) will be gone. This
        can't be undone.
      </p>
      <div class="ai-actions">
        <button class="ghost" onclick={() => (pendingDelete = null)}
          >Cancel</button
        >
        <button class="ghost danger" onclick={confirmRemove}
          ><Icon name="trash" size={14} /> Delete</button
        >
      </div>
    </div>
  </div>
{/if}

{#if hoverVar}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="hoverpanel"
    style:left="{hoverVar.x}px"
    style:top="{hoverVar.y}px"
    onmouseenter={hoverKeep}
    onmouseleave={varLeave}
  >
    <span class="hp-name">{hoverVar.name}</span>
    <textarea
      class="field hp-text"
      rows="9"
      bind:value={valueMap[hoverVar.name]}
      oninput={() => clearAiMark(hoverVar.name)}
    ></textarea>
    <span class="hp-hint">Edits apply live — remember to Save profile.</span>
  </div>
{/if}

{#if aiPanel}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="ai-overlay"
    onclick={(e) =>
      e.target === e.currentTarget && !enrichBusy && (aiPanel = null)}
  >
    <div class="ai-modal">
      <div class="ai-head">
        <h3>
          <Icon name="sparkle" size={16} /> Enrich “{aiPanel.profile.name}” with
          Castline AI
        </h3>
        <button
          class="icon-btn"
          title="Close"
          disabled={enrichBusy}
          onclick={() => (aiPanel = null)}
          ><Icon name="close" size={16} /></button
        >
      </div>
      <p class="ai-sub">
        One AI call fills the library's variables from the profile's current
        values, your variable descriptions, and any extra information you add
        below.
      </p>

      <label class="ai-fld">
        <span>Extra context (optional)</span>
        <textarea
          class="field"
          rows="4"
          bind:value={aiContext}
          placeholder="Anything you know: notes from a call, a LinkedIn blurb, the company's about-page text…"
        ></textarea>
      </label>

      <div class="ai-row">
        {#if aiFile}
          <span class="ai-file" title="Attached file">
            <Icon name="template" size={13} />
            {aiFile.name}
            <span class="ai-size">{Math.ceil(aiFile.text.length / 1000)} k</span
            >
            <button
              class="icon-btn xs"
              title="Remove file"
              onclick={() => (aiFile = null)}
              ><Icon name="close" size={12} /></button
            >
          </span>
        {:else}
          <button class="ghost sm" onclick={attachAiFile}
            ><Icon name="plus" size={13} /> Attach a .txt / .md file</button
          >
        {/if}
      </div>

      <!-- Everything below is opt-in: with nothing ticked the generation stays
           simple — profile values + variable descriptions only. -->
      <div class="ai-opts">
        <label class="ai-web" title="OpenRouter :online — works with any model">
          <input type="checkbox" bind:checked={aiWeb} />
          <span>Web research</span>
        </label>
        <label
          class="ai-web"
          class:off={!aiToneText}
          title={aiToneText
            ? `Applies: ${aiToneText.slice(0, 140)}${aiToneText.length > 140 ? "…" : ""}`
            : "No tone configured — set one in Settings → AI workflow or on this profile"}
        >
          <input type="checkbox" bind:checked={aiTone} disabled={!aiToneText} />
          <span>Tone of voice</span>
        </label>
        <label
          class="ai-web"
          title="Adds the templates where your variables are used, so generated text fits the sentence around it"
        >
          <input type="checkbox" bind:checked={aiLib} />
          <span>Use library as reference</span>
        </label>
      </div>

      <div class="ai-actions">
        <button
          class="ghost"
          disabled={enrichBusy}
          onclick={() => (aiPanel = null)}>Cancel</button
        >
        <button class="btn" disabled={enrichBusy} onclick={runAiEnrich}>
          {enrichBusy ? "Researching…" : "Enrich profile"}
        </button>
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
  .emi {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .emi.ai {
    color: var(--accent-strong);
    font-weight: 600;
  }
  .emi:disabled {
    color: var(--faint);
    font-weight: 400;
    cursor: default;
  }
  .emi:disabled:hover {
    background: none;
  }
  .emi-label {
    font-size: 10.5px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--faint);
    padding: 6px 9px 2px;
  }
  .emi-sep {
    height: 1px;
    background: var(--border);
    margin: 4px 2px;
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
  .vval.lockedv {
    opacity: 0.55;
    border-style: dashed;
  }
  /* A value the AI just wrote — green until you touch (= vet) it. */
  .vval.aifresh {
    border-color: color-mix(in srgb, #6fb894 55%, var(--border));
    background: color-mix(in srgb, #6fb894 9%, var(--well));
  }
  .aibadge {
    font-size: 10px;
    font-weight: 600;
    color: #86c7a4;
    border: 1px solid rgba(111, 184, 148, 0.4);
    border-radius: 5px;
    padding: 1px 6px;
    white-space: nowrap;
  }
  .lockbtn {
    flex-shrink: 0;
    color: var(--faint);
  }
  .lockbtn.on {
    color: var(--accent-strong);
    background: var(--accent-soft);
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
    align-items: center;
    flex-wrap: wrap;
  }
  .addvar {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .avname {
    width: 240px;
    font-family: var(--font-mono);
    font-size: 12.5px;
  }
  .btn.sm,
  .ghost.sm {
    padding: 7px 12px;
    font-size: 12px;
  }
  .ai-modal.confirm {
    width: min(420px, 100%);
  }
  .ghost.danger {
    color: #d98a8a;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  /* ── Long-value hover editor ── */
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
    min-height: 120px;
  }
  .hp-hint {
    font-size: 11px;
    color: var(--faint);
  }

  /* ── Castline AI enrich dialog ── */
  .ai-overlay {
    position: fixed;
    inset: 0;
    z-index: 70;
    background: rgba(4, 7, 13, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }
  .ai-modal {
    width: min(560px, 100%);
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-modal), var(--edge);
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .ai-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .ai-head h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 700;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--accent-strong);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ai-sub {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.55;
  }
  .ai-fld {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
  }
  .ai-fld textarea {
    resize: vertical;
    font-family: inherit;
  }
  .ai-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
  }
  .ai-file {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 12.5px;
    font-family: var(--font-mono);
    color: var(--text);
    background: var(--well);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 6px 9px;
    max-width: 320px;
    overflow: hidden;
  }
  .ai-size {
    color: var(--faint);
    font-size: 11px;
  }
  .ghost.sm {
    padding: 6px 10px;
    font-size: 12px;
  }
  .icon-btn.xs {
    padding: 3px;
  }
  .ai-opts {
    display: flex;
    align-items: center;
    gap: 18px;
    flex-wrap: wrap;
    border-top: 1px solid var(--border);
    padding-top: 11px;
  }
  .ai-web {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 13px;
    color: var(--muted);
    cursor: pointer;
    user-select: none;
  }
  .ai-web.off {
    color: var(--faint);
    cursor: default;
  }
  .ai-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
</style>
