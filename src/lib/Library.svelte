<script>
  import {
    libCreateFolder,
    libRenameFolder,
    libDeleteFolder,
    libSetFolderColor,
    libSetFolderIcon,
    libReorderFolders,
    libReorderItems,
    libSaveItem,
    libDeleteItem,
    libMoveItem,
    libToggleFavorite,
    libRecordUse,
    clipCopy,
    connectorSend,
    pickSaveDoc,
    saveTextFile,
  } from "./api.js";
  import { extractVars, itemVars, itemPlainText, applyVars } from "./vars.js";
  import Icon from "./Icon.svelte";
  import FolderIcon from "./FolderIcon.svelte";
  import { FOLDER_ICON_NAMES, FOLDER_COLORS } from "./foldericons.js";

  // props (library is bindable so imports/webhook refreshes from the parent flow in)
  let {
    library = $bindable(),
    profiles = [],
    layout = [],
    activeProfile = null,
    compact = false,
    connectors = [],
    flash,
    onFill,
  } = $props();

  const ALL = "__all";
  const FAV = "__fav";

  let activeId = $state(ALL);
  let search = $state("");
  let selectedTags = $state([]);

  let realFolders = $derived(library.folders || []);
  let activeFolder = $derived(
    realFolders.find((f) => f.id === activeId) || null,
  );
  let isVirtual = $derived(activeId === ALL || activeId === FAV);

  // Every item in the current scope, tagged with its folder for context.
  let scopedEntries = $derived.by(() => {
    const out = [];
    for (const f of realFolders) {
      if (!isVirtual && f.id !== activeId) continue;
      for (const i of f.items || []) {
        if (activeId === FAV && !i.favorite) continue;
        out.push({
          item: i,
          folderId: f.id,
          folderName: f.name,
          folderColor: f.color,
          folderIcon: f.icon,
        });
      }
    }
    return out;
  });

  // Tags present in the current scope, for the filter chips.
  let scopeTags = $derived.by(() => {
    const set = new Set();
    for (const e of scopedEntries)
      for (const t of e.item.tags || []) set.add(t);
    return [...set].sort();
  });

  let visible = $derived.by(() => {
    const q = search.trim().toLowerCase();
    let list = scopedEntries;
    if (q) {
      list = list.filter(
        ({ item: i }) =>
          i.name.toLowerCase().includes(q) ||
          (i.text || "").toLowerCase().includes(q) ||
          (i.item_type || "").toLowerCase().includes(q) ||
          (i.steps || []).some(
            (s) =>
              s.title.toLowerCase().includes(q) ||
              s.text.toLowerCase().includes(q),
          ) ||
          (i.tags || []).some((t) => t.toLowerCase().includes(q)),
      );
    }
    if (selectedTags.length) {
      list = list.filter(({ item: i }) =>
        (i.tags || []).some((t) => selectedTags.includes(t)),
      );
    }
    // "Most used" sorts by copy count wherever you are (presentation only).
    if (sortMode === "used") {
      return [...list].sort(
        (a, b) =>
          (b.item.uses || 0) - (a.item.uses || 0) ||
          a.item.name.localeCompare(b.item.name),
      );
    }
    // A real folder keeps its manual (stored) order so drag-reorder is honoured;
    // the All / Favorites views sort favourites-first then by name.
    if (isVirtual) {
      return [...list].sort(
        (a, b) =>
          Number(b.item.favorite) - Number(a.item.favorite) ||
          a.item.name.localeCompare(b.item.name),
      );
    }
    return list;
  });

  // "manual" honours stored/drag order; "used" surfaces the most-copied items.
  let sortMode = $state("manual");

  function toggleTag(t) {
    selectedTags = selectedTags.includes(t)
      ? selectedTags.filter((x) => x !== t)
      : [...selectedTags, t];
  }

  // ── Folder create/edit modal (name + icon + colour) ──
  let folderModalOpen = $state(false);
  let fmMode = $state("new"); // "new" | "edit"
  let fmId = $state(null);
  let fmName = $state("");
  let fmIcon = $state("folder");
  let fmColor = $state("");
  let fmConfirmDelete = $state(false);
  let fmColorInput;

  function openNewFolder() {
    fmMode = "new";
    fmId = null;
    fmName = "";
    fmIcon = "folder";
    fmColor = "";
    fmConfirmDelete = false;
    folderModalOpen = true;
  }
  function openEditFolder() {
    if (!activeFolder) return;
    openEditFolderById(activeFolder.id);
  }
  // Edit any folder by id (used by the folder rail's right-click menu). Pass
  // { confirmDelete:true } to open straight on the delete confirmation.
  function openEditFolderById(id, opts = {}) {
    const f = library.folders.find((x) => x.id === id);
    if (!f) return;
    fmMode = "edit";
    fmId = f.id;
    fmName = f.name;
    fmIcon = f.icon || "folder";
    fmColor = f.color || "";
    fmConfirmDelete = !!opts.confirmDelete;
    folderModalOpen = true;
  }

  // Right-click context menu on a rail folder.
  let folderMenu = $state(null); // { id, x, y }
  function openFolderMenu(e, id) {
    e.preventDefault();
    folderMenu = { id, x: e.clientX, y: e.clientY };
  }
  function menuAction(fn) {
    const id = folderMenu?.id;
    folderMenu = null;
    if (id) fn(id);
  }
  async function saveFolder() {
    const name = fmName.trim();
    if (!name) {
      flash("Folder name is required");
      return;
    }
    if (fmMode === "new") {
      let lib = await libCreateFolder(name);
      const newId = lib.folders[lib.folders.length - 1].id;
      lib = await libSetFolderIcon(newId, fmIcon);
      if (fmColor) lib = await libSetFolderColor(newId, fmColor);
      library = lib;
      activeId = newId;
    } else {
      let lib = await libRenameFolder(fmId, name);
      lib = await libSetFolderIcon(fmId, fmIcon);
      lib = await libSetFolderColor(fmId, fmColor);
      library = lib;
    }
    folderModalOpen = false;
    flash(fmMode === "new" ? "Folder created" : "Folder saved");
  }
  async function deleteFolderConfirmed() {
    if (!fmId) return;
    library = await libDeleteFolder(fmId);
    if (activeId === fmId) activeId = ALL;
    folderModalOpen = false;
  }

  // ── Drag & drop (folders reorder · items move/reorder) ──
  let drag = $state(null); // { kind:'folder'|'item', id, fromFolderId }
  let dropFolderId = $state(null); // rail folder highlighted as a drop target
  let dropOnId = $state(null); // card highlighted (insertion)
  let dropAfter = $state(false);

  function clearDrag() {
    drag = null;
    dropFolderId = null;
    dropOnId = null;
  }
  async function dropOnFolder(targetId) {
    const d = drag;
    dropFolderId = null;
    if (!d) return;
    if (d.kind === "folder" && d.id !== targetId) {
      const ids = realFolders.map((f) => f.id);
      const from = ids.indexOf(d.id);
      const to = ids.indexOf(targetId);
      if (from !== -1 && to !== -1) {
        ids.splice(from, 1);
        ids.splice(to, 0, d.id);
        library = await libReorderFolders(ids);
      }
    } else if (d.kind === "item" && d.fromFolderId !== targetId) {
      library = await libMoveItem(d.fromFolderId, targetId, d.id);
    }
    clearDrag();
  }
  async function dropOnCard(targetItemId, targetFolderId) {
    const d = drag;
    const after = dropAfter;
    dropOnId = null;
    if (!d || d.kind !== "item") {
      clearDrag();
      return;
    }
    // Cross-folder drop onto a card → move into that folder (appended).
    if (d.fromFolderId !== targetFolderId) {
      library = await libMoveItem(d.fromFolderId, targetFolderId, d.id);
      clearDrag();
      return;
    }
    // Same folder → reorder relative to the target card.
    const folder = realFolders.find((f) => f.id === targetFolderId);
    if (!folder) {
      clearDrag();
      return;
    }
    const ids = folder.items.map((i) => i.id);
    const from = ids.indexOf(d.id);
    if (from !== -1) ids.splice(from, 1);
    let to = ids.indexOf(targetItemId);
    if (to === -1) to = ids.length;
    else if (after) to += 1;
    ids.splice(to, 0, d.id);
    library = await libReorderItems(targetFolderId, ids);
    clearDrag();
  }
  function cardDragOver(e, itemId) {
    if (!drag || drag.kind !== "item") return;
    e.preventDefault();
    const r = e.currentTarget.getBoundingClientRect();
    dropAfter = e.clientX > r.left + r.width / 2;
    dropOnId = itemId;
  }

  // ── Item editor ──
  let editorOpen = $state(false);
  let editingId = $state(null);
  let editingFolderId = $state(null);
  let fName = $state("");
  let fTags = $state("");
  let fKind = $state("template");
  let fText = $state("");
  let fSteps = $state([]);
  let fFolderId = $state(null);

  let editorVars = $derived(
    fKind === "sop"
      ? (() => {
          const all = [];
          for (const s of fSteps)
            for (const v of extractVars(s.text))
              if (!all.includes(v)) all.push(v);
          return all;
        })()
      : extractVars(fText),
  );

  function defaultFolderId() {
    return activeFolder ? activeFolder.id : realFolders[0]?.id || null;
  }

  function openNewItem() {
    if (!realFolders.length) {
      flash("Create a folder first");
      return;
    }
    editingId = null;
    editingFolderId = defaultFolderId();
    fFolderId = defaultFolderId();
    fName = "";
    fTags = "";
    fKind = "template";
    fText = "";
    fSteps = [{ id: "", title: "Step 1", text: "" }];
    editorOpen = true;
  }
  function openEditItem(folderId, item) {
    editingId = item.id;
    editingFolderId = folderId;
    fFolderId = folderId;
    fName = item.name;
    fTags = (item.tags || []).join(", ");
    fKind = item.kind === "sop" ? "sop" : "template";
    fText = item.text || "";
    fSteps = (item.steps || []).map((s) => ({ ...s }));
    if (fKind === "sop" && fSteps.length === 0)
      fSteps = [{ id: "", title: "Step 1", text: "" }];
    editorOpen = true;
  }
  function addStep() {
    fSteps = [
      ...fSteps,
      { id: "", title: `Step ${fSteps.length + 1}`, text: "" },
    ];
  }
  function removeStep(i) {
    fSteps = fSteps.filter((_, idx) => idx !== i);
  }
  function moveStep(i, dir) {
    const j = i + dir;
    if (j < 0 || j >= fSteps.length) return;
    const next = [...fSteps];
    [next[i], next[j]] = [next[j], next[i]];
    fSteps = next;
  }
  function setKind(sop) {
    fKind = sop ? "sop" : "template";
    if (sop && fSteps.length === 0)
      fSteps = [{ id: "", title: "Step 1", text: "" }];
  }
  async function saveItem() {
    const name = fName.trim();
    if (!name) {
      flash("Name is required");
      return;
    }
    const tags = fTags
      .split(",")
      .map((t) => t.trim())
      .filter(Boolean);
    const item = {
      id: editingId || "",
      name,
      kind: fKind,
      type: "",
      text: fKind === "template" ? fText : "",
      steps:
        fKind === "sop"
          ? fSteps.map((s) => ({
              id: s.id || "",
              title: s.title,
              text: s.text,
            }))
          : [],
      tags,
      favorite: false,
      created_at: "",
      updated_at: "",
    };
    // Preserve favorite when editing existing.
    if (editingId) {
      const existing = realFolders
        .find((f) => f.id === editingFolderId)
        ?.items.find((i) => i.id === editingId);
      if (existing) item.favorite = existing.favorite;
    }
    if (editingId && fFolderId !== editingFolderId) {
      await libMoveItem(editingFolderId, fFolderId, editingId);
    }
    library = await libSaveItem(fFolderId, item);
    editorOpen = false;
    flash(editingId ? "Saved" : "Added");
  }
  // ── Item right-click menu + always-confirm delete ──
  let itemMenu = $state(null); // { folderId, item, x, y }
  let pendingDelete = $state(null); // { folderId, item }
  function openItemMenu(e, folderId, item) {
    e.preventDefault();
    itemMenu = { folderId, item, x: e.clientX, y: e.clientY };
  }
  function askDeleteItem(folderId, item) {
    itemMenu = null;
    pendingDelete = { folderId, item };
  }
  async function confirmDeleteItem() {
    const d = pendingDelete;
    pendingDelete = null;
    if (!d) return;
    library = await libDeleteItem(d.folderId, d.item.id);
    flash("Item deleted");
  }
  async function toggleFav(folderId, item) {
    library = await libToggleFavorite(folderId, item.id);
  }
  // Copy an item; if a profile is active in the top bar, fill its {{variables}}.
  // {{today}}/{{now}} tokens always resolve, profile or not.
  async function copyItem(item) {
    const raw = itemPlainText(item);
    const text = applyVars(raw, activeProfile?.values || {});
    const ok = await clipCopy(text);
    flash(
      ok
        ? activeProfile
          ? `Copied · ${activeProfile.name}`
          : "Copied"
        : "Copy failed",
    );
    if (ok) library = await libRecordUse(item.id);
  }

  // Duplicate an item into its folder (blank id → upsert creates a copy).
  async function duplicateItem(folderId, item) {
    const copy = JSON.parse(JSON.stringify(item));
    copy.id = "";
    copy.name = `${item.name} (copy)`;
    copy.uses = 0;
    copy.favorite = false;
    for (const s of copy.steps || []) s.id = "";
    library = await libSaveItem(folderId, copy);
    flash("Item duplicated");
  }

  // POST one item to an outbound connector (raw text — mapping lives in Make/n8n).
  async function sendItemTo(item, c) {
    const payload = {
      name: item.name,
      type: item.item_type || "",
      kind: item.kind,
      tags: item.tags || [],
      text: itemPlainText(item),
      steps: (item.steps || []).map((s) => ({ title: s.title, text: s.text })),
    };
    try {
      const res = await connectorSend(c.url, JSON.stringify(payload));
      flash(
        res.status >= 200 && res.status < 300
          ? `Sent “${item.name}” → ${c.name || "webhook"}`
          : `Webhook answered ${res.status}`,
      );
    } catch (e) {
      flash(String(e));
    }
  }

  // ── Multi-select (Ctrl/Cmd+click) — ordered, so selection order == copy order ──
  let selected = $state([]); // item ids, in the order they were picked

  function selIndex(id) {
    return selected.indexOf(id);
  }
  function toggleSelect(id) {
    selected = selected.includes(id)
      ? selected.filter((x) => x !== id)
      : [...selected, id];
  }
  function cardClick(e, item) {
    // Ctrl/Cmd + click toggles multi-select (drag handles move/reorder; the Copy
    // button copies). A plain click does nothing.
    if (e.target.closest && e.target.closest("button")) return;
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      toggleSelect(item.id);
    }
  }
  function clearSelection() {
    selected = [];
  }

  // Resolve a selected id to its item + folder, preserving selection order.
  let selectedEntries = $derived.by(() => {
    const out = [];
    for (const id of selected) {
      for (const f of realFolders) {
        const it = f.items.find((i) => i.id === id);
        if (it) {
          out.push({ item: it, folderName: f.name });
          break;
        }
      }
    }
    return out;
  });

  // One combined document: each item as a "## Name" section, in picked order.
  // SOP steps are expanded so a custom SOP reads as a clean sequence.
  function combinedText() {
    return selectedEntries
      .map(({ item }) => {
        if (item.kind === "sop") {
          const steps = (item.steps || [])
            .map((s) => `### ${s.title}\n\n${s.text}`)
            .join("\n\n");
          return `## ${item.name}\n\n${steps}`;
        }
        return `## ${item.name}\n\n${item.text || ""}`;
      })
      .join("\n\n---\n\n");
  }

  async function copyCombined() {
    if (!selected.length) return;
    const ok = await clipCopy(combinedText());
    flash(
      ok
        ? `Copied ${selected.length} item${selected.length === 1 ? "" : "s"}`
        : "Copy failed",
    );
  }

  async function exportSelectedMd() {
    if (!selected.length) return;
    const path = await pickSaveDoc("custom-sop.md");
    if (!path) return;
    try {
      await saveTextFile(path, combinedText());
      flash("Exported to file");
    } catch (e) {
      flash(String(e));
    }
  }

  // Turn the picked items into one new SOP (great for assembling a client SOP).
  function newSopFromSelection() {
    if (!selected.length) return;
    const steps = [];
    for (const { item } of selectedEntries) {
      if (item.kind === "sop") {
        for (const s of item.steps || [])
          steps.push({ id: "", title: s.title, text: s.text });
      } else {
        steps.push({ id: "", title: item.name, text: item.text || "" });
      }
    }
    editingId = null;
    editingFolderId = defaultFolderId();
    fFolderId = defaultFolderId();
    fName = "";
    fTags = "";
    fKind = "sop";
    fText = "";
    fSteps = steps.length ? steps : [{ id: "", title: "Step 1", text: "" }];
    editorOpen = true;
  }

  let totalItems = $derived(
    realFolders.reduce((n, f) => n + f.items.length, 0),
  );
  let favCount = $derived(
    realFolders.reduce(
      (n, f) => n + f.items.filter((i) => i.favorite).length,
      0,
    ),
  );
</script>

<div class="layout">
  <!-- Folder rail -->
  <aside class="rail">
    <div class="virt-row">
      <button
        class="virt"
        class:active={activeId === ALL}
        onclick={() => (activeId = ALL)}
      >
        <Icon name="layers" size={15} /><span class="vl">All</span><span
          class="vc">{totalItems}</span
        >
      </button>
      <button
        class="virt"
        class:active={activeId === FAV}
        onclick={() => (activeId = FAV)}
      >
        <Icon name="star" size={15} fill={activeId === FAV} /><span class="vl"
          >Pinned</span
        ><span class="vc">{favCount}</span>
      </button>
    </div>

    <ul class="folders">
      {#each realFolders as f (f.id)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <li
          class="folder-li"
          class:drop={dropFolderId === f.id}
          class:ghost={drag?.kind === "folder" && drag.id === f.id}
          draggable="true"
          ondragstart={(e) => {
            drag = { kind: "folder", id: f.id };
            e.dataTransfer.effectAllowed = "move";
          }}
          ondragend={clearDrag}
          ondragover={(e) => {
            if (drag) {
              e.preventDefault();
              dropFolderId = f.id;
            }
          }}
          ondragleave={() => {
            if (dropFolderId === f.id) dropFolderId = null;
          }}
          ondrop={() => dropOnFolder(f.id)}
          oncontextmenu={(e) => openFolderMenu(e, f.id)}
        >
          <button
            class="folder"
            class:active={f.id === activeId}
            onclick={() => (activeId = f.id)}
          >
            <span class="ficon"
              ><FolderIcon
                name={f.icon || "folder"}
                color={f.color || "var(--muted)"}
                size={16}
              /></span
            >
            <span class="fname">{f.name}</span>
            <span class="count">{f.items.length}</span>
          </button>
        </li>
      {/each}
    </ul>
    <button class="newfolder" onclick={openNewFolder}
      ><Icon name="plus" size={15} /> New folder</button
    >
  </aside>

  <!-- Main -->
  <section class="main">
    <div class="toolbar">
      <div class="search-wrap">
        <span class="search-ic"><Icon name="search" size={16} /></span>
        <input
          class="search"
          placeholder="Search {isVirtual
            ? 'all items'
            : activeFolder?.name || ''}…"
          bind:value={search}
        />
      </div>
      <select class="sortsel" bind:value={sortMode} title="Sort items">
        <option value="manual">Manual order</option>
        <option value="used">Most used</option>
      </select>
      <button class="btn with-ic" onclick={openNewItem}
        ><Icon name="plus" size={15} /> New item</button
      >
    </div>

    {#if !isVirtual && activeFolder}
      <div class="folder-bar">
        <span class="fb-icon"
          ><FolderIcon
            name={activeFolder.icon || "folder"}
            color={activeFolder.color || "var(--muted)"}
            size={18}
          /></span
        >
        <strong>{activeFolder.name}</strong>
        <span class="fcount"
          >{activeFolder.items.length} item{activeFolder.items.length === 1
            ? ""
            : "s"}</span
        >
        <button
          class="icon-btn edit-folder"
          title="Edit folder"
          onclick={openEditFolder}><Icon name="edit" size={15} /></button
        >
      </div>
    {/if}

    {#if scopeTags.length}
      <div class="tagbar">
        {#each scopeTags as t}
          <button
            class="chip"
            class:on={selectedTags.includes(t)}
            onclick={() => toggleTag(t)}>{t}</button
          >
        {/each}
        {#if selectedTags.length}<button
            class="chip clear"
            onclick={() => (selectedTags = [])}>clear</button
          >{/if}
      </div>
    {/if}

    {#if !realFolders.length}
      <p class="empty">No folders yet.</p>
    {:else if visible.length === 0}
      <p class="empty">
        No items{search || selectedTags.length ? " match" : " yet"}.
      </p>
    {:else}
      <div class="grid">
        {#each visible as { item, folderId, folderName, folderColor } (item.id)}
          {@const vars = itemVars(item)}
          {@const pos = selIndex(item.id)}
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
          <article
            class="card"
            class:fav={item.favorite}
            class:sel={pos >= 0}
            class:compact
            class:ghost={drag?.kind === "item" && drag.id === item.id}
            class:drop-before={dropOnId === item.id && !dropAfter}
            class:drop-after={dropOnId === item.id && dropAfter}
            style:--fcolor={folderColor || "var(--border)"}
            draggable="true"
            ondragstart={(e) => {
              drag = { kind: "item", id: item.id, fromFolderId: folderId };
              e.dataTransfer.effectAllowed = "move";
            }}
            ondragend={clearDrag}
            ondragover={(e) => cardDragOver(e, item.id)}
            ondragleave={() => {
              if (dropOnId === item.id) dropOnId = null;
            }}
            ondrop={() => dropOnCard(item.id, folderId)}
            onclick={(e) => cardClick(e, item)}
            oncontextmenu={(e) => openItemMenu(e, folderId, item)}
          >
            {#if pos >= 0}<span class="selnum">{pos + 1}</span>{/if}

            <!-- Corner: pin + how often this item has been copied. -->
            <span
              class="type-corner"
              title={item.uses
                ? `Copied ${item.uses} time${item.uses === 1 ? "" : "s"}`
                : "Not copied yet"}
            >
              {#if item.favorite}<span class="fav-mark"
                  ><Icon name="star" size={12} fill={true} /></span
                >{/if}
              {#if item.uses}<span class="tc-n uses"
                  ><Icon name="copy" size={11} />{item.uses}</span
                >{/if}
            </span>

            <!-- Hover actions sit on a dark corner-piece so they don't clash with the type marker. -->
            <div class="hover-actions">
              <button
                class="star"
                class:on={item.favorite}
                title={item.favorite ? "Unpin" : "Pin"}
                onclick={() => toggleFav(folderId, item)}
              >
                <Icon name="star" size={15} fill={item.favorite} />
              </button>
              <button
                class="icon-btn xs"
                title="Edit"
                onclick={() => openEditItem(folderId, item)}
                ><Icon name="edit" size={14} /></button
              >
              <button
                class="icon-btn xs"
                title="Delete"
                onclick={() => askDeleteItem(folderId, item)}
                ><Icon name="trash" size={14} /></button
              >
            </div>

            <div class="name" title={item.name}>
              {item.name}
              {#if item.kind === "sop"}<span
                  class="sop-badge"
                  title={`SOP · ${item.steps.length} steps`}
                  ><Icon name="sop" size={11} />{item.steps.length}</span
                >{/if}
            </div>

            {#if !compact}
              {#if isVirtual}
                <div class="badges">
                  <span class="badge origin"
                    ><span
                      class="odot"
                      style:background={folderColor || "var(--muted)"}
                    ></span>{folderName}</span
                  >
                </div>
              {/if}

              <p class="preview">
                {item.kind === "sop" ? item.steps[0]?.text || "" : item.text}
              </p>

              {#if (item.tags || []).length}
                <div class="tags">
                  {#each item.tags.slice(0, 3) as t}<span class="chip">{t}</span
                    >{/each}
                  {#if item.tags.length > 3}<span class="chip"
                      >+{item.tags.length - 3}</span
                    >{/if}
                </div>
              {/if}
            {/if}

            <footer>
              <button class="act" onclick={() => copyItem(item)}
                ><Icon name="copy" size={13} /> Copy</button
              >
              {#if vars.length || item.kind === "sop"}
                <button
                  class="act primary"
                  onclick={() =>
                    onFill(item, item.kind === "sop" ? "steps" : "auto")}
                >
                  <Icon
                    name={item.kind === "sop" ? "sop" : "sparkle"}
                    size={13}
                  /> Fill &amp; copy
                </button>
              {/if}
            </footer>
          </article>
        {/each}
      </div>
    {/if}

    {#if selected.length}
      <div class="selbar">
        <div class="selinfo">
          <span class="selcount">{selected.length} selected</span>
        </div>
        <div class="selactions">
          <button class="ghost" onclick={clearSelection}>Clear</button>
          <button class="ghost" onclick={exportSelectedMd}
            ><Icon name="reveal" size={14} /> Export .md</button
          >
          <button class="ghost" onclick={newSopFromSelection}
            ><Icon name="sop" size={14} /> New SOP</button
          >
          <button class="btn" onclick={copyCombined}
            ><Icon name="copy" size={14} /> Copy combined</button
          >
        </div>
      </div>
    {/if}
  </section>
</div>

<!-- Item editor (simple: name · text · tags) -->
{#if editorOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="overlay"
    onclick={(e) => e.target === e.currentTarget && (editorOpen = false)}
  >
    <div class="modal">
      <div class="modal-head">
        <h3>{editingId ? "Edit item" : "New item"}</h3>
        <button
          class="icon-btn"
          title="Close"
          onclick={() => (editorOpen = false)}
          ><Icon name="close" size={16} /></button
        >
      </div>

      <label class="fld">
        <span class="fld-label">Name</span>
        <input
          class="field"
          bind:value={fName}
          placeholder="e.g. Cold outreach"
        />
      </label>

      {#if fKind === "template"}
        <label class="fld">
          <span class="fld-label">Prompt text</span>
          <textarea
            class="field"
            rows="9"
            bind:value={fText}
            placeholder="Enter your prompt here…"
          ></textarea>
          <span class="charcount">{(fText || "").length} characters</span>
        </label>
      {:else}
        <div class="fld">
          <span class="fld-label">Steps</span>
          <div class="steps">
            {#each fSteps as step, i (i)}
              <div class="step">
                <div class="step-head">
                  <input
                    class="field step-title"
                    bind:value={step.title}
                    placeholder={`Step ${i + 1} title`}
                  />
                  <div class="step-ctl">
                    <button
                      class="icon-btn"
                      title="Up"
                      onclick={() => moveStep(i, -1)}
                      disabled={i === 0}
                      ><Icon name="chevronUp" size={14} /></button
                    >
                    <button
                      class="icon-btn"
                      title="Down"
                      onclick={() => moveStep(i, 1)}
                      disabled={i === fSteps.length - 1}
                      ><Icon name="chevronDown" size={14} /></button
                    >
                    <button
                      class="icon-btn"
                      title="Remove"
                      onclick={() => removeStep(i)}
                      disabled={fSteps.length === 1}
                      ><Icon name="close" size={14} /></button
                    >
                  </div>
                </div>
                <textarea
                  class="field"
                  rows="4"
                  bind:value={step.text}
                  placeholder="Prompt text for this step…"
                ></textarea>
              </div>
            {/each}
            <button class="ghost with-ic" onclick={addStep}
              ><Icon name="plus" size={14} /> Add step</button
            >
          </div>
        </div>
      {/if}

      <label class="fld">
        <span class="fld-label"
          >Tags <span class="dim">(comma separated)</span></span
        >
        <input
          class="field"
          bind:value={fTags}
          placeholder="e.g. coding, review, assistant"
        />
      </label>

      {#if editorVars.length}
        <p class="hint">
          Variables: {#each editorVars as v}<span class="vchip">{v}</span>
          {/each}
        </p>
      {/if}

      <label class="sop-toggle">
        <input
          type="checkbox"
          checked={fKind === "sop"}
          onchange={(e) => setKind(e.currentTarget.checked)}
        />
        <span>Multi-step SOP — copy step by step</span>
      </label>

      <div class="modal-actions">
        <button class="ghost" onclick={() => (editorOpen = false)}
          >Cancel</button
        >
        <button class="btn" onclick={saveItem}>Save</button>
      </div>
    </div>
  </div>
{/if}

<!-- Folder create / edit -->
{#if folderModalOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="overlay"
    onclick={(e) => e.target === e.currentTarget && (folderModalOpen = false)}
  >
    <div class="modal">
      <div class="modal-head">
        <h3>{fmMode === "new" ? "New folder" : "Edit folder"}</h3>
        <button
          class="icon-btn"
          title="Close"
          onclick={() => (folderModalOpen = false)}
          ><Icon name="close" size={16} /></button
        >
      </div>

      <label class="fld">
        <span class="fld-label">Name</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          class="field"
          bind:value={fmName}
          placeholder="e.g. Client work"
          autofocus
        />
      </label>

      <div class="fld">
        <span class="fld-label">Icon</span>
        <div class="icongrid">
          {#each FOLDER_ICON_NAMES as n}
            <button
              class="ic"
              class:sel={fmIcon === n}
              onclick={() => (fmIcon = n)}
            >
              <FolderIcon
                name={n}
                color={fmColor || "var(--muted)"}
                size={18}
              />
            </button>
          {/each}
        </div>
      </div>

      <div class="fld">
        <span class="fld-label">Colour</span>
        <div class="swatches">
          {#each FOLDER_COLORS as c}
            <button
              class="sw"
              class:sel={fmColor === c}
              style:background={c}
              onclick={() => (fmColor = c)}
              aria-label={c}
            ></button>
          {/each}
          <button
            class="sw custom"
            title="Custom colour"
            onclick={() => fmColorInput?.click()}
            ><Icon name="plus" size={13} /></button
          >
          <input
            bind:this={fmColorInput}
            class="hidden-color"
            type="color"
            value={fmColor || "#8b9fa4"}
            onchange={(e) => (fmColor = e.target.value)}
          />
        </div>
      </div>

      <div class="modal-actions folder-actions-row">
        {#if fmMode === "edit"}
          {#if fmConfirmDelete}
            <span class="del-confirm">Delete this folder?</span>
            <button class="ghost danger" onclick={deleteFolderConfirmed}
              >Yes, delete</button
            >
            <button class="ghost" onclick={() => (fmConfirmDelete = false)}
              >No</button
            >
          {:else}
            <button
              class="ghost danger left"
              onclick={() => (fmConfirmDelete = true)}
              ><Icon name="trash" size={14} /> Delete</button
            >
          {/if}
        {/if}
        {#if !fmConfirmDelete}
          <button class="ghost" onclick={() => (folderModalOpen = false)}
            >Cancel</button
          >
          <button class="btn" onclick={saveFolder}
            >{fmMode === "new" ? "Create" : "Save"}</button
          >
        {/if}
      </div>
    </div>
  </div>
{/if}

{#if folderMenu}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="ctx-backdrop"
    onclick={() => (folderMenu = null)}
    oncontextmenu={(e) => {
      e.preventDefault();
      folderMenu = null;
    }}
  ></div>
  <div
    class="ctx-menu"
    style:left="{folderMenu.x}px"
    style:top="{folderMenu.y}px"
  >
    <button
      class="ctx-item"
      onclick={() => menuAction((id) => openEditFolderById(id))}
    >
      <Icon name="edit" size={14} /> Rename
    </button>
    <button
      class="ctx-item"
      onclick={() => menuAction((id) => openEditFolderById(id))}
    >
      <Icon name="droplet" size={14} /> Change icon &amp; colour
    </button>
    <div class="ctx-sep"></div>
    <button
      class="ctx-item danger"
      onclick={() =>
        menuAction((id) => openEditFolderById(id, { confirmDelete: true }))}
    >
      <Icon name="trash" size={14} /> Delete
    </button>
  </div>
{/if}

{#if itemMenu}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="ctx-backdrop"
    onclick={() => (itemMenu = null)}
    oncontextmenu={(e) => {
      e.preventDefault();
      itemMenu = null;
    }}
  ></div>
  <div class="ctx-menu" style:left="{itemMenu.x}px" style:top="{itemMenu.y}px">
    <button
      class="ctx-item"
      onclick={() => {
        const m = itemMenu;
        itemMenu = null;
        openEditItem(m.folderId, m.item);
      }}
    >
      <Icon name="edit" size={14} /> Edit
    </button>
    <button
      class="ctx-item"
      onclick={() => {
        const m = itemMenu;
        itemMenu = null;
        copyItem(m.item);
      }}
    >
      <Icon name="copy" size={14} /> Copy
    </button>
    <button
      class="ctx-item"
      onclick={() => {
        const m = itemMenu;
        itemMenu = null;
        duplicateItem(m.folderId, m.item);
      }}
    >
      <Icon name="layers" size={14} /> Duplicate
    </button>
    <button
      class="ctx-item"
      onclick={() => {
        const m = itemMenu;
        itemMenu = null;
        toggleFav(m.folderId, m.item);
      }}
    >
      <Icon name="star" size={14} fill={itemMenu.item.favorite} />
      {itemMenu.item.favorite ? "Unpin" : "Pin"}
    </button>
    {#if connectors.length}
      <div class="ctx-sep"></div>
      <div class="ctx-label"><Icon name="plug" size={12} /> Send to webhook</div>
      {#each connectors as c (c.id)}
        <button
          class="ctx-item sub"
          onclick={() => {
            const m = itemMenu;
            itemMenu = null;
            sendItemTo(m.item, c);
          }}
        >
          {c.name || c.url}
        </button>
      {/each}
    {/if}
    <div class="ctx-sep"></div>
    <button
      class="ctx-item danger"
      onclick={() => askDeleteItem(itemMenu.folderId, itemMenu.item)}
    >
      <Icon name="trash" size={14} /> Delete
    </button>
  </div>
{/if}

{#if pendingDelete}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="overlay"
    onclick={(e) => e.target === e.currentTarget && (pendingDelete = null)}
  >
    <div class="modal confirm">
      <div class="modal-head"><h3>Delete item</h3></div>
      <p class="confirm-text">
        Are you sure you want to delete <strong
          >“{pendingDelete.item.name}”</strong
        >? This can't be undone.
      </p>
      <div class="modal-actions">
        <button class="ghost" onclick={() => (pendingDelete = null)}
          >Cancel</button
        >
        <button class="ghost danger" onclick={confirmDeleteItem}
          ><Icon name="trash" size={14} /> Delete</button
        >
      </div>
    </div>
  </div>
{/if}

<style>
  .ctx-backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
  }
  .ctx-menu {
    position: fixed;
    z-index: 61;
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
  .ctx-item {
    display: flex;
    align-items: center;
    gap: 9px;
    text-align: left;
    background: none;
    border: none;
    color: var(--text);
    cursor: pointer;
    font-size: 13px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
  }
  .ctx-item:hover {
    background: var(--elevated);
  }
  .ctx-item.danger {
    color: #d98a8a;
  }
  .ctx-item.danger:hover {
    background: color-mix(in srgb, #b5544f 22%, var(--elevated));
    color: #f0b4b4;
  }
  .ctx-sep {
    height: 1px;
    background: var(--border);
    margin: 4px 2px;
  }
  .ctx-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--faint);
    padding: 5px 10px 2px;
  }
  .ctx-item.sub {
    padding-left: 26px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 240px;
  }
  .modal.confirm {
    max-width: 380px;
  }
  .confirm-text {
    color: var(--muted);
    font-size: 14px;
    line-height: 1.55;
    margin: 4px 0 18px;
  }
  .confirm-text strong {
    color: var(--text);
  }

  .layout {
    display: flex;
    height: 100%;
    min-height: 0;
  }
  .rail {
    width: 216px;
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    padding: 14px 12px;
    overflow-y: auto;
    background: color-mix(in srgb, var(--surface) 55%, transparent);
  }
  .rail-head {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
    margin-bottom: 10px;
    padding: 0 4px;
  }
  .virt-row {
    display: flex;
    gap: 4px;
    margin-bottom: 14px;
  }
  .virt {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 9px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: var(--elevated);
    color: var(--muted);
    cursor: pointer;
    font-size: 12.5px;
    min-width: 0;
  }
  .virt:hover {
    color: var(--text);
  }
  .virt.active {
    background: var(--accent-soft);
    border-color: color-mix(in srgb, var(--accent) 40%, var(--border));
    color: var(--text);
  }
  .virt .vl {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .virt .vc {
    color: var(--faint);
    font-size: 11px;
  }
  .rail-sub {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--faint);
    padding: 0 4px;
    margin-bottom: 6px;
  }
  .folders {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .folder-li {
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
  }
  .folder-li[draggable="true"] {
    cursor: grab;
  }
  .folder-li.drop {
    border-color: var(--accent);
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .folder-li.ghost {
    opacity: 0.4;
  }
  .newfolder {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    width: 100%;
    margin-top: 12px;
    padding: 9px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--muted);
    cursor: pointer;
    font-size: 13px;
    transition:
      color 0.12s var(--ease),
      border-color 0.12s var(--ease);
  }
  .newfolder:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .folder {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    padding: 8px 10px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text);
    cursor: pointer;
    font-size: 13.5px;
    text-align: left;
  }
  .folder:hover {
    background: var(--elevated);
  }
  .folder.active {
    background: var(--accent-soft);
    border-color: color-mix(in srgb, var(--accent) 40%, var(--border));
    box-shadow: var(--edge);
  }
  .ficon {
    width: 18px;
    text-align: center;
    flex-shrink: 0;
  }
  .fname {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    color: var(--muted);
    font-size: 12px;
  }
  .main {
    flex: 1;
    min-width: 0;
    padding: 18px 22px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }
  .toolbar {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-bottom: 14px;
  }
  .sortsel {
    background: var(--well);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--muted);
    font-size: 13px;
    padding: 9px 10px;
    cursor: pointer;
  }
  .sortsel:hover {
    color: var(--text);
    border-color: var(--border-strong);
  }
  .search-wrap {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
  }
  .search-ic {
    position: absolute;
    left: 12px;
    color: var(--muted);
    font-size: 15px;
    pointer-events: none;
  }
  .search {
    flex: 1;
    padding: 9px 12px 9px 34px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--well);
    color: var(--text);
    font-size: 13px;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.28);
    transition: border-color 0.12s var(--ease);
  }
  .search:focus {
    outline: none;
    border-color: var(--accent);
  }
  .folder-bar {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
    font-size: 15px;
  }
  .edit-folder {
    margin-left: auto;
  }
  /* Item / folder editor extras */
  .modal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .modal-head h3 {
    margin: 0;
  }
  .fld {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .fld-label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
  }
  .fld-label .dim {
    text-transform: none;
    letter-spacing: 0;
    color: var(--faint);
  }
  .charcount {
    align-self: flex-end;
    font-size: 11px;
    color: var(--faint);
  }
  .sop-toggle {
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 13px;
    color: var(--text);
    cursor: pointer;
    border-top: 1px solid var(--border);
    padding-top: 12px;
  }
  .folder-actions-row {
    align-items: center;
  }
  .folder-actions-row .left {
    margin-right: auto;
  }
  .del-confirm {
    margin-right: auto;
    font-size: 13px;
    color: var(--muted);
  }
  .ghost.danger {
    color: #d98a8a;
  }
  .ghost.danger:hover {
    border-color: #d98a8a;
    background: color-mix(in srgb, #d98a8a 10%, transparent);
  }
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 30;
  }
  .cust {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 31;
    width: 250px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-modal), var(--edge);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .cust-sec {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--muted);
  }
  .icongrid {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 3px;
  }
  .ic {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 30px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: none;
    cursor: pointer;
  }
  .ic:hover {
    background: var(--elevated);
  }
  .ic.sel {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }
  .sw {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    padding: 0;
  }
  .sw.sel {
    border-color: var(--text);
  }
  .sw.custom {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--elevated);
    border: 1px dashed var(--border-strong);
    color: var(--muted);
  }
  .fb-icon {
    font-size: 16px;
  }
  .fcount {
    color: var(--muted);
    font-size: 12px;
  }
  .folder-actions {
    display: flex;
    align-items: center;
    gap: 3px;
    margin-left: auto;
  }
  .swatch {
    width: 14px;
    height: 14px;
    border-radius: 4px;
    border: 1px solid var(--border);
    display: block;
  }
  .hidden-color {
    position: absolute;
    width: 0;
    height: 0;
    padding: 0;
    border: 0;
    opacity: 0;
    pointer-events: none;
  }
  .tagbar {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 14px;
  }
  .chip.clear {
    color: var(--muted);
    border-style: dashed;
    cursor: pointer;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 14px;
    align-content: start;
  }
  .card {
    position: relative;
    border: 1px solid color-mix(in srgb, var(--fcolor) 55%, var(--border));
    border-radius: var(--radius);
    background: var(--surface);
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 9px;
    box-shadow: var(--shadow-card);
    cursor: grab;
    transition:
      border-color 0.12s var(--ease),
      transform 0.08s var(--ease);
  }
  .card:active {
    cursor: grabbing;
  }
  .card:hover {
    border-color: color-mix(in srgb, var(--fcolor) 85%, transparent);
    transform: translateY(-1px);
  }
  .card.fav {
    border-color: color-mix(in srgb, var(--fcolor) 75%, var(--border));
  }
  .card.sel {
    border-color: var(--accent);
    box-shadow:
      inset 0 0 0 1px var(--accent),
      var(--shadow-card);
  }
  .card.ghost {
    opacity: 0.4;
  }
  /* Insertion indicator when dragging a card next to another */
  .card.drop-before::before,
  .card.drop-after::after {
    content: "";
    position: absolute;
    top: 6px;
    bottom: 6px;
    width: 3px;
    border-radius: 3px;
    background: var(--accent);
  }
  .card.drop-before::before {
    left: -8px;
  }
  .card.drop-after::after {
    right: -8px;
  }
  .card.compact {
    gap: 6px;
    padding: 11px 14px;
  }
  .selnum {
    position: absolute;
    top: -9px;
    left: -9px;
    min-width: 22px;
    height: 22px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 12px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: var(--shadow-card);
    z-index: 4;
  }
  .name {
    font-weight: 600;
    font-size: 14.5px;
    line-height: 1.3;
    padding-right: 30px;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    word-break: break-word;
  }

  /* Corner type marker */
  .type-corner {
    position: absolute;
    top: 11px;
    right: 12px;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--muted);
    font-weight: 600;
    pointer-events: none;
  }
  .fav-mark {
    display: flex;
    color: var(--accent-strong);
  }
  .tc-n {
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: 10.5px;
  }
  .tc-n.uses {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }
  .sop-badge {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    vertical-align: 1px;
    margin-left: 6px;
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: 10.5px;
    font-weight: 600;
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 1px 5px;
    background: var(--elevated);
  }

  /* Hover actions on a dark corner-piece */
  .hover-actions {
    position: absolute;
    top: 0;
    right: 0;
    z-index: 3;
    display: flex;
    align-items: center;
    gap: 1px;
    padding: 8px 10px 14px 20px;
    border-radius: 0 var(--radius) 0 16px;
    background: radial-gradient(
      135% 135% at 100% 0%,
      color-mix(in srgb, var(--bg) 95%, transparent) 44%,
      transparent 74%
    );
    opacity: 0;
    transition: opacity 0.12s var(--ease);
  }
  .card:hover .hover-actions {
    opacity: 1;
  }
  .star {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--muted);
    line-height: 1;
    padding: 3px;
    display: flex;
    transition: color 0.12s;
  }
  .star.on {
    color: var(--accent-strong);
  }
  .icon-btn.xs {
    width: 26px;
    height: 26px;
    font-size: 12px;
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 2px 7px;
    color: var(--muted);
    white-space: nowrap;
  }
  .badge.origin {
    text-transform: none;
    letter-spacing: 0;
  }
  .odot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .preview {
    margin: 0;
    font-size: 13px;
    color: var(--muted);
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    white-space: pre-wrap;
    word-break: break-word;
    min-height: 1em;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }
  .card footer {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 2px;
  }
  .act {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text);
    cursor: pointer;
    font-size: 12px;
    transition:
      border-color 0.12s var(--ease),
      background 0.12s var(--ease);
  }
  .act:hover {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }
  .act.primary {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
  }
  .empty {
    color: var(--muted);
    font-size: 14px;
    margin-top: 8px;
  }
  .erow {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .kind {
    display: flex;
    gap: 8px;
  }
  .kind button {
    flex: 1;
    padding: 9px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text);
    cursor: pointer;
    font-size: 13px;
  }
  .kind button.active {
    border-color: var(--accent);
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }
  .steps {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .step {
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .step-head {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .step-title {
    flex: 1;
  }
  .step-ctl {
    display: flex;
    gap: 3px;
  }

  /* Multi-select action bar (sticky at the bottom of the workspace) */
  .selbar {
    position: sticky;
    bottom: 0;
    margin-top: 16px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 11px 14px;
    background: var(--elevated);
    border: 1px solid color-mix(in srgb, var(--accent) 45%, var(--border));
    border-radius: var(--radius);
    box-shadow: var(--shadow-modal);
    flex-wrap: wrap;
  }
  .selinfo {
    display: flex;
    flex-direction: column;
    line-height: 1.25;
  }
  .selcount {
    font-weight: 600;
    font-size: 13px;
  }
  .selhint {
    font-size: 11px;
    color: var(--muted);
  }
  .selactions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
</style>
