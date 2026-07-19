<script>
  // A small, dependency-free icon set. Stroke-based, 24px grid, 1.5 weight,
  // currentColor — so icons inherit text colour and stay crisp at any size.
  let { name, size = 18, fill = false, strokeWidth = 1.5 } = $props();

  // Inner SVG markup per icon (trusted, static — safe for {@html}).
  const ICONS = {
    search: '<circle cx="11" cy="11" r="7"/><path d="M21 21l-4.3-4.3"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    star: '<path d="M12 3.2l2.6 5.3 5.8.8-4.2 4.1 1 5.8-5.2-2.7-5.2 2.7 1-5.8L3.6 9.3l5.8-.8z"/>',
    edit: '<path d="M12 20h9"/><path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4z"/>',
    trash: '<path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6M10 11v6M14 11v6"/>',
    copy: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/>',
    folder: '<path d="M3 7a2 2 0 0 1 2-2h3.5l2 2H19a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
    folderOpen: '<path d="M3 7a2 2 0 0 1 2-2h3.5l2 2H19a2 2 0 0 1 2 2M3 7v10a2 2 0 0 0 2 2h13a2 2 0 0 0 1.9-1.4L22 11H6.5a2 2 0 0 0-1.9 1.4z"/>',
    layers: '<path d="M12 3l9 5-9 5-9-5z"/><path d="M3 13l9 5 9-5"/>',
    sliders: '<path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6"/>',
    user: '<path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>',
    close: '<path d="M18 6L6 18M6 6l12 12"/>',
    chevronUp: '<path d="M6 15l6-6 6 6"/>',
    chevronDown: '<path d="M6 9l6 6 6-6"/>',
    chevronRight: '<path d="M9 6l6 6-6 6"/>',
    arrowLeft: '<path d="M19 12H5M11 18l-6-6 6-6"/>',
    arrowRight: '<path d="M5 12h14M13 6l6 6-6 6"/>',
    droplet: '<path d="M12 3s6 5.5 6 10a6 6 0 0 1-12 0c0-4.5 6-10 6-10z"/>',
    command: '<path d="M15 6a3 3 0 1 1 3 3h-3zM9 6a3 3 0 1 0-3 3h3zM15 18a3 3 0 1 0 3-3h-3zM9 18a3 3 0 1 1-3-3h3zM9 9h6v6H9z"/>',
    sparkle: '<path d="M12 3l1.7 4.6L18 9l-4.3 1.4L12 15l-1.7-4.6L6 9l4.3-1.4z"/>',
    sop: '<path d="M8 6h11M8 12h11M8 18h11"/><circle cx="4" cy="6" r="1"/><circle cx="4" cy="12" r="1"/><circle cx="4" cy="18" r="1"/>',
    template: '<rect x="4" y="4" width="16" height="16" rx="2"/><path d="M4 9h16M9 9v11"/>',
    plug: '<path d="M12 3v6M8 9h8v3a4 4 0 0 1-8 0zM12 17v4"/>',
    check: '<path d="M20 6L9 17l-5-5"/>',
    winMin: '<path d="M5 12h14"/>',
    winMax: '<rect x="5.5" y="5.5" width="13" height="13" rx="1.5"/>',
    winRestore: '<rect x="7.5" y="7.5" width="11" height="11" rx="1.5"/><path d="M5.5 14.5V6a1.5 1.5 0 0 1 1.5-1.5h8.5"/>',
    grip: '<circle cx="9" cy="6" r="1.2"/><circle cx="9" cy="12" r="1.2"/><circle cx="9" cy="18" r="1.2"/><circle cx="15" cy="6" r="1.2"/><circle cx="15" cy="12" r="1.2"/><circle cx="15" cy="18" r="1.2"/>',
    divider: '<path d="M3 12h18"/>',
    eye: '<path d="M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7-10-7-10-7z"/><circle cx="12" cy="12" r="3"/>',
    eyeOff: '<path d="M2 12s3.6-7 10-7c2.1 0 3.9.6 5.4 1.5M22 12s-3.6 7-10 7c-2.1 0-3.9-.6-5.4-1.5"/><path d="M9.6 9.6a3 3 0 0 0 4.2 4.2"/><path d="M3 3l18 18"/>',
    reveal: '<path d="M3 7a2 2 0 0 1 2-2h3.5l2 2H19a2 2 0 0 1 2 2M3 7v10a2 2 0 0 0 2 2h13a2 2 0 0 0 1.9-1.4L22 11H6.5a2 2 0 0 0-1.9 1.4z"/><path d="M12 12v3M10.5 13.5h3"/>',
    terminal: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 9l3 3-3 3M13 15h4"/>',
    info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v5M12 8h.01"/>',
    shield: '<path d="M12 3l7 3v5c0 4.6-3 8.4-7 10-4-1.6-7-5.4-7-10V6z"/><path d="M9 12l2 2 4-4"/>',
    webhook: '<path d="M18 16.98h-5.99c-1.1 0-1.95.94-2.48 1.9A4 4 0 0 1 2 17c.01-.7.2-1.4.57-2"/><path d="m6 17 3.13-5.78c.53-.97.1-2.18-.5-3.1a4 4 0 1 1 6.89-4.06"/><path d="m12 6 3.13 5.73C15.66 12.7 16.9 13 18 13a4 4 0 0 1 0 8"/>',
    lock: '<rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/>',
  };
</script>

<svg
  class="ic"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill={fill ? "currentColor" : "none"}
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>{@html ICONS[name] || ""}</svg>

<style>
  .ic {
    display: block;
    flex-shrink: 0;
  }
</style>
