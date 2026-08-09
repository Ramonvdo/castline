<script>
  // Preview a blueprint before anything lands in the library: what's inside,
  // which {{variables}} it expects, and where it should go. Nothing is written
  // until Import is pressed.
  import { blueprintParse, blueprintImport } from "./api.js";
  import { extractVars } from "./vars.js";
  import Icon from "./Icon.svelte";

  let { text, folders = [], defaultFolderId = null, flash, onClose, onImported } = $props();

  let bp = $state(null);
  let busy = $state(false);
  let target = $state("");

  // Parsing is async, so remember what we last parsed rather than deriving —
  // a $state flag here would re-trigger this effect and loop.
  let parsed = "";
  $effect(() => {
    const t = text;
    if (!t || t === parsed) return;
    parsed = t;
    blueprintParse(t)
      .then((data) => {
        // Drop a stale result: a second file dropped mid-parse must not leave
        // the preview showing one blueprint while Import commits another.
        if (t !== parsed) return;
        bp = data;
        // A blueprint that carries its own folder offers to recreate it ("" =
        // create), otherwise drop into whichever folder the user was just in.
        target = data.folder ? "" : (defaultFolderId ?? folders[0]?.id ?? "");
      })
      .catch((e) => {
        if (t !== parsed) return;
        flash(String(e));
        onClose();
      });
  });

  // Recomputed from the items — bp.variables is informational and could be
  // stale or hand-edited, so it's never trusted.
  let vars = $derived.by(() => {
    if (!bp) return [];
    const all = [];
    for (const i of bp.items || []) {
      const texts = [i.subject || "", i.text || "", ...(i.steps || []).map((s) => s.text || "")];
      for (const t of texts) {
        for (const v of extractVars(t)) if (!all.includes(v)) all.push(v);
      }
    }
    return all;
  });

  function snippet(item) {
    const raw = item.kind === "sop" ? item.steps?.[0]?.text || "" : item.text || "";
    return raw.trim();
  }

  async function doImport() {
    if (busy || !bp) return;
    busy = true;
    try {
      const data = await blueprintImport(target || null, text);
      onImported(data);
      const n = bp.items.length;
      flash(`Imported ${n} template${n === 1 ? "" : "s"}`);
      onClose();
    } catch (e) {
      flash(String(e));
    }
    busy = false;
  }
</script>

{#if bp}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="overlay" onclick={(e) => e.target === e.currentTarget && onClose()}>
    <div class="modal">
      <div class="bp-head">
        <h3>Import blueprint</h3>
        <span class="bp-count">
          {bp.items.length} template{bp.items.length === 1 ? "" : "s"}
        </span>
      </div>

      <div class="bp-items">
        {#each bp.items as item, i (i)}
          <div class="bp-item">
            <div class="bp-name">
              <span class="bp-title">{item.name}</span>
              {#if item.kind === "sop"}
                <span class="bp-badge"><Icon name="sop" size={11} />{item.steps?.length || 0}</span>
              {:else if item.type === "email"}
                <span class="bp-badge">✉</span>
              {/if}
            </div>
            {#if snippet(item)}<p class="bp-snippet">{snippet(item)}</p>{/if}
            {#if (item.tags || []).length}
              <div class="bp-tags">
                {#each item.tags.slice(0, 4) as t}<span class="chip">{t}</span>{/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>

      {#if vars.length}
        <div class="bp-vars">
          <span class="bp-label">Fills</span>
          <div class="bp-chips">
            {#each vars as v}<span class="vchip">{v}</span>{/each}
          </div>
        </div>
      {/if}

      <label>
        Import into
        <select class="field" bind:value={target}>
          {#if bp.folder}
            <option value="">+ New folder: {bp.folder.name}</option>
          {:else if !folders.length}
            <!-- Nowhere to put it yet — the backend creates this on import. -->
            <option value="">+ New folder: Imported</option>
          {/if}
          {#each folders as f (f.id)}
            <option value={f.id}>{f.name}</option>
          {/each}
        </select>
      </label>

      <p class="hint">
        Imported templates are added as copies — your existing items are never overwritten.
      </p>

      <div class="modal-actions">
        <button class="ghost" onclick={onClose}>Cancel</button>
        <button class="btn" disabled={busy} onclick={doImport}>
          {busy ? "Importing…" : "Import"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .bp-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
  }
  .bp-head h3 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }
  .bp-count {
    font-size: 12px;
    color: var(--muted);
  }
  .bp-items {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 260px;
    overflow-y: auto;
  }
  .bp-item {
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--well);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .bp-name {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .bp-title {
    font-weight: 600;
    font-size: 13.5px;
  }
  .bp-badge {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: 10.5px;
    font-weight: 600;
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 1px 5px;
    background: var(--elevated);
  }
  .bp-snippet {
    margin: 0;
    font-size: 12.5px;
    color: var(--muted);
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .bp-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }
  .bp-vars {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .bp-label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
  }
  .bp-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }
</style>
