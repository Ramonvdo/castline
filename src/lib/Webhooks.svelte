<script>
  import { onMount } from "svelte";
  import { getSettings, setReceiver, webhookStatus, webhookPreview, clipCopy } from "./api.js";
  import Icon from "./Icon.svelte";

  let { settings, flash, onSettings } = $props();

  let uid = 0;
  const nextUid = () => `wh${++uid}`;

  // Editable local copy of the receiver config.
  function clone(r) {
    const src = r || { enabled: false, port: 8787, webhooks: [] };
    return {
      enabled: !!src.enabled,
      port: src.port || 8787,
      webhooks: (src.webhooks || []).map((w) => ({
        _uid: nextUid(),
        id: w.id || "",
        name: w.name || "",
        path: w.path || "",
        token: w.token || "",
        name_template: w.name_template ?? "{{first_name}} {{last_name}}",
        mappings: (w.mappings || []).map((m) => ({ ...m })),
        passthrough: w.passthrough ?? true,
      })),
    };
  }
  let rc = $state(clone(settings.receiver));
  let activePort = $state(null);
  let tests = $state({}); // _uid -> { text, result, error }

  onMount(async () => {
    // Load fresh settings so the view is correct regardless of prop timing.
    const s = await getSettings();
    rc = clone(s.receiver);
    activePort = await webhookStatus();
  });

  function addWebhook() {
    rc.webhooks = [
      ...rc.webhooks,
      { _uid: nextUid(), id: "", name: "New webhook", path: "", token: "", name_template: "{{first_name}} {{last_name}}", mappings: [{ from: "first_name", to: "firstName" }], passthrough: true },
    ];
  }
  function removeWebhook(i) {
    rc.webhooks = rc.webhooks.filter((_, idx) => idx !== i);
  }
  function addMapping(wh) {
    wh.mappings = [...wh.mappings, { from: "", to: "" }];
    rc = rc;
  }
  function removeMapping(wh, i) {
    wh.mappings = wh.mappings.filter((_, idx) => idx !== i);
    rc = rc;
  }

  async function saveAll() {
    // Strip transient _uid before sending; the backend fills ids/tokens/paths.
    const config = {
      enabled: !!rc.enabled,
      port: Number(rc.port) || 8787,
      webhooks: rc.webhooks.map((w) => ({
        id: w.id,
        name: w.name,
        path: w.path,
        token: w.token,
        name_template: w.name_template,
        mappings: w.mappings.filter((m) => m.from.trim()),
        passthrough: !!w.passthrough,
      })),
    };
    const s = await setReceiver(config);
    rc = clone(s.receiver);
    onSettings(s);
    activePort = await webhookStatus();
    flash(config.enabled ? "Webhooks saved · receiver running" : "Webhooks saved");
  }

  function endpoint(wh) {
    if (!wh.path || !wh.token) return "";
    return `http://127.0.0.1:${rc.port}/hook/${wh.path}?token=${wh.token}`;
  }
  async function copyEndpoint(wh) {
    const url = endpoint(wh);
    if (url) {
      await clipCopy(url);
      flash("Endpoint URL copied");
    }
  }

  function sampleValue(key) {
    const k = key.toLowerCase();
    if (k.includes("first")) return "Sam";
    if (k.includes("last")) return "Rivera";
    if (k.includes("email")) return "sam@example.com";
    if (k.includes("phone")) return "+1 555 0100";
    if (k.includes("company")) return "Acme";
    return "sample";
  }
  function exampleJson(wh) {
    const keys = new Set();
    for (const m of wh.mappings) if (m.from.trim()) keys.add(m.from.trim());
    // include name-template placeholders too
    const re = /\{\{\s*([^{}]+?)\s*\}\}/g;
    let m;
    while ((m = re.exec(wh.name_template || "")) !== null) keys.add(m[1].trim());
    if (keys.size === 0) {
      keys.add("first_name");
      keys.add("email");
    }
    const obj = {};
    for (const k of keys) obj[k] = sampleValue(k);
    return JSON.stringify(obj, null, 2);
  }
  function ensureTest(wh) {
    if (!tests[wh._uid]) tests = { ...tests, [wh._uid]: { text: exampleJson(wh), result: null, error: "" } };
    return tests[wh._uid];
  }
  async function runTest(wh) {
    const t = ensureTest(wh);
    try {
      const profile = await webhookPreview(
        { id: wh.id || "x", name: wh.name, path: wh.path || "x", token: wh.token || "x", name_template: wh.name_template, mappings: wh.mappings.filter((m) => m.from.trim()), passthrough: !!wh.passthrough },
        t.text,
      );
      tests = { ...tests, [wh._uid]: { ...t, result: profile, error: "" } };
    } catch (e) {
      tests = { ...tests, [wh._uid]: { ...t, result: null, error: String(e) } };
    }
  }
  function setTestText(wh, text) {
    const t = ensureTest(wh);
    tests = { ...tests, [wh._uid]: { ...t, text } };
  }
</script>

<div class="view">
  <div class="view-head">
    <h2>Webhooks</h2>
    <button class="btn" onclick={saveAll}>Save &amp; apply</button>
  </div>
  <p class="sub">
    Turn form submissions (Calendly, Typeform, a CRM…) into profiles automatically. localhost isn't
    reachable from the internet, so point a tunnel (ngrok / Cloudflare) or a relay (Make / n8n / Zapier)
    at an endpoint below. No tunnel? Use <strong>Paste JSON</strong> in Profiles.
  </p>

  <div class="panel receiver">
    <label class="toggle">
      <input type="checkbox" bind:checked={rc.enabled} />
      <span>Enable local receiver</span>
    </label>
    <label class="portf">Port<input class="field" type="number" min="1024" max="65535" bind:value={rc.port} /></label>
    <span class="status">
      <span class="dot" class:live={activePort}></span>
      {activePort ? `listening on 127.0.0.1:${activePort}` : "stopped"}
    </span>
  </div>

  {#each rc.webhooks as wh, i (wh._uid)}
    {@const t = tests[wh._uid]}
    <div class="panel wh">
      <div class="wh-head">
        <input class="field wname" bind:value={wh.name} placeholder="Webhook name" />
        <button class="icon-btn" title="Delete webhook" onclick={() => removeWebhook(i)}><Icon name="trash" size={15} /></button>
      </div>

      <div class="two">
        <label>Path (URL slug)<input class="field" bind:value={wh.path} placeholder="calendly" /></label>
        <label>Profile name template<input class="field" bind:value={wh.name_template} placeholder="{'{{first_name}} {{last_name}}'}" /></label>
      </div>

      {#if endpoint(wh)}
        <div class="endpoint">
          <code class="url">{endpoint(wh)}</code>
          <button class="ghost" onclick={() => copyEndpoint(wh)}><Icon name="copy" size={14} /> Copy</button>
        </div>
      {:else}
        <p class="tiny">Save to generate this webhook's secret endpoint URL.</p>
      {/if}

      <div class="maps">
        <span class="maps-head">Field mapping — incoming JSON key → <code>{"{{variable}}"}</code></span>
        {#each wh.mappings as m, mi (mi)}
          <div class="map-row">
            <input class="field" bind:value={m.from} placeholder="first_name" />
            <Icon name="arrowRight" size={15} />
            <input class="field" bind:value={m.to} placeholder="firstName" />
            <button class="icon-btn" title="Remove" onclick={() => removeMapping(wh, mi)}><Icon name="close" size={14} /></button>
          </div>
        {/each}
        <button class="ghost sm" onclick={() => addMapping(wh)}><Icon name="plus" size={14} /> Add mapping</button>
      </div>

      <label class="toggle">
        <input type="checkbox" bind:checked={wh.passthrough} />
        <span>Pass unmapped fields through as variables of the same name</span>
      </label>

      <div class="test">
        <div class="test-head">
          <span class="maps-head">Example payload — test the mapping</span>
          <button class="ghost sm" onclick={() => runTest(wh)}><Icon name="check" size={14} /> Test</button>
        </div>
        <textarea class="field mono" rows="4" value={t ? t.text : exampleJson(wh)} oninput={(e) => setTestText(wh, e.target.value)}></textarea>
        {#if t?.error}
          <p class="err">{t.error}</p>
        {:else if t?.result}
          <div class="result">
            <div class="rname">→ profile “{t.result.name}”</div>
            <div class="rvals">
              {#each Object.entries(t.result.values) as [k, v]}
                <span class="rv"><span class="vchip">{k}</span> {v}</span>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/each}

  <button class="ghost add" onclick={addWebhook}><Icon name="plus" size={15} /> Add webhook</button>
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
  .receiver {
    flex-direction: row;
    align-items: center;
    gap: 18px;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    cursor: pointer;
  }
  .portf {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
    color: var(--muted);
  }
  .portf .field {
    width: 100px;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--muted);
    margin-left: auto;
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--faint);
  }
  .dot.live {
    background: #6fb894;
    box-shadow: 0 0 0 3px color-mix(in srgb, #6fb894 22%, transparent);
  }
  .wh-head {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .wname {
    font-weight: 600;
  }
  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .two label,
  label {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
    color: var(--muted);
  }
  .endpoint {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .url {
    font-family: var(--font-mono);
    font-size: 12px;
    background: var(--well);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 7px 9px;
    flex: 1;
    min-width: 0;
    overflow-x: auto;
    white-space: nowrap;
    color: var(--text);
  }
  .tiny,
  .ttiny {
    font-size: 12px;
    color: var(--faint);
    margin: 0;
  }
  .maps {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .maps-head {
    font-size: 12px;
    color: var(--muted);
  }
  .map-row {
    display: grid;
    grid-template-columns: 1fr auto 1fr auto;
    align-items: center;
    gap: 8px;
    color: var(--muted);
  }
  .ghost.sm {
    padding: 6px 10px;
    font-size: 12px;
    align-self: flex-start;
  }
  .test {
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-top: 1px solid var(--border);
    padding-top: 12px;
  }
  .test-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 12.5px;
  }
  .err {
    color: #d98a8a;
    font-size: 12.5px;
    margin: 0;
  }
  .result {
    background: var(--well);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .rname {
    font-weight: 600;
    font-size: 13px;
  }
  .rvals {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 12px;
  }
  .rv {
    font-size: 12.5px;
    color: var(--text);
  }
  .add {
    width: 100%;
    justify-content: center;
    padding: 11px;
    border-style: dashed;
  }
</style>
