<script>
  import { onMount } from "svelte";
  import { getSettings, setConnectors, connectorSend, clipCopy } from "./api.js";
  import { allLibraryVars } from "./vars.js";
  import Icon from "./Icon.svelte";

  let { connectors = [], folders = [], flash, onSettings } = $props();

  let uid = 0;
  const nextUid = () => `c${++uid}`;

  // All variables used across the library — the fields Castline sends.
  let libVars = $derived(allLibraryVars(folders));

  function clone(list) {
    return (list || []).map((c) => ({ _uid: nextUid(), id: c.id || "", name: c.name || "", url: c.url || "" }));
  }
  let rc = $state(clone(connectors));
  let tests = $state({}); // _uid -> { status, body, error, sent }

  onMount(async () => {
    const s = await getSettings();
    rc = clone(s.connectors || []);
  });

  function addConnector() {
    rc = [...rc, { _uid: nextUid(), id: "", name: "New connector", url: "" }];
  }
  function removeConnector(i) {
    rc = rc.filter((_, idx) => idx !== i);
  }
  async function saveAll() {
    const list = rc.map((c) => ({ id: c.id, name: c.name, url: c.url }));
    const s = await setConnectors(list);
    rc = clone(s.connectors || []);
    onSettings(s);
    flash("Connectors saved");
  }

  function sampleValue(key) {
    const k = key.toLowerCase();
    if (k.includes("first")) return "Sam";
    if (k.includes("last")) return "Rivera";
    if (k.includes("email")) return "sam@example.com";
    if (k.includes("phone")) return "+1 555 0100";
    if (k.includes("company")) return "Acme";
    if (k.includes("name")) return "Sam Rivera";
    return "value";
  }
  function exampleJson() {
    const keys = libVars.length ? libVars : ["first_name", "last_name", "email"];
    const obj = {};
    for (const k of keys) obj[k] = sampleValue(k);
    return JSON.stringify(obj, null, 2);
  }
  function prettyBody(body) {
    try {
      return JSON.stringify(JSON.parse(body), null, 2);
    } catch {
      return body;
    }
  }
  async function copyText(text, msg) {
    await clipCopy(text);
    flash(msg);
  }
  async function runTest(c) {
    if (!c.url.trim()) {
      flash("Paste the webhook URL first");
      return;
    }
    const sent = exampleJson();
    tests = { ...tests, [c._uid]: { pending: true, sent } };
    try {
      const res = await connectorSend(c.url, sent);
      tests = { ...tests, [c._uid]: { status: res.status, body: res.body, sent, error: "" } };
    } catch (e) {
      tests = { ...tests, [c._uid]: { error: String(e), sent } };
    }
  }
</script>

<div class="view">
  <div class="view-head">
    <h2>Connectors</h2>
    <button class="btn" onclick={saveAll}>Save</button>
  </div>
  <p class="sub">
    Paste a <strong>Make / n8n webhook URL</strong>. Castline POSTs a profile's fields to it and reads the
    JSON your scenario returns (Make <em>Webhook response</em> / n8n <em>Respond to Webhook</em>) — so you
    map fields inside Make/n8n, and there's no tunnel or open port. Run a connector from a profile
    (<strong>Enrich</strong>) or via <strong>New from connector</strong> in Profiles.
  </p>

  {#each rc as c, i (c._uid)}
    {@const t = tests[c._uid]}
    <div class="panel">
      <div class="c-head">
        <input class="field cname" bind:value={c.name} placeholder="Connector name" />
        <button class="icon-btn" title="Delete connector" onclick={() => removeConnector(i)}><Icon name="trash" size={15} /></button>
      </div>

      <label class="fld">
        <span class="rlabel">Webhook URL (from Make / n8n)</span>
        <input class="field" bind:value={c.url} placeholder="https://hook.eu2.make.com/…" />
      </label>

      <div class="example">
        <div class="ex-head">
          <span class="rlabel">Castline sends these fields — map them in Make / n8n</span>
          <div class="ex-actions">
            <button class="ghost sm" onclick={() => copyText(exampleJson(), "Example payload copied")}><Icon name="copy" size={14} /> Copy</button>
            <button class="ghost sm" onclick={() => runTest(c)}><Icon name="check" size={14} /> Test</button>
          </div>
        </div>
        <pre class="code">{exampleJson()}</pre>

        {#if t?.pending}
          <p class="tiny">Sending…</p>
        {:else if t?.error}
          <p class="err">{t.error}</p>
        {:else if t}
          <div class="resp">
            <span class="rlabel">Response <span class="badge-n" class:ok={t.status >= 200 && t.status < 300}>{t.status}</span></span>
            {#if t.body}
              <pre class="code resp-body">{prettyBody(t.body)}</pre>
            {:else}
              <p class="tiny">No response body — add a "Webhook response" / "Respond to Webhook" module to return data.</p>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  {/each}

  <button class="ghost add" onclick={addConnector}><Icon name="plus" size={15} /> Add connector</button>
</div>

<style>
  .view {
    height: 100%;
    overflow-y: auto;
    padding: 22px 26px;
    max-width: 820px;
  }
  .view-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .view-head h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
  }
  .sub {
    color: var(--muted);
    font-size: 13px;
    line-height: 1.55;
    margin: 6px 0 16px;
  }
  .sub strong {
    color: var(--text);
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
  .c-head {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .cname {
    font-weight: 600;
  }
  .fld {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .rlabel {
    font-size: 12px;
    color: var(--muted);
  }
  .example {
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-top: 1px solid var(--border);
    padding-top: 12px;
  }
  .ex-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .ex-actions {
    display: flex;
    gap: 6px;
  }
  .code {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 12.5px;
    line-height: 1.5;
    background: var(--well);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.28);
    padding: 11px 12px;
    max-height: 200px;
    overflow: auto;
    white-space: pre;
    color: var(--text);
  }
  .resp {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .resp-body {
    max-height: 160px;
  }
  .badge-n {
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--muted);
    background: var(--elevated);
    border-radius: 5px;
    padding: 1px 6px;
  }
  .badge-n.ok {
    color: var(--on-accent);
    background: #6fb894;
  }
  .ghost.sm {
    padding: 6px 10px;
    font-size: 12px;
  }
  .tiny {
    font-size: 12px;
    color: var(--faint);
    margin: 0;
  }
  .err {
    color: #d98a8a;
    font-size: 12.5px;
    margin: 0;
  }
  .add {
    width: 100%;
    justify-content: center;
    padding: 11px;
    border-style: dashed;
  }
</style>
