<script>
  // The Agent tab: an embedded terminal running the user's own `claude` CLI in
  // the Castline data dir, where the app keeps a generated CLAUDE.md describing
  // the library/profiles + the local write endpoint. Stays mounted across tab
  // switches (App.svelte hides it with CSS) so the TUI never loses its screen.
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";
  import {
    aiStatus,
    aiStart,
    aiInput,
    aiResize,
    aiStop,
    refreshAgentContext,
    onAiOutput,
    onAiExit,
  } from "./api.js";
  import Icon from "./Icon.svelte";

  let { active = false } = $props();

  let host; // terminal container (bind:this)
  let term, fit, ro, offOut, offExit;
  let status = $state(null); // { installed, path, workspace, running }
  let running = $state(false);
  let starting = $state(false);
  let autoStarted = false; // only auto-launch claude once, on first tab open
  let note = $state("");
  let noteTimer;
  function flash(msg) {
    note = msg;
    clearTimeout(noteTimer);
    noteTimer = setTimeout(() => (note = ""), 2800);
  }

  const cssVar = (n) => getComputedStyle(document.documentElement).getPropertyValue(n).trim();
  const termTheme = () => ({
    background: cssVar("--well") || cssVar("--bg") || "#070b14",
    foreground: cssVar("--text") || "#e9f0ff",
    cursor: cssVar("--accent") || "#5f9cf2",
    cursorAccent: cssVar("--bg") || "#0a0f1a",
    selectionBackground: (cssVar("--accent") || "#5f9cf2") + "55",
  });

  onMount(async () => {
    status = await aiStatus();
    term = new Terminal({
      fontSize: 13,
      fontFamily: "Consolas, 'Cascadia Mono', 'Courier New', monospace",
      scrollback: 5000,
      cursorBlink: true,
      theme: termTheme(),
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host);
    safeFit();
    term.onData((d) => {
      if (running) aiInput(d).catch(() => {});
    });
    ro = new ResizeObserver(() => {
      // Fitting while the tab is display:none computes 0×0 — skip until visible.
      if (!host || !host.offsetParent) return;
      safeFit();
      if (running) aiResize(term.rows, term.cols).catch(() => {});
    });
    ro.observe(host);
    offOut = await onAiOutput((chunk) => term.write(chunk));
    offExit = await onAiExit((code) => {
      running = false;
      term.write(`\r\n\x1b[2m[claude exited (${code}) — press Start to relaunch]\x1b[0m\r\n`);
    });
    if (status?.running) {
      // A session survived a remount (e.g. dev reload) — reattach, don't respawn.
      running = true;
      autoStarted = true;
      aiResize(term.rows, term.cols).catch(() => {});
    }
    // Don't auto-launch on mount: the component is always mounted, so that would
    // spawn claude on app boot. We start lazily on first Agent-tab open (below).
  });

  function safeFit() {
    try {
      if (host && host.offsetParent) fit.fit();
    } catch {}
  }

  // Tab became visible: the container had display:none before, so re-fit + focus.
  // Also the first time it opens, lazily launch claude (if it's installed).
  $effect(() => {
    if (active && fit) {
      requestAnimationFrame(() => {
        safeFit();
        if (running) {
          aiResize(term.rows, term.cols).catch(() => {});
          term.focus();
        } else if (status?.installed && !starting && !autoStarted) {
          autoStarted = true;
          start();
        }
      });
    }
  });

  async function start() {
    if (starting) return;
    starting = true;
    term.clear();
    safeFit();
    try {
      await aiStart(term.rows, term.cols);
      running = true;
      term.focus();
    } catch (e) {
      flash(String(e));
      term.write(`\r\n\x1b[31m[failed to start claude: ${String(e)}]\x1b[0m\r\n`);
    } finally {
      starting = false;
    }
  }

  async function restart() {
    await aiStop().catch(() => {});
    running = false;
    await start();
  }

  async function refresh() {
    try {
      await refreshAgentContext();
      flash("CLAUDE.md refreshed");
    } catch (e) {
      flash(String(e));
    }
  }

  onDestroy(() => {
    ro?.disconnect();
    offOut?.();
    offExit?.();
    term?.dispose();
  });
</script>

<div class="ai-root">
  <div class="toolbar">
    <span class="dot" class:on={running}></span>
    <span class="ws" title={status?.workspace || ""}>{status?.workspace || "AI agent"}</span>
    {#if note}<span class="flash">{note}</span>{/if}
    <button class="ghost" onclick={refresh}><Icon name="reveal" size={14} /> Refresh context</button>
    <button class="ghost" onclick={restart} disabled={!status?.installed || starting}>
      <Icon name="play" size={14} /> {running ? "Restart" : "Start"}
    </button>
  </div>

  {#if status && !status.installed}
    <div class="empty-state">
      <h3>Claude Code not found</h3>
      <p>
        The Agent tab embeds your own <code>claude</code> CLI and launches it in Castline's data folder,
        where the app keeps a <code>CLAUDE.md</code> describing your <b>library</b> and <b>profiles</b> and a
        local write endpoint — so Claude can research contacts and <b>create or enrich profiles</b> for you.
      </p>
      <p>
        Install it from <b>claude.ai/code</b> (or
        <code>npm install -g @anthropic-ai/claude-code</code>), then reopen this tab. A custom binary path
        can be set later in Settings.
      </p>
    </div>
  {/if}

  <div class="term" bind:this={host} style:display={status && !status.installed ? "none" : "block"}></div>
</div>

<style>
  .ai-root {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 16px 18px 18px;
    min-height: 0;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--faint);
    flex: none;
  }
  .dot.on {
    background: #6fb894;
    box-shadow: 0 0 0 3px color-mix(in srgb, #6fb894 22%, transparent);
  }
  .ws {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
    font-size: 12.5px;
    font-family: var(--font-mono);
  }
  .flash {
    color: var(--accent-strong);
    font-size: 13px;
  }
  .ghost {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--elevated);
    color: var(--text);
    cursor: pointer;
    font-size: 13px;
    transition: border-color 0.12s var(--ease), color 0.12s var(--ease);
  }
  .ghost:hover:not(:disabled) {
    border-color: var(--border-strong);
    color: var(--accent-strong);
  }
  .ghost:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .term {
    flex: 1;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    background: var(--well);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.3);
    padding: 8px;
  }
  .empty-state {
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius);
    padding: 22px 24px;
    color: var(--muted);
    font-size: 14px;
    line-height: 1.6;
    margin-bottom: 12px;
    max-width: 640px;
  }
  .empty-state h3 {
    margin: 0 0 8px;
    color: var(--text);
  }
  .empty-state code {
    font-family: var(--font-mono);
    font-size: 12.5px;
    background: var(--well);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 1px 5px;
  }
</style>
