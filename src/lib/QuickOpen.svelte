<script>
  import { itemVars, itemPlainText, applyVars } from "./vars.js";
  import { clipCopy, libRecordUse } from "./api.js";
  import Icon from "./Icon.svelte";
  import FolderIcon from "./FolderIcon.svelte";

  // props
  let { library, activeProfile = null, flash, onFill, onClose, onUsed = () => {} } = $props();

  let query = $state("");
  let active = $state(0);
  let inputEl;

  $effect(() => {
    inputEl?.focus();
  });

  // Flatten every item across folders, carrying its folder for context.
  let all = $derived.by(() => {
    const out = [];
    for (const f of library.folders || []) {
      for (const i of f.items || []) {
        out.push({ item: i, folderId: f.id, folderName: f.name, folderIcon: f.icon, folderColor: f.color });
      }
    }
    return out;
  });

  // Simple subsequence fuzzy match on "name · folder · tags".
  function score(hay, q) {
    hay = hay.toLowerCase();
    q = q.toLowerCase();
    if (!q) return 0;
    if (hay.includes(q)) return 1000 - hay.indexOf(q);
    let hi = 0,
      qi = 0,
      s = 0;
    while (hi < hay.length && qi < q.length) {
      if (hay[hi] === q[qi]) {
        s += 1;
        qi += 1;
      }
      hi += 1;
    }
    return qi === q.length ? s : -1;
  }

  let results = $derived.by(() => {
    const q = query.trim();
    const scored = all.map((e) => {
      const hay = `${e.item.name} ${e.folderName} ${(e.item.tags || []).join(" ")} ${e.item.item_type || ""}`;
      return { ...e, s: q ? score(hay, q) : 0 };
    });
    const filtered = q ? scored.filter((e) => e.s >= 0) : scored;
    filtered.sort((a, b) => b.s - a.s || Number(b.item.favorite) - Number(a.item.favorite));
    return filtered.slice(0, 40);
  });

  $effect(() => {
    // keep the highlighted row in range as results change
    void results;
    if (active >= results.length) active = Math.max(0, results.length - 1);
  });

  async function choose(entry) {
    if (!entry) return;
    const item = entry.item;
    // No active profile + has variables → open the fill / step-by-step flow.
    if (itemVars(item).length && !activeProfile) {
      onFill(item, item.kind === "sop" ? "steps" : "auto");
      return;
    }
    const raw = itemPlainText(item);
    const text = applyVars(raw, activeProfile?.values || {});
    const ok = await clipCopy(text);
    flash(ok ? `Copied “${item.name}”${activeProfile ? " · " + activeProfile.name : ""}` : "Copy failed");
    if (ok) libRecordUse(item.id).then(onUsed).catch(() => {});
    onClose();
  }

  function onKey(e) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      active = Math.min(active + 1, results.length - 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      active = Math.max(active - 1, 0);
    } else if (e.key === "Enter") {
      e.preventDefault();
      choose(results[active]);
    } else if (e.key === "Escape") {
      onClose();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="overlay top" onclick={(e) => e.target === e.currentTarget && onClose()}>
  <div class="palette">
    <div class="qhead">
      <Icon name="search" size={17} />
      <input
        bind:this={inputEl}
        class="q"
        placeholder="Search every item — Enter to copy, ↑↓ to move…"
        bind:value={query}
        onkeydown={onKey}
      />
    </div>
    <div class="results">
      {#if results.length === 0}
        <div class="none">No matches</div>
      {:else}
        {#each results as e, i (e.item.id)}
          {@const vars = itemVars(e.item)}
          <button
            type="button"
            class="row"
            class:active={i === active}
            onmousemove={() => (active = i)}
            onclick={() => choose(e)}
          >
            <span class="ricon">
              {#if e.folderIcon}<FolderIcon name={e.folderIcon} color={e.folderColor || "var(--muted)"} size={15} />{:else}<Icon name={e.item.kind === "sop" ? "sop" : "template"} size={15} />{/if}
            </span>
            <span class="rname">{e.item.name}</span>
            <span class="rfolder">{e.folderName}</span>
            {#if e.item.kind === "sop"}<span class="rtag">SOP · {e.item.steps.length}</span>{/if}
            {#if vars.length}<span class="rtag accent">fill {vars.length}</span>{/if}
            <span class="rhint">{vars.length ? "Fill & copy" : "Copy"} ↵</span>
          </button>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .overlay.top {
    align-items: flex-start;
    padding-top: 12vh;
  }
  .palette {
    width: 100%;
    max-width: 620px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-modal);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .qhead {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 14px 17px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
  }
  .q {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 15px;
    padding: 0;
  }
  .q:focus {
    outline: none;
  }
  .results {
    max-height: 52vh;
    overflow-y: auto;
    padding: 6px;
  }
  .none {
    padding: 22px;
    text-align: center;
    color: var(--muted);
    font-size: 13px;
  }
  .row {
    width: 100%;
    border: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 11px;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .row.active {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  .ricon {
    width: 18px;
    text-align: center;
    flex-shrink: 0;
  }
  .rname {
    font-size: 13.5px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex-shrink: 1;
    min-width: 0;
  }
  .rfolder {
    font-size: 11.5px;
    color: var(--muted);
    white-space: nowrap;
  }
  .rtag {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    color: var(--muted);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 1px 5px;
    flex-shrink: 0;
  }
  .rtag.accent {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
  }
  .rhint {
    margin-left: auto;
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
    flex-shrink: 0;
  }
</style>
