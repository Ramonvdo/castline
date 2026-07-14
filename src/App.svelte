<script>
  import { onMount } from "svelte";
  import Library from "./lib/Library.svelte";
  import QuickOpen from "./lib/QuickOpen.svelte";
  import Profiles from "./lib/Profiles.svelte";
  import Settings from "./lib/Settings.svelte";
  import FillCopy from "./lib/FillCopy.svelte";
  import Icon from "./lib/Icon.svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getLibrary, getProfiles, getSettings, applyAccent, onProfilesChanged } from "./lib/api.js";

  const appWindow = getCurrentWindow();
  const winMinimize = () => appWindow.minimize();
  const winToggleMax = () => appWindow.toggleMaximize();
  const winClose = () => appWindow.close();

  let library = $state({ folders: [] });
  let profiles = $state({ profiles: [], layout: [] });
  let settings = $state({ accent: "#4f8cff", theme: "dark", webhook: {} });

  let quickOpen = $state(false);
  let showProfiles = $state(false);
  let showSettings = $state(false);
  let fillItem = $state(null);

  let toast = $state("");
  let toastTimer;
  function flash(m) {
    toast = m;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = ""), 2000);
  }

  onMount(async () => {
    settings = await getSettings();
    applyAccent(settings.accent);
    library = await getLibrary();
    profiles = await getProfiles();

    // Webhook-created profiles arrive live from the Rust side.
    const un = await onProfilesChanged(async () => {
      profiles = await getProfiles();
      flash("New profile received");
    });

    const onKey = (e) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        quickOpen = true;
      } else if (e.key === "Escape") {
        quickOpen = false;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      un();
    };
  });

  function openFill(item) {
    quickOpen = false;
    fillItem = item;
  }
</script>

<div class="shell">
  <header class="titlebar" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <span class="logo"><Icon name="sparkle" size={17} fill={true} /></span>
      <span class="wordmark">Castline</span>
    </div>
    <div class="top-actions">
      <button class="ghost search-cta" onclick={() => (quickOpen = true)}>
        <Icon name="command" size={15} /><span>Quick find</span><kbd>Ctrl K</kbd>
      </button>
      <button class="ghost with-ic" onclick={() => (showProfiles = true)}>
        <Icon name="user" size={15} /><span>Profiles</span><span class="badge-n">{profiles.profiles.length}</span>
      </button>
      <button class="ghost with-ic" onclick={() => (showSettings = true)}>
        <Icon name="sliders" size={15} /><span>Settings</span>
      </button>
      <div class="winctl">
        <button class="wbtn" title="Minimize" onclick={winMinimize}><Icon name="winMin" size={15} /></button>
        <button class="wbtn" title="Maximize" onclick={winToggleMax}><Icon name="winMax" size={13} /></button>
        <button class="wbtn danger" title="Close" onclick={winClose}><Icon name="close" size={15} /></button>
      </div>
    </div>
  </header>

  <main class="body">
    <Library bind:library profiles={profiles.profiles} {flash} onFill={openFill} />
  </main>
</div>

{#if toast}<div class="toast">{toast}</div>{/if}

{#if quickOpen}
  <QuickOpen {library} {flash} onFill={openFill} onClose={() => (quickOpen = false)} />
{/if}

{#if fillItem}
  <FillCopy item={fillItem} profiles={profiles.profiles} layout={profiles.layout || []} {flash} onClose={() => (fillItem = null)} />
{/if}

{#if showProfiles}
  <Profiles
    profiles={profiles.profiles}
    layout={profiles.layout || []}
    folders={library.folders}
    {flash}
    onData={(d) => (profiles = d)}
    onClose={() => (showProfiles = false)}
  />
{/if}

{#if showSettings}
  <Settings
    {settings}
    {flash}
    onLibraryData={(d) => (library = d)}
    onProfilesData={(d) => (profiles = d)}
    onSettings={(s) => (settings = s)}
    onClose={() => (showSettings = false)}
  />
{/if}

<style>
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
    padding: 0 0 0 16px;
    border-bottom: 1px solid var(--border);
    background: var(--sheen), var(--surface);
    box-shadow: var(--edge);
    flex-shrink: 0;
    user-select: none;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
  }
  .logo {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: 7px;
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 30%, var(--border));
  }
  .wordmark {
    font-weight: 700;
    font-size: 15px;
    letter-spacing: -0.01em;
  }
  .badge-n {
    font-size: 11px;
    color: var(--muted);
    background: var(--elevated);
    border-radius: 999px;
    padding: 1px 7px;
  }
  .top-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-right: 8px;
    height: 100%;
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
    transition: background 0.12s var(--ease), color 0.12s var(--ease);
  }
  .wbtn:hover {
    background: var(--elevated);
    color: var(--text);
  }
  .wbtn.danger:hover {
    background: #e5484d;
    color: #fff;
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
  .body {
    flex: 1;
    min-height: 0;
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
</style>
