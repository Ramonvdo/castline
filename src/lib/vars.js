// Variable helpers — the core of the feature. Variables are written as
// {{name}} in any template or SOP step. Pure string work, shared by the
// library view, the Fill & copy flow, profiles, and quick-open.

export const VAR_RE = /\{\{\s*([^{}]+?)\s*\}\}/g;

/** Ordered, de-duplicated list of {{variables}} found in `text`. */
export function extractVars(text) {
  const seen = [];
  let m;
  VAR_RE.lastIndex = 0;
  while ((m = VAR_RE.exec(text || "")) !== null) {
    const name = m[1].trim();
    if (name && !seen.includes(name)) seen.push(name);
  }
  return seen;
}

/** Replace {{name}} with values[name]; leave untouched when there's no value. */
export function applyVars(text, values) {
  return (text || "").replace(VAR_RE, (full, name) => {
    const v = values[name.trim()];
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
