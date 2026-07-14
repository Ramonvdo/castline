<script>
  import { FOLDER_ICONS } from "./foldericons.js";
  let { name = "folder", color = "currentColor", size = 16 } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill={color} aria-hidden="true" style="display:block;flex-shrink:0">
  {@html FOLDER_ICONS[name] || FOLDER_ICONS.folder}
</svg>
