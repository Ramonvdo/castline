<script>
  import { onMount } from "svelte";
  import Library from "./lib/Library.svelte";
  import QuickOpen from "./lib/QuickOpen.svelte";
  import Profiles from "./lib/Profiles.svelte";
  import Webhooks from "./lib/Webhooks.svelte";
  import Settings from "./lib/Settings.svelte";
  import FillCopy from "./lib/FillCopy.svelte";
  import Icon from "./lib/Icon.svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getLibrary, getProfiles, getSettings, onProfilesChanged } from "./lib/api.js";

  const appWindow = getCurrentWindow();
  const winMinimize = () => appWindow.minimize();
  const winToggleMax = () => appWindow.toggleMaximize();
  const winClose = () => appWindow.close();

  let library = $state({ folders: [] });
  let profiles = $state({ profiles: [], layout: [] });
  let settings = $state({ theme: "dark", receiver: { enabled: false, port: 8787, webhooks: [] } });

  let view = $state("library"); // library | profiles | webhooks | settings
  let quickOpen = $state(false);
  let fillItem = $state(null);
  let fillMode = $state("auto"); // auto | steps

  // Active profile (top-right selector). When set, card Copy auto-fills it.
  let activeProfileId = $state(null);
  let profileMenuOpen = $state(false);
  let activeProfile = $derived(profiles.profiles.find((p) => p.id === activeProfileId) || null);
  $effect(() => {
    if (activeProfileId && !profiles.profiles.some((p) => p.id === activeProfileId)) activeProfileId = null;
  });

  let toast = $state("");
  let toastTimer;
  function flash(m) {
    toast = m;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = ""), 2000);
  }

  onMount(async () => {
    settings = await getSettings();
    library = await getLibrary();
    profiles = await getProfiles();

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
        profileMenuOpen = false;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      un();
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
    { id: "webhooks", label: "Webhooks", icon: "plug" },
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
          <button class="navlink" class:active={view === n.id} onclick={() => (view = n.id)}>{n.label}</button>
        {/each}
      </nav>
    </div>

    <div class="right">
      <!-- Active profile selector -->
      <div class="profsel">
        <button class="profbtn" class:on={activeProfile} onclick={() => (profileMenuOpen = !profileMenuOpen)} title="Fill copies with this profile">
          <Icon name="user" size={14} />
          <span class="pl">{activeProfile ? activeProfile.name : "No profile"}</span>
          <Icon name="chevronDown" size={13} />
        </button>
        {#if profileMenuOpen}
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
          <div class="backdrop" onclick={() => (profileMenuOpen = false)}></div>
          <div class="menu">
            <button class="mi" class:sel={!activeProfileId} onclick={() => { activeProfileId = null; profileMenuOpen = false; }}>
              No profile
            </button>
            {#if profiles.profiles.length === 0}
              <div class="mi empty">No profiles yet</div>
            {:else}
              {#each profiles.profiles as p (p.id)}
                <button class="mi" class:sel={p.id === activeProfileId} onclick={() => { activeProfileId = p.id; profileMenuOpen = false; }}>
                  {p.name}
                </button>
              {/each}
            {/if}
          </div>
        {/if}
      </div>

      <button class="ghost search-cta" onclick={() => (quickOpen = true)}>
        <Icon name="command" size={15} /><span>Quick find</span><kbd>Ctrl K</kbd>
      </button>

      <div class="winctl">
        <button class="wbtn" title="Minimize" onclick={winMinimize}><Icon name="winMin" size={15} /></button>
        <button class="wbtn" title="Maximize" onclick={winToggleMax}><Icon name="winMax" size={13} /></button>
        <button class="wbtn danger" title="Close" onclick={winClose}><Icon name="close" size={15} /></button>
      </div>
    </div>
  </header>

  <main class="body">
    {#if view === "library"}
      <Library bind:library profiles={profiles.profiles} layout={profiles.layout || []} {activeProfile} {flash} onFill={openFill} />
    {:else if view === "profiles"}
      <Profiles profiles={profiles.profiles} layout={profiles.layout || []} folders={library.folders} {flash} onData={(d) => (profiles = d)} />
    {:else if view === "webhooks"}
      <Webhooks {settings} {flash} onSettings={(s) => (settings = s)} />
    {:else if view === "settings"}
      <Settings {flash} onLibraryData={(d) => (library = d)} onProfilesData={(d) => (profiles = d)} />
    {/if}
  </main>
</div>

{#if toast}<div class="toast">{toast}</div>{/if}

{#if quickOpen}
  <QuickOpen {library} {activeProfile} {flash} onFill={openFill} onClose={() => (quickOpen = false)} />
{/if}

{#if fillItem}
  <FillCopy item={fillItem} mode={fillMode} profiles={profiles.profiles} layout={profiles.layout || []} {activeProfile} {flash} onClose={() => (fillItem = null)} />
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
    transition: color 0.12s var(--ease), background 0.12s var(--ease);
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
    transition: background 0.12s var(--ease), color 0.12s var(--ease);
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
