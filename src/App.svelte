<script>
  import { onMount } from "svelte";
  import Library from "./lib/Library.svelte";
  import QuickOpen from "./lib/QuickOpen.svelte";
  import Profiles from "./lib/Profiles.svelte";
  import Connectors from "./lib/Connectors.svelte";
  import Agent from "./lib/Agent.svelte";
  import Settings from "./lib/Settings.svelte";
  import FillCopy from "./lib/FillCopy.svelte";
  import BlueprintImport from "./lib/BlueprintImport.svelte";
  import Tour from "./lib/Tour.svelte";
  import Icon from "./lib/Icon.svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    getLibrary,
    getProfiles,
    getSettings,
    onProfilesChanged,
    onLibraryChanged,
    onScheduleRan,
    storageWarnings,
    setTourSeen,
  } from "./lib/api.js";

  const appWindow = getCurrentWindow();
  const winMinimize = () => appWindow.minimize();
  const winToggleMax = () => appWindow.toggleMaximize();
  const winClose = () => appWindow.close();

  let library = $state({ folders: [] });
  let profiles = $state({ profiles: [], layout: [] });
  let settings = $state({ theme: "dark", connectors: [] });

  let view = $state("library"); // library | profiles | connectors | agent | settings
  let quickOpen = $state(false);
  let fillItem = $state(null);
  let fillMode = $state("auto"); // auto | steps

  // An instruction queued for the Agent tab (e.g. "Enrich → Ask the Agent").
  let agentPrompt = $state("");
  function askAgent(text) {
    agentPrompt = text;
    view = "agent";
  }

  // View density (persisted): full → compact → super compact, one button.
  // Migrates the old boolean "castline-compact" key on first run.
  let viewMode = $state(
    localStorage.getItem("castline-view") ||
      (localStorage.getItem("castline-compact") === "1" ? "compact" : "full"),
  );
  // Set the density without persisting — the tour borrows full cards for its
  // duration and hands the user's choice back, so a crash mid-tour must not
  // leave "full" written over their preference.
  function setViewMode(m) {
    viewMode = m;
  }
  function cycleView() {
    viewMode =
      viewMode === "full"
        ? "compact"
        : viewMode === "compact"
          ? "super"
          : "full";
    localStorage.setItem("castline-view", viewMode);
    flash(
      viewMode === "full"
        ? "Full cards"
        : viewMode === "compact"
          ? "Compact view"
          : "Super compact — hover a card for its actions",
    );
  }

  // Safe mode (persisted, default ON): refuse to send unfilled {{variables}}
  // to external webhooks — the app asks you to fill them first.
  let safeMode = $state(localStorage.getItem("castline-safe") !== "0");
  function toggleSafe() {
    safeMode = !safeMode;
    localStorage.setItem("castline-safe", safeMode ? "1" : "0");
    flash(
      safeMode
        ? "Safe mode on — unfilled {{variables}} won't be sent"
        : "Safe mode off",
    );
  }

  // Active profile (top-right selector). When set, card Copy auto-fills it.
  let activeProfileId = $state(null);
  let profileMenuOpen = $state(false);
  let activeProfile = $derived(
    profiles.profiles.find((p) => p.id === activeProfileId) || null,
  );
  $effect(() => {
    if (
      activeProfileId &&
      !profiles.profiles.some((p) => p.id === activeProfileId)
    )
      activeProfileId = null;
  });

  let toast = $state("");
  let toastTimer;
  function flash(m, ms = 2000) {
    toast = m;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = ""), ms);
    // The tour's "copy it to continue" gate watches this: every copy path in
    // the app flashes, so one counter covers cards, the fill modal and Ctrl+K
    // without threading a callback through each of them.
    if (/^Copied/i.test(m)) copyCount += 1;
  }

  // ── First-run walkthrough ──
  let tour = $state(null); // bound Tour component
  let copyCount = $state(0);
  // The card the tour spotlights, so its copy describes that item rather than
  // the seeded example.
  let firstItem = $state(null);
  function replayTour() {
    view = "library";
    tour?.start();
  }

  // A store failed to load (e.g. a corrupt file was quarantined). This is a
  // data-loss notice, so it gets its own dismissable banner — not the ephemeral
  // toast, which any "Copied" would clobber within its window.
  let storeWarn = $state("");

  // A blueprint waiting to be previewed (from a drop, the toolbar, or the clipboard).
  let pendingBlueprint = $state("");
  let dragging = $state(false);
  // Which folder the library is showing, so an import defaults there.
  let libraryFolderId = $state(null);
  async function drainStoreWarnings() {
    const warns = await storageWarnings();
    if (warns.length) storeWarn = warns.join(" · ");
  }

  onMount(async () => {
    settings = await getSettings();
    library = await getLibrary();
    profiles = await getProfiles();

    // Two things must wait for the window to actually be on screen: a store-load
    // warning, and the first-run welcome prompt. An autostart launch starts
    // hidden in the tray, and either one fired into a hidden window is simply
    // lost. Defer both to the first time it's shown/focused.
    const onFirstShown = () => {
      drainStoreWarnings();
      if (!settings.tour_seen) tour?.offerWelcome();
    };
    if (await appWindow.isVisible()) {
      onFirstShown();
    } else {
      const unShown = await appWindow.onFocusChanged(({ payload: focused }) => {
        if (focused) {
          onFirstShown();
          unShown();
        }
      });
    }

    const un = await onProfilesChanged(async () => {
      profiles = await getProfiles();
      flash("New profile received");
    });
    const unLib = await onLibraryChanged(async () => {
      library = await getLibrary();
    });
    const unSched = await onScheduleRan(async (msg) => {
      flash(String(msg));
      settings = await getSettings(); // pick up the new last_run stamps
    });

    const onKey = (e) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        quickOpen = true;
      } else if (e.key === "Escape") {
        quickOpen = false;
        profileMenuOpen = false;
        pendingBlueprint = "";
        // NOT fillItem: that modal holds typed values, so it owns its own
        // Escape handling and asks before discarding them.
      }
    };
    window.addEventListener("keydown", onKey);

    // ── Drop a blueprint file anywhere on the window to import it ──
    // tauri.conf.json keeps `dragDropEnabled: false` (Tauri's native handler
    // would swallow the web drag events the library's own card reordering runs
    // on), so this is plain HTML5 drag & drop. `types` tells the two apart: a
    // card being dragged carries no files, so it never reaches the import path.
    const hasFiles = (e) =>
      !!e.dataTransfer && Array.from(e.dataTransfer.types || []).includes("Files");
    let dragDepth = 0;
    const onDragEnter = (e) => {
      if (!hasFiles(e)) return;
      dragDepth += 1;
      dragging = true;
    };
    const onDragOver = (e) => {
      // Without this the webview navigates away to the dropped file.
      if (hasFiles(e)) e.preventDefault();
    };
    // No hasFiles() guard here: a leave event that reports no types would
    // otherwise be skipped and strand the overlay on screen forever.
    const onDragLeave = () => {
      dragDepth = Math.max(0, dragDepth - 1);
      if (dragDepth === 0) dragging = false;
    };
    // A drag cancelled with Escape fires neither leave nor drop.
    const onDragEnd = () => {
      dragDepth = 0;
      dragging = false;
    };
    const onDrop = async (e) => {
      if (!hasFiles(e)) return;
      e.preventDefault();
      dragDepth = 0;
      dragging = false;
      const file = e.dataTransfer.files?.[0];
      if (!file) return;
      if (!file.name.toLowerCase().endsWith(".json")) {
        flash("Drop a .json blueprint file");
        return;
      }
      if (file.size > 1_000_000) {
        flash("That file is too large to be a blueprint");
        return;
      }
      try {
        pendingBlueprint = await file.text();
      } catch (err) {
        flash(String(err));
      }
    };
    window.addEventListener("dragenter", onDragEnter);
    window.addEventListener("dragover", onDragOver);
    window.addEventListener("dragleave", onDragLeave);
    window.addEventListener("dragend", onDragEnd);
    window.addEventListener("drop", onDrop);

    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("dragenter", onDragEnter);
      window.removeEventListener("dragover", onDragOver);
      window.removeEventListener("dragleave", onDragLeave);
      window.removeEventListener("dragend", onDragEnd);
      window.removeEventListener("drop", onDrop);
      un();
      unLib();
      unSched();
    };
  });

  function openFill(item, mode = "auto") {
    quickOpen = false;
    fillMode = mode;
    fillItem = item;
  }

  const NAV = [
    { id: "library", label: "Library", icon: "layers" },
    { id: "profiles", label: "Profiles", icon: "user" },
    { id: "connectors", label: "Connectors", icon: "plug" },
    { id: "agent", label: "Agent", icon: "terminal" },
    { id: "settings", label: "Settings", icon: "sliders" },
  ];
</script>

<div class="shell">
  <header class="titlebar" data-tauri-drag-region>
    <div class="left" data-tauri-drag-region>
      <button class="brand" onclick={() => (view = "library")} title="Castline">
        <img class="brand-img" src="/icon.png" alt="Castline" />
      </button>
      <nav class="nav">
        {#each NAV as n}
          <button
            class="navlink"
            class:active={view === n.id}
            data-tour={"nav-" + n.id}
            onclick={() => (view = n.id)}>{n.label}</button
          >
        {/each}
      </nav>
    </div>

    <div class="right">
      <!-- Active profile selector -->
      <div class="profsel" data-tour="profsel">
        <button
          class="profbtn"
          class:on={activeProfile}
          onclick={() => (profileMenuOpen = !profileMenuOpen)}
          title="Fill copies with this profile"
        >
          <Icon name="user" size={14} />
          <span class="pl"
            >{activeProfile ? activeProfile.name : "No profile"}</span
          >
          <Icon name="chevronDown" size={13} />
        </button>
        {#if profileMenuOpen}
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
          <div class="backdrop" onclick={() => (profileMenuOpen = false)}></div>
          <div class="menu">
            <button
              class="mi"
              class:sel={!activeProfileId}
              onclick={() => {
                activeProfileId = null;
                profileMenuOpen = false;
              }}
            >
              No profile
            </button>
            {#if profiles.profiles.length === 0}
              <div class="mi empty">No profiles yet</div>
            {:else}
              {#each profiles.profiles as p (p.id)}
                <button
                  class="mi"
                  class:sel={p.id === activeProfileId}
                  onclick={() => {
                    activeProfileId = p.id;
                    profileMenuOpen = false;
                  }}
                >
                  {p.name}
                </button>
              {/each}
            {/if}
          </div>
        {/if}
      </div>

      <button
        class="ghost eye"
        class:on={viewMode !== "full"}
        onclick={cycleView}
        title={viewMode === "full"
          ? "Compact view"
          : viewMode === "compact"
            ? "Super compact view — Copy/View appear on hover"
            : "Back to full cards"}
      >
        <Icon
          name={viewMode === "full"
            ? "eye"
            : viewMode === "compact"
              ? "eyeOff"
              : "rows"}
          size={16}
        />
      </button>

      <button
        class="ghost eye"
        class:on={safeMode}
        onclick={toggleSafe}
        title={safeMode
          ? "Safe mode ON — unfilled {{variables}} can't be sent to webhooks"
          : "Safe mode OFF — sends go out even with unfilled {{variables}}"}
      >
        <Icon name="shield" size={16} />
      </button>

      <button
        class="ghost search-cta"
        data-tour="quickfind"
        onclick={() => (quickOpen = true)}
      >
        <Icon name="command" size={15} /><span>Search</span><kbd>Ctrl K</kbd>
      </button>

      <div class="winctl">
        <button class="wbtn" title="Minimize" onclick={winMinimize}
          ><Icon name="winMin" size={15} /></button
        >
        <button class="wbtn" title="Maximize" onclick={winToggleMax}
          ><Icon name="winMax" size={13} /></button
        >
        <button class="wbtn danger" title="Close" onclick={winClose}
          ><Icon name="close" size={15} /></button
        >
      </div>
    </div>
  </header>

  <main class="body">
    {#if view === "library"}
      <Library
        bind:library
        profiles={profiles.profiles}
        layout={profiles.layout || []}
        {activeProfile}
        {viewMode}
        {safeMode}
        connectors={settings.connectors || []}
        {flash}
        onFill={openFill}
        onBlueprintText={(t) => (pendingBlueprint = t)}
        bind:currentFolderId={libraryFolderId}
        onFirstItem={(x) => (firstItem = x)}
      />
    {:else if view === "profiles"}
      <Profiles
        profiles={profiles.profiles}
        layout={profiles.layout || []}
        locked={profiles.locked || []}
        folders={library.folders}
        connectors={settings.connectors || []}
        llm={settings.llm || {}}
        {flash}
        onData={(d) => (profiles = d)}
        onAgent={askAgent}
      />
    {:else if view === "connectors"}
      <Connectors
        connectors={settings.connectors || []}
        folders={library.folders}
        {flash}
        onSettings={(s) => (settings = s)}
      />
    {:else if view === "settings"}
      <Settings
        {flash}
        folders={library.folders}
        connectors={settings.connectors || []}
        onLibraryData={(d) => (library = d)}
        onProfilesData={(d) => (profiles = d)}
        onSettings={(s) => (settings = s)}
        onReplayTour={replayTour}
      />
    {/if}
    <!-- Agent stays mounted so the terminal survives tab switches -->
    <div class="agent-wrap" style:display={view === "agent" ? "block" : "none"}>
      <Agent
        active={view === "agent"}
        pending={agentPrompt}
        onPendingSent={() => (agentPrompt = "")}
      />
    </div>
  </main>
</div>

{#if toast}<div class="toast">{toast}</div>{/if}

{#if storeWarn}
  <div class="store-warn" role="alert">
    <Icon name="info" size={15} />
    <span>{storeWarn}</span>
    <button type="button" class="sw-dismiss" onclick={() => (storeWarn = "")} aria-label="Dismiss">
      <Icon name="close" size={14} />
    </button>
  </div>
{/if}

{#if quickOpen}
  <QuickOpen
    {library}
    {activeProfile}
    {flash}
    onFill={openFill}
    onClose={() => (quickOpen = false)}
    onUsed={(d) => (library = d)}
  />
{/if}

{#if fillItem}
  <FillCopy
    item={fillItem}
    mode={fillMode}
    profiles={profiles.profiles}
    layout={profiles.layout || []}
    {activeProfile}
    {safeMode}
    llm={settings.llm || {}}
    connectors={settings.connectors || []}
    {flash}
    onClose={() => (fillItem = null)}
    onUsed={(d) => (library = d)}
  />
{/if}

{#if pendingBlueprint}
  <BlueprintImport
    text={pendingBlueprint}
    folders={library.folders}
    defaultFolderId={libraryFolderId || library.folders[0]?.id || null}
    {flash}
    onClose={() => (pendingBlueprint = "")}
    onImported={(d) => (library = d)}
  />
{/if}

{#if dragging}
  <div class="dropzone"><span>Drop a blueprint to import</span></div>
{/if}

<Tour
  bind:this={tour}
  {view}
  {quickOpen}
  {fillItem}
  {copyCount}
  {viewMode}
  {firstItem}
  profileCount={profiles.profiles.length}
  onView={(v) => (view = v)}
  onViewMode={setViewMode}
  onCloseFill={() => (fillItem = null)}
  onCloseQuickOpen={() => (quickOpen = false)}
  onFinish={() => setTourSeen(true).catch(() => {})}
/>

<style>
  /* Purely a hint — never intercepts the drop itself. */
  .dropzone {
    position: fixed;
    inset: 12px;
    z-index: 100;
    pointer-events: none;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 2px dashed var(--accent);
    border-radius: var(--radius-lg);
    background: color-mix(in srgb, var(--bg) 72%, transparent);
    backdrop-filter: blur(2px);
  }
  .dropzone span {
    background: var(--elevated);
    border: 1px solid color-mix(in srgb, var(--accent) 45%, var(--border));
    border-radius: 999px;
    padding: 10px 20px;
    font-size: 14px;
    font-weight: 600;
    box-shadow: var(--shadow-modal);
  }
  .shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  .titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 46px;
    padding: 0 0 0 14px;
    border-bottom: 1px solid var(--border);
    background: var(--sheen), var(--surface);
    box-shadow: var(--edge);
    flex-shrink: 0;
    user-select: none;
  }
  .left,
  .right {
    display: flex;
    align-items: center;
    height: 100%;
  }
  .brand {
    display: flex;
    align-items: center;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    margin-right: 16px;
  }
  .brand-img {
    width: 30px;
    height: 30px;
    display: block;
    border-radius: 7px;
  }
  .nav {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .navlink {
    background: none;
    border: none;
    color: var(--muted);
    cursor: pointer;
    font-size: 13px;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    transition:
      color 0.12s var(--ease),
      background 0.12s var(--ease);
  }
  .navlink:hover {
    color: var(--text);
  }
  .navlink.active {
    color: var(--text);
    font-weight: 600;
  }
  .right {
    gap: 8px;
    padding-right: 8px;
  }
  .profsel {
    position: relative;
  }
  .profbtn {
    display: flex;
    align-items: center;
    gap: 7px;
    background: var(--elevated);
    border: 1px solid var(--border);
    color: var(--muted);
    cursor: pointer;
    font-size: 13px;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    transition: all 0.12s var(--ease);
  }
  .profbtn:hover {
    color: var(--text);
    border-color: var(--border-strong);
  }
  .profbtn.on {
    color: var(--on-accent);
    background: var(--btn-accent);
    border-color: color-mix(in srgb, var(--accent) 55%, #000);
    font-weight: 600;
  }
  .profbtn .pl {
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
  }
  .menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 41;
    min-width: 200px;
    max-height: 60vh;
    overflow-y: auto;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-modal), var(--edge);
    padding: 5px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .mi {
    text-align: left;
    background: none;
    border: none;
    color: var(--text);
    cursor: pointer;
    font-size: 13px;
    padding: 7px 9px;
    border-radius: var(--radius-sm);
  }
  .mi:hover {
    background: var(--elevated);
  }
  .mi.sel {
    color: var(--accent-strong);
    background: var(--accent-soft);
    font-weight: 600;
  }
  .mi.empty {
    color: var(--faint);
    cursor: default;
  }
  .eye {
    padding: 8px 9px;
    color: var(--muted);
  }
  .eye.on {
    color: var(--accent-strong);
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
    background: var(--accent-soft);
  }
  .search-cta {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--muted);
  }
  .search-cta kbd {
    font-family: inherit;
    font-size: 10px;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1px 5px;
    color: var(--muted);
  }
  .winctl {
    display: flex;
    align-items: stretch;
    height: 100%;
    margin-left: 4px;
  }
  .wbtn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 42px;
    height: 100%;
    border: none;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    transition:
      background 0.12s var(--ease),
      color 0.12s var(--ease);
  }
  .wbtn:hover {
    background: var(--elevated);
    color: var(--text);
  }
  .wbtn.danger:hover {
    background: #b5544f;
    color: #fff;
  }
  .body {
    flex: 1;
    min-height: 0;
  }
  .agent-wrap {
    height: 100%;
  }
  .toast {
    position: fixed;
    bottom: 22px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--elevated);
    border: 1px solid color-mix(in srgb, var(--accent) 45%, var(--border));
    color: var(--text);
    border-radius: 999px;
    padding: 9px 18px;
    font-size: 13px;
    z-index: 80;
    box-shadow: var(--shadow-modal);
  }
  /* Data-loss notice: sticky, dismissable, distinct from the toast. */
  .store-warn {
    position: fixed;
    top: 44px;
    left: 50%;
    transform: translateX(-50%);
    max-width: min(680px, calc(100vw - 32px));
    display: flex;
    align-items: center;
    gap: 9px;
    background: var(--elevated);
    border: 1px solid #b4791f;
    color: var(--text);
    border-radius: 10px;
    padding: 10px 12px 10px 14px;
    font-size: 13px;
    line-height: 1.35;
    z-index: 90;
    box-shadow: var(--shadow-modal);
  }
  .store-warn > span {
    flex: 1;
  }
  .sw-dismiss {
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    color: var(--muted);
    cursor: pointer;
    padding: 2px;
    border-radius: 6px;
    flex-shrink: 0;
  }
  .sw-dismiss:hover {
    color: var(--text);
    background: var(--accent-soft);
  }
</style>
