<script>
  import { onMount } from "svelte";
  import {
    getSettings,
    setConnectors,
    connectorSend,
    clipCopy,
    httpStatus,
    setHttpEndpoint,
  } from "./api.js";
  import { allLibraryVars } from "./vars.js";
  import Icon from "./Icon.svelte";

  let { connectors = [], folders = [], flash, onSettings } = $props();

  let uid = 0;
  const nextUid = () => `c${++uid}`;

  // All variables used across the library — the fields Castline sends.
  let libVars = $derived(allLibraryVars(folders));

  function clone(list) {
    return (list || []).map((c) => ({
      _uid: nextUid(),
      id: c.id || "",
      name: c.name || "",
      url: c.url || "",
    }));
  }
  let rc = $state(clone(connectors));
  let tests = $state({}); // _uid -> { status, body, error, sent }

  // ── Inbound HTTP endpoint ──
  let http = $state({
    enabled: false,
    port: 8787,
    active: false,
    token: "",
    baseUrl: "http://127.0.0.1:8787",
  });
  let httpTests = $state({}); // action id -> { status, body, error, pending }
  const HTTP_ACTIONS = [
    {
      id: "create-profile",
      title: "Create profile",
      desc: "Makes a brand-new profile from the JSON body.",
    },
    {
      id: "update-profile",
      title: "Update / enrich profile",
      desc: "Merges the JSON into an existing profile matched by name or email.",
    },
  ];

  onMount(async () => {
    const s = await getSettings();
    rc = clone(s.connectors || []);
    http = await httpStatus();
  });

  async function toggleHttp() {
    await setHttpEndpoint(!http.enabled, Number(http.port) || 8787);
    http = await httpStatus();
    flash(http.enabled ? "HTTP endpoint on" : "HTTP endpoint off");
  }
  async function applyPort() {
    await setHttpEndpoint(http.enabled, Number(http.port) || 8787);
    http = await httpStatus();
    flash("Port saved");
  }
  const actionUrl = (id) => `${http.baseUrl}/api/${id}`;
  const headerLines = () =>
    `Content-Type: application/json\nAuthorization: Bearer ${http.token}`;
  function actionBody(id) {
    const obj = JSON.parse(exampleJson());
    // Update needs an identifier — make sure a name (or email) is present + first.
    if (id === "update-profile" && !obj.name && !obj.email) {
      return JSON.stringify(
        { name: obj.full_name || "Sam Rivera", ...obj },
        null,
        2,
      );
    }
    return JSON.stringify(obj, null, 2);
  }
  async function testHttp(id) {
    if (!http.enabled) {
      flash("Enable the endpoint first");
      return;
    }
    httpTests = { ...httpTests, [id]: { pending: true } };
    try {
      const url = `${actionUrl(id)}?token=${encodeURIComponent(http.token)}`;
      const res = await connectorSend(url, actionBody(id));
      httpTests = {
        ...httpTests,
        [id]: { status: res.status, body: res.body, error: "" },
      };
    } catch (e) {
      httpTests = { ...httpTests, [id]: { error: String(e) } };
    }
  }

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
    const keys = libVars.length
      ? libVars
      : ["first_name", "last_name", "email"];
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
      tests = {
        ...tests,
        [c._uid]: { status: res.status, body: res.body, sent, error: "" },
      };
    } catch (e) {
      tests = { ...tests, [c._uid]: { error: String(e), sent } };
    }
  }
</script>

<div class="view">
  <div class="view-head">
    <h3>
      Webhooks <span class="tag">outbound</span>
    </h3>
    <button class="btn" onclick={saveAll}>Save</button>
  </div>
  <p class="sub"></p>

  {#each rc as c, i (c._uid)}
    {@const t = tests[c._uid]}
    <div class="panel">
      <div class="c-head">
        <input
          class="field cname"
          bind:value={c.name}
          placeholder="Connector name"
        />
        <button
          class="icon-btn"
          title="Delete connector"
          onclick={() => removeConnector(i)}
          ><Icon name="trash" size={15} /></button
        >
      </div>

      <label class="fld">
        <span class="rlabel">Webhook URL</span>
        <input
          class="field"
          bind:value={c.url}
          placeholder="https://hook.eu2.make.com/…"
        />
      </label>

      <div class="example">
        <div class="ex-head">
          <span class="rlabel"
            >Castline sends these fields — map them in Make / n8n</span
          >
          <div class="ex-actions">
            <button
              class="ghost sm"
              onclick={() => copyText(exampleJson(), "Example payload copied")}
              ><Icon name="copy" size={14} /> Copy</button
            >
            <button class="ghost sm" onclick={() => runTest(c)}
              ><Icon name="check" size={14} /> Test</button
            >
          </div>
        </div>
        <pre class="code">{exampleJson()}</pre>

        {#if t?.pending}
          <p class="tiny">Sending…</p>
        {:else if t?.error}
          <p class="err">{t.error}</p>
        {:else if t}
          <div class="resp">
            <span class="rlabel"
              >Response <span
                class="badge-n"
                class:ok={t.status >= 200 && t.status < 300}>{t.status}</span
              ></span
            >
            {#if t.body}
              <pre class="code resp-body">{prettyBody(t.body)}</pre>
            {:else}
              <p class="tiny">
                No response body — add a "Webhook response" / "Respond to
                Webhook" module to return data.
              </p>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  {/each}

  <button class="ghost add" onclick={addConnector}
    ><Icon name="plus" size={15} /> Add connector</button
  >

  <!-- ── Inbound HTTP endpoint ── -->
  <section class="http">
    <div class="http-head">
      <div class="hh-left">
        <h3>
          HTTP endpoint <span class="tag">inbound</span>
          <span
            class="dot"
            class:on={http.active}
            title={http.active ? "listening" : "off"}
          ></span>
        </h3>
        <p class="sub2"></p>
      </div>
      <label class="switch">
        <input type="checkbox" checked={http.enabled} onchange={toggleHttp} />
        <span class="track"><span class="knob"></span></span>
        <span class="swlabel">{http.enabled ? "On" : "Off"}</span>
      </label>
    </div>

    {#if http.enabled}
      <div class="portrow">
        <span class="rlabel">Port</span>
        <input
          class="field port"
          type="number"
          bind:value={http.port}
          min="1024"
          max="65535"
        />
        <button class="ghost sm" onclick={applyPort}>Apply</button>
      </div>

      {#each HTTP_ACTIONS as a}
        {@const ht = httpTests[a.id]}
        <div class="action">
          <div class="a-head">
            <span class="a-title">{a.title}</span>
            <button class="ghost sm" onclick={() => testHttp(a.id)}
              ><Icon name="check" size={14} /> Test locally</button
            >
          </div>
          <p class="tiny">
            {a.desc}
            <span class="paste"
              >Paste this into a Make HTTP module (or n8n HTTP Request node):</span
            >
          </p>

          <div class="kv">
            <span class="k">Method</span>
            <code class="v">POST</code>
            <button
              class="icon-btn xs"
              title="Copy"
              onclick={() => copyText("POST", "Method copied")}
              ><Icon name="copy" size={13} /></button
            >
          </div>
          <div class="kv">
            <span class="k">URL</span>
            <code class="v url">{actionUrl(a.id)}</code>
            <button
              class="icon-btn xs"
              title="Copy"
              onclick={() => copyText(actionUrl(a.id), "URL copied")}
              ><Icon name="copy" size={13} /></button
            >
          </div>
          <div class="kv">
            <span class="k">Headers</span>
            <code class="v pre">{headerLines()}</code>
            <button
              class="icon-btn xs"
              title="Copy"
              onclick={() => copyText(headerLines(), "Headers copied")}
              ><Icon name="copy" size={13} /></button
            >
          </div>
          <div class="kv col">
            <div class="kv-top">
              <span class="k">Body · Raw / JSON</span>
              <button
                class="ghost sm"
                onclick={() => copyText(actionBody(a.id), "Body copied")}
                ><Icon name="copy" size={13} /> Copy</button
              >
            </div>
            <pre class="code">{actionBody(a.id)}</pre>
          </div>

          {#if ht?.pending}
            <p class="tiny">Sending…</p>
          {:else if ht?.error}
            <p class="err">{ht.error}</p>
          {:else if ht}
            <div class="resp">
              <span class="rlabel"
                >Response <span
                  class="badge-n"
                  class:ok={ht.status >= 200 && ht.status < 300}
                  >{ht.status}</span
                ></span
              >
              {#if ht.body}<pre class="code resp-body">{prettyBody(
                    ht.body,
                  )}</pre>{/if}
            </div>
          {/if}
        </div>
      {/each}

      <div class="reach">
        <Icon name="info" size={15} />
        <p class="tiny">
          The endpoint binds <code>127.0.0.1</code> and is token-gated. A
          <strong>self-hosted n8n</strong>
          on this machine/LAN can call it directly. <strong>Make cloud</strong>
          (or any internet scenario) can't see localhost — run a tunnel (<strong
            >ngrok</strong
          >
          / <strong>Cloudflare Tunnel</strong>) and use that URL in place of
          <code>127.0.0.1:{http.port}</code>.
        </p>
      </div>
    {:else}
      <p class="tiny off-hint">
        Turn it on to get a token and the exact HTTP-module config for Create /
        Update profile.
      </p>
    {/if}
  </section>
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

  /* ── Inbound HTTP endpoint ── */
  .http {
    margin-top: 30px;
    padding-top: 22px;
    border-top: 1px solid var(--border-strong);
  }
  .http-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }
  .hh-left h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 700;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .tag {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
    background: var(--elevated);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 1px 6px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--faint);
  }
  .dot.on {
    background: #6fb894;
    box-shadow: 0 0 0 3px color-mix(in srgb, #6fb894 22%, transparent);
  }
  .sub2 {
    color: var(--muted);
    font-size: 13px;
    line-height: 1.55;
    margin: 6px 0 0;
    max-width: 560px;
  }
  .sub2 strong {
    color: var(--text);
  }
  /* Toggle switch */
  .switch {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    flex-shrink: 0;
    user-select: none;
  }
  .switch input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }
  .track {
    width: 38px;
    height: 22px;
    border-radius: 999px;
    background: var(--well);
    border: 1px solid var(--border-strong);
    position: relative;
    transition:
      background 0.15s var(--ease),
      border-color 0.15s var(--ease);
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--muted);
    transition:
      transform 0.15s var(--ease),
      background 0.15s var(--ease);
  }
  .switch input:checked + .track {
    background: var(--btn-accent);
    border-color: color-mix(in srgb, var(--accent) 55%, #000);
  }
  .switch input:checked + .track .knob {
    transform: translateX(16px);
    background: var(--on-accent);
  }
  .swlabel {
    font-size: 12px;
    color: var(--muted);
    min-width: 20px;
  }
  .portrow {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 16px;
  }
  .port {
    width: 110px;
  }
  .action {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--edge);
    padding: 14px;
    margin-top: 14px;
    display: flex;
    flex-direction: column;
    gap: 9px;
  }
  .a-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .a-title {
    font-weight: 600;
    font-size: 14px;
  }
  .paste {
    color: var(--faint);
  }
  .kv {
    display: grid;
    grid-template-columns: 74px 1fr auto;
    align-items: center;
    gap: 10px;
  }
  .kv.col {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
  }
  .kv-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .k {
    font-size: 12px;
    color: var(--muted);
  }
  .v {
    font-family: var(--font-mono);
    font-size: 12.5px;
    color: var(--text);
    background: var(--well);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 7px 10px;
    overflow-x: auto;
    white-space: nowrap;
  }
  .v.pre {
    white-space: pre;
    line-height: 1.5;
  }
  .v.url {
    color: var(--accent-strong);
  }
  .icon-btn.xs {
    padding: 5px;
  }
  .reach {
    display: flex;
    gap: 9px;
    align-items: flex-start;
    margin-top: 14px;
    padding: 11px 12px;
    background: var(--accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent) 28%, var(--border));
    border-radius: var(--radius-sm);
    color: var(--accent-strong);
  }
  .reach :global(.ic) {
    margin-top: 1px;
    flex-shrink: 0;
  }
  .reach .tiny {
    color: var(--muted);
  }
  .reach code,
  .sub2 code {
    font-family: var(--font-mono);
    font-size: 12px;
    background: var(--well);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 4px;
    color: var(--text);
  }
  .off-hint {
    margin-top: 14px;
  }
</style>
