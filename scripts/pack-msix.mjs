// Build the Microsoft Store package.
//
// Tauri doesn't emit MSIX, so this stages the release binary plus the Store
// assets into a layout folder and packs it with makeappx (Windows SDK). The
// Store signs the package when you submit it — signing here is only needed if
// you want to test-install locally (pass --sign, see README).
//
//   node scripts/pack-msix.mjs            # build + pack  -> Castline_<ver>_x64.msix
//   node scripts/pack-msix.mjs --no-build # pack an existing release binary
//
// The manifest's Version is rewritten from tauri.conf.json on every run, so the
// package version can never silently drift from the app version.

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, copyFileSync, readFileSync, writeFileSync, rmSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const layout = join(root, "dist-msix");
const assetsOut = join(layout, "Assets");
const iconsDir = join(root, "src-tauri", "icons");
const exeSrc = join(root, "src-tauri", "target", "release", "castline.exe");

const conf = JSON.parse(readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8"));
const version = conf.version; // e.g. "1.1.3"
const msixVersion = `${version}.0`; // MSIX wants four parts
const outFile = join(root, `Castline_${version}_x64.msix`);

// Every asset the manifest references. Missing one fails packaging with a
// confusing error, so check up front and say exactly which is absent.
const ASSETS = [
  "StoreLogo.png",
  "Square44x44Logo.png",
  "Square71x71Logo.png",
  "Square150x150Logo.png",
];

// No shell anywhere: the SDK lives under "Program Files (x86)" and shell
// quoting of that path is a reliable source of pain.
function run(cmd, args) {
  execFileSync(cmd, args, { stdio: "inherit", cwd: root });
}

// Drive the Tauri CLI through node rather than npm: Node 22 refuses to spawn
// .cmd shims (npm.cmd) without a shell, and this skips the indirection anyway.
function runTauri(args) {
  run(process.execPath, [join(root, "node_modules", "@tauri-apps", "cli", "tauri.js"), ...args]);
}

/** makeappx.exe isn't on PATH; find the newest Windows SDK copy. */
function findMakeAppx() {
  const bases = [
    "C:\\Program Files (x86)\\Windows Kits\\10\\bin",
    "C:\\Program Files\\Windows Kits\\10\\bin",
  ];
  const found = [];
  for (const base of bases) {
    if (!existsSync(base)) continue;
    for (const dir of readdirSync(base)) {
      const candidate = join(base, dir, "x64", "makeappx.exe");
      if (existsSync(candidate)) found.push(candidate);
    }
  }
  if (!found.length) {
    throw new Error(
      "makeappx.exe not found. Install the Windows SDK (or Visual Studio Build Tools\n" +
        "with the Windows SDK component), then re-run.",
    );
  }
  // Highest SDK version wins — the paths sort naturally (10.0.22621.0 etc).
  return found.sort().at(-1);
}

const build = !process.argv.includes("--no-build");
if (build) {
  console.log("→ building release binary…");
  runTauri(["build", "--no-bundle"]);
}
if (!existsSync(exeSrc)) {
  throw new Error(`Release binary not found at ${exeSrc}. Run without --no-build first.`);
}

console.log("→ staging package layout…");
rmSync(layout, { recursive: true, force: true });
mkdirSync(assetsOut, { recursive: true });
copyFileSync(exeSrc, join(layout, "castline.exe"));
for (const asset of ASSETS) {
  const src = join(iconsDir, asset);
  if (!existsSync(src)) throw new Error(`Missing Store asset: ${src}`);
  copyFileSync(src, join(assetsOut, asset));
}

// Version always tracks tauri.conf.json; identity comes from the environment so
// the Partner Center values never have to be committed to a public repo.
const xmlEscape = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
const identityName = process.env.MSIX_IDENTITY_NAME;
const publisher = process.env.MSIX_PUBLISHER;
const publisherDisplay = process.env.MSIX_PUBLISHER_DISPLAY_NAME;

let manifest = readFileSync(join(root, "src-tauri", "msix", "Package.appxmanifest"), "utf8");
manifest = manifest.replace(/<Identity[\s\S]*?\/>/, (block) => {
  let out = block.replace(/Version="[^"]*"/, `Version="${msixVersion}"`);
  if (identityName) out = out.replace(/Name="[^"]*"/, `Name="${xmlEscape(identityName)}"`);
  if (publisher) out = out.replace(/Publisher="[^"]*"/, `Publisher="${xmlEscape(publisher)}"`);
  return out;
});
if (publisherDisplay) {
  manifest = manifest.replace(
    /<PublisherDisplayName>[\s\S]*?<\/PublisherDisplayName>/,
    `<PublisherDisplayName>${xmlEscape(publisherDisplay)}</PublisherDisplayName>`,
  );
}

const forStore = identityName && publisher && publisherDisplay;
if (!forStore) {
  console.warn(
    "\n⚠ Building with DEVELOPMENT identity (Castline.Dev).\n" +
      "  Fine for local test-installs; Partner Center will reject it.\n" +
      "  For a submission, set MSIX_IDENTITY_NAME, MSIX_PUBLISHER and\n" +
      "  MSIX_PUBLISHER_DISPLAY_NAME from Partner Center > Product identity.\n",
  );
}
writeFileSync(join(layout, "AppxManifest.xml"), manifest);

console.log(`→ packing ${outFile}…`);
rmSync(outFile, { force: true });
run(findMakeAppx(), ["pack", "/d", layout, "/p", outFile, "/o"]);

console.log(`\n✔ ${outFile}`);
if (forStore) {
  console.log("  Submit this file in Partner Center — the Store signs it for you.");
} else {
  console.log("  Development build. To test-install it, sign it with a self-signed");
  console.log("  certificate first (see README → Microsoft Store).");
}
