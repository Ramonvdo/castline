// Variable helpers — the core of the feature. Variables are written as
// {{name}} in any template or SOP step. Pure string work, shared by the
// library view, the Fill & copy flow, profiles, and quick-open.

export const VAR_RE = /\{\{\s*([^{}]+?)\s*\}\}/g;

// ── Auto date/time tokens — {{today}}, {{now}}, with Make-style formats:
// {{today:YYYY-MM-DD}}, {{now:HH:mm}}, {{today:MMM D, YYYY}} …
// Resolved at copy time; never shown as fill fields or profile variables.

const MONTHS = ["January","February","March","April","May","June","July","August","September","October","November","December"];

/** Is this variable an auto-filled date/time token? */
export function isAutoVar(name) {
  const head = (name || "").split(":")[0].trim().toLowerCase();
  return head === "today" || head === "now";
}

/** Format `d` with YYYY YY MMMM MMM MM M DD D HH mm ss tokens (Make-style). */
export function formatDate(d, fmt) {
  const pad = (n) => String(n).padStart(2, "0");
  const map = {
    YYYY: String(d.getFullYear()),
    YY: String(d.getFullYear()).slice(-2),
    MMMM: MONTHS[d.getMonth()],
    MMM: MONTHS[d.getMonth()].slice(0, 3),
    MM: pad(d.getMonth() + 1),
    M: String(d.getMonth() + 1),
    DD: pad(d.getDate()),
    D: String(d.getDate()),
    HH: pad(d.getHours()),
    mm: pad(d.getMinutes()),
    ss: pad(d.getSeconds()),
  };
  return fmt.replace(/YYYY|YY|MMMM|MMM|MM|M|DD|D|HH|mm|ss/g, (t) => map[t]);
}

/** Resolve an auto token ("today", "now:HH:mm") to its formatted value. */
export function autoValue(name, now = new Date()) {
  const idx = name.indexOf(":");
  const head = (idx === -1 ? name : name.slice(0, idx)).trim().toLowerCase();
  const fmt = idx === -1 ? "" : name.slice(idx + 1).trim();
  if (head === "today") return formatDate(now, fmt || "YYYY-MM-DD");
  if (head === "now") return formatDate(now, fmt || "HH:mm");
  return null;
}

/** Ordered, de-duplicated list of {{variables}} found in `text` (auto tokens excluded). */
export function extractVars(text) {
  const seen = [];
  let m;
  VAR_RE.lastIndex = 0;
  while ((m = VAR_RE.exec(text || "")) !== null) {
    const name = m[1].trim();
    if (name && !isAutoVar(name) && !seen.includes(name)) seen.push(name);
  }
  return seen;
}

/**
 * Replace {{name}} with values[name]; leave untouched when there's no value.
 * {{today}}/{{now}} (with optional :format) always resolve, profile or not.
 */
export function applyVars(text, values) {
  return (text || "").replace(VAR_RE, (full, name) => {
    const n = name.trim();
    if (isAutoVar(n)) return autoValue(n) ?? full;
    const v = (values || {})[n];
    return v !== undefined && v !== "" ? v : full;
  });
}

/** All variables used by an item (a template, or every step of an SOP). */
export function itemVars(item) {
  if (!item) return [];
  if (item.kind === "sop") {
    const all = [];
    for (const s of item.steps || []) {
      for (const v of extractVars(s.text)) if (!all.includes(v)) all.push(v);
    }
    return all;
  }
  return extractVars(item.text);
}

/** The full plain text of an item (SOP steps joined with blank lines). */
export function itemPlainText(item) {
  return item.kind === "sop"
    ? (item.steps || []).map((s) => s.text).join("\n\n")
    : item.text || "";
}

/** Every variable used anywhere across all folders/items. */
export function allLibraryVars(folders) {
  const all = [];
  for (const f of folders || []) {
    for (const i of f.items || []) {
      for (const v of itemVars(i)) if (!all.includes(v)) all.push(v);
    }
  }
  return all;
}

/**
 * Group a set of variable names under the global layout (an ordered list of
 * { type:"splitter", label } | { type:"var", name } entries). Returns an ordered
 * list of { label, vars: [...] } groups. Any name not placed in the layout falls
 * into a trailing "Other" group. Presentation only — never affects values, so the
 * webhook/n8n/Make contract (keyed on variable names) is untouched by grouping.
 */
export function groupVarsByLayout(varNames, layout) {
  const names = new Set(varNames);
  const groups = [];
  let current = { label: "", vars: [] };
  const placed = new Set();

  for (const entry of layout || []) {
    if (entry.type === "splitter") {
      if (current.vars.length || current.label) groups.push(current);
      current = { label: entry.label || "", vars: [] };
    } else if (entry.type === "var" && names.has(entry.name) && !placed.has(entry.name)) {
      current.vars.push(entry.name);
      placed.add(entry.name);
    }
  }
  if (current.vars.length || current.label) groups.push(current);

  const leftover = varNames.filter((n) => !placed.has(n));
  if (leftover.length) groups.push({ label: "Other", vars: leftover, isOther: true });

  // Drop empty unlabelled leading group if nothing landed in it.
  return groups.filter((g) => g.vars.length || g.label);
}

/** A small emoji + label for an item's free-form `type`. */
export function typeMeta(type) {
  const t = (type || "").toLowerCase();
  const map = {
    prompt: { icon: "✦", label: "Prompt" },
    note: { icon: "📝", label: "Note" },
    email: { icon: "✉", label: "Email" },
    message: { icon: "💬", label: "Message" },
    snippet: { icon: "❯", label: "Snippet" },
    doc: { icon: "📄", label: "Doc" },
  };
  return map[t] || null;
}
