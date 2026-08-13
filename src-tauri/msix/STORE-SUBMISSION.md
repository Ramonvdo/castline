# Microsoft Store submission — checklist and copy

Everything needed for the Partner Center listing. Nothing here is code; it's the text and the
answers to paste in. Work top to bottom.

---

## 1. Account and reservation — ✅ DONE

Partner Center product is reserved. Identity is baked into `Package.appxmanifest`:

| Field | Value |
| --- | --- |
| Package/Identity/Name | `Ravando.Castline` |
| Package/Identity/Publisher | `CN=0FDF4166-D594-445A-AC0D-659012821ABB` |
| Package/Properties/PublisherDisplayName | `Ravando` |
| Package Family Name | `Ravando.Castline_36ex7sfbaqfcj` |
| Store ID | `9NVJX06SSMTH` |

The PFN suffix is a hash of the Publisher string, and the committed value reproduces
`36ex7sfbaqfcj` exactly — so the identity is verified correct, not just copied.

## 1b. Build and upload the package

```bash
npm run pack:msix        # -> Castline_<version>_x64.msix
```

It prints the identity it packed; confirm it says `Ravando.Castline` before uploading. Then in
Partner Center: **Submission → Packages → upload the .msix**. The Store signs it — you do not need a
certificate.

Each later submission needs a **higher** version. Bump the app version (README → Cutting a release)
and the manifest version follows automatically.

---

## 2. Pricing and availability

Castline is listed **free**. That keeps the Store consistent with castline.dev ("Free without
limits"), avoids Microsoft's 15% cut, and — the practical part — means **no payout account and no
US tax forms (W-8BEN)** are needed before submitting. Money comes later from Castline Cloud, which
can be sold through your own payment provider while keeping 100%.

| Field | Set it to |
| --- | --- |
| **Base price** | **Free** ← this clears the "No PriceSchedule created for purchasable product" error |
| Markets | **All worldwide markets** (240) — leave "make my product available in any future market" ticked |
| Audience | **Public audience** |
| Discoverability | **Make this product available and discoverable in the Microsoft Store** |
| Schedule → Release | **as soon as possible** |
| Schedule → Stop acquisition | **never** |
| Free trial | none (not applicable to a free app) |
| Sale pricing | none |
| Organizational licensing | leave the default — volume acquisition allowed, offline licensing off. Lets companies deploy Castline without extra work on your side. |

## 3. Properties

| Field | Set it to |
| --- | --- |
| Category | **Productivity** |
| Privacy policy URL | **https://castline.dev/privacy/** — required, because the app can make network calls |
| Minimum OS | **Windows 10 version 1809 (10.0.17763.0)** |

The minimum OS **must** match `TargetDeviceFamily MinVersion` in `Package.appxmanifest`. If they
disagree, the listing offers the app to machines the package refuses to install on.

### Support info (and the DSA question)

| Field | Set it to |
| --- | --- |
| Which details | **Use different details for this app** |
| Website | **https://castline.dev** |
| Support contact info | **https://github.com/Ramonvdo/castline/issues** |
| Phone number | **leave blank** (see below) |
| Address fields | **leave blank** (see below) |

Phone and address aren't marked required on this form, so leave them empty and try to save — a
website plus a support URL is enough for customers to reach you.

The catch is the **EU Digital Services Act**. A "trader" is anyone acting for purposes relating to
their trade or profession, and traders must publish an **address, phone number and email** on the
product page. With a KvK registration behind Castline you very likely count as one, free app or not.
So:

- Check **Account Settings → Legal info → Developer tab** for your DSA/trader status. If Microsoft
  has flagged you as a trader, it will demand these details there, and they become **publicly
  visible on the listing**.
- If you do have to supply an address, use a **business address, not your home address** — a KvK
  registered address, a postbus, or a virtual office. For a Dutch eenmanszaak the KvK address is
  often the home address, and a Store listing broadcasts it far more widely than the KvK register.

### Product declarations

| Declaration | Set it to |
| --- | --- |
| Allows purchases without the Microsoft commerce system | **Unchecked** — Castline has no purchases at all |
| Tested to meet accessibility guidelines | **Unchecked** — it hasn't been tested; Microsoft explicitly warns against claiming this |
| Install to alternate drives or removable storage | **Leave checked** (default) — no reason to restrict |
| Include this product's data in automatic OneDrive backups | **Leave checked** (default) — it's the user's own OneDrive and only if they turn backups on, and it protects their library. Uncheck if you'd rather the "nothing leaves your device" promise hold even for the user's own backups. |
| Record/broadcast clips | **Unchecked** — games only |
| Supports pen and ink input | **Unchecked** |
| **Incorporates generative AI features** | **CHECK THIS** ⚠ |

That last one is easy to miss and matters. Castline **does** incorporate generative AI: AI enrich
generates variable values through OpenRouter, and the Agent tab runs Claude Code. Microsoft's rule
covers models "accessed via cloud services or APIs" and "provided by third-party AI platforms", so
bring-your-own-key and opt-in don't exempt it. Declaring it is both accurate and avoids a compliance
problem later.

### System requirements

**Leave everything unchecked and "Not specified"** — every hardware row (touch, keyboard, mouse,
camera, NFC, Bluetooth, telephony, microphone, gamepad, Mixed Reality) plus Memory, DirectX, Video
memory, Processor and Graphics.

Castline is an ordinary desktop app that needs nothing special, and anything listed as a **Minimum**
requirement makes the Store warn customers whose hardware doesn't report it — and blocks them from
rating or reviewing. There's no upside to declaring a keyboard.

### Display mode

Leave **PC** and **HoloLens** unchecked. That section is for immersive Windows Mixed Reality
experiences; Castline is a normal 2D desktop window.

---

## 4. Listing copy

**Name:** Castline

**Short description** (max ~200 chars)

> Cast it once, paste it anywhere. A local-first library for your AI prompts, email templates and
> SOPs — with live {{variables}} that fill themselves and one-click copy.

**Description**

> Castline is a fast, local-first library for any text you reuse: AI prompts, email templates,
> notes and multi-step SOPs.
>
> Write a template once and mark the parts that change as {{variables}}. Fill them from a reusable
> profile — a client, a prospect, a project — and copy the finished text in one click. No more
> hunting through old documents or rewriting the same message.
>
> **What you can do**
>
> • Store prompts, templates, notes and SOPs in colour-coded folders
> • Fill {{variables}} from reusable profiles, with a live preview of the result
> • Copy any template with one click, or find it instantly with Ctrl+K
> • Step through multi-step SOPs, copying one step at a time
> • Share templates as blueprints — export a .json file or copy it to your clipboard, and anyone
>   can import it into their own library
> • Send filled templates straight to Make or n8n webhooks
> • Bring your own AI key (OpenRouter) to fill in the blanks in your own tone
> • Run Claude Code inside the app to research contacts and build profiles for you
>
> **Yours, and local**
>
> Everything lives in plain JSON files on your own machine. No account, no cloud, no telemetry, no
> subscription. Castline is free and open source under the MIT licence — you can read every line at
> github.com/Ramonvdo/castline
>
> Network access is entirely opt-in: nothing leaves your computer unless you set up a webhook, add
> your own AI key, or ask the built-in agent to look something up.

**What's new in this version:** **leave blank** — Partner Center says to skip it on a first
submission. From v1.1.4 onward, put the release highlights here.

### Supplemental fields

| Field | Value |
| --- | --- |
| Short title | **leave blank** (Xbox-only) |
| Voice title | **leave blank** (Xbox/Kinect-only) |
| Short description | see below (≤270 chars) |

> Cast it once, paste it anywhere. Keep your AI prompts, email templates and SOPs in one local-first
> library — fill {{variables}} from reusable profiles and copy in a click. No account, no cloud, no
> telemetry.

### Additional information

**Keywords** (max 7, ≤40 chars each, ≤21 words total — this set is 12 words):

```
prompt manager
AI prompt library
text templates
email templates
snippets
SOP
productivity
```

Deliberately **not** "clipboard" or "clipboard manager" — Castline isn't a clipboard-history tool,
and that keyword would pull in users looking for something else, who then leave bad reviews.

| Field | Value |
| --- | --- |
| Copyright and trademark info | `© 2026 Ravando. Castline is open source under the MIT License.` |
| Additional license terms | `Castline is free and open-source software licensed under the MIT License: https://github.com/Ramonvdo/castline/blob/main/LICENSE` |
| Developed by | `Ravando` |

**Category:** Productivity · **Website:** https://castline.dev ·
**Privacy policy:** https://castline.dev/privacy/ ·
**Support:** https://github.com/Ramonvdo/castline/issues

---

## 4b. Store logos and art — ✅ GENERATED

`python scripts/store-assets.py logos` builds all of these into `store-assets/`, from `icon.png`
and the app's own palette, so they match the product exactly:

| Listing slot | File |
| --- | --- |
| 9:16 Poster art (720×1080) | `poster-720x1080.png` (also `poster-1440x2160.png`) |
| 1:1 Box art (1080×1080) | `box-1080x1080.png` (also `box-2160x2160.png`) |
| Store logo 300×300 | `logo-300.png` |
| Store logo 150×150 | `logo-150.png` |
| Store logo 71×71 | `logo-71.png` |
| 16:9 Super hero art (1920×1080) | `hero-1920x1080.png` |

Poster art is **highly recommended** — it's the main logo customers see on Windows 10/11. The hero
art deliberately carries **no text**, because Microsoft requires it not to include the product title.

---

## 5. Screenshots

At least one is required; **four or five is the sweet spot**. Most people never read the
description — they swipe the screenshots and decide. So each one should make a single point, with a
caption that says the benefit rather than naming the UI.

**Capture these five**, window maximised, then save them into `store-assets/raw/` with these exact
names and run `python scripts/store-assets.py shots`. That composes each onto a branded 1920×1080
frame with the caption already applied, so the set looks designed rather than like five loose grabs.

| Save as | What to show | Caption applied |
| --- | --- | --- |
| `1-library.png` | The library grid: folder rail on the left, a healthy spread of template cards | **Every prompt, template and SOP in one place** — Colour-coded folders. Search, tags, and the ones you use most, first. |
| `2-fill.png` | Fill & copy open, a profile selected, `{{variables}}` filled, live preview showing the finished text | **Fill {{variables}} from a profile, copy in one click** — Live preview shows exactly what lands on your clipboard. |
| `3-blueprint.png` | The blueprint import preview, showing the templates and variables a shared file contains | **Share a template as a blueprint** — Export a .json, send it to anyone, import in one step. |
| `4-quickfind.png` | Ctrl+K palette open, a few letters typed, results listed | **Ctrl+K. Type. Copied.** — Find any template instantly, without leaving the keyboard. |
| `5-sop.png` | A multi-step SOP mid-walkthrough, on step 2 of 3 | **Walk through multi-step SOPs** — Copy one step at a time, in order, without losing your place. |

If you'd rather word them differently, the captions live in `CAPTIONS` at the top of
`scripts/store-assets.py` — edit and re-run.

**Order matters.** Partner Center shows them in upload order, so upload 1 → 5. The first screenshot
is the one that sells; that's why it's the full library rather than a dialog.

> **Never screenshot the Settings tab.** It shows your real Make webhook URL and OpenRouter API key.
> Nothing in the list above touches it.

**Your live library is your real one** — client names, case studies, "Operation Apex" and so on —
so straight screenshots of it would publish your actual work to a public Store listing. Two clean
ways round that:

- **Curate as you shoot.** Open a folder with nothing client-specific in it (Copywriting or Claude
  Code work well), or briefly rename anything identifying. Grab the five shots with `Win+Shift+S`
  at a maximised window, which is comfortably over 1366×768 on your display.
- **Shoot a fresh instance.** Castline seeds a small example library when it starts with no data,
  which makes for clean, generic screenshots. The data folder can't be redirected with an
  environment variable (Windows resolves it through a known-folder API), so this means temporarily
  moving `%APPDATA%\Castline` aside and putting it back afterwards. Reversible, but it touches real
  data — ask before doing it.

---

## 5b. Trailer (optional, but it converts)

Not required, and the listing is fine without one. If you do make it, a trailer plays at the top of
the listing — and it only appears if a **16:9 hero image** is supplied, which
`hero-1920x1080.png` already covers.

**Rules of thumb:** 20–30 seconds. **Assume it plays muted** — carry everything in on-screen text,
never narration. Record at 1920×1080/30fps. Move the mouse slowly and deliberately; fast cursor
movement reads as chaotic at small sizes. Use a clean library with no client names.

**Storyboard**

| Time | On screen | Caption |
| --- | --- | --- |
| 0:00–0:04 | The library grid, cursor drifting over cards | *Everything you retype, in one place* |
| 0:04–0:11 | Open a template, pick a profile, `{{variables}}` fill live, hit Copy, "Copied" toast | *Fill it from a profile. Copy in one click.* |
| 0:11–0:16 | Press Ctrl+K, type three letters, Enter | *Or just Ctrl+K* |
| 0:16–0:23 | Right-click → Export blueprint; then drag the .json back in → import preview | *Share a template with anyone* |
| 0:23–0:30 | Hero art / logo, hold still | *Castline — free, open source, and entirely on your machine* · castline.dev |

Record with **Xbox Game Bar** (`Win+G`), OBS, or ScreenToGif. One continuous take with slow,
deliberate actions beats five hard cuts — you're demonstrating that the flow is fast, so let the
speed of the app do the talking rather than the editing.

**Skip it if you're short on time.** Five good captioned screenshots do most of the work; a rushed
trailer is worse than none.

---

## 6. Age rating and questionnaire answers

Run the Store's rating questionnaire with these answers:

| Question | Answer |
| --- | --- |
| Violence / sexual content / profanity / gambling / drugs | **No** to all |
| Does the app collect personal information? | **No** — all data stays on the user's device |
| Does the app share data with third parties? | **No** |
| Does the app access the internet? | **Yes** — only when the user configures a webhook, adds their own AI API key, or uses the built-in agent |
| User-generated content shared between users? | **No** — blueprints are files the user chooses to share themselves |
| Target age | **General audiences** (expected rating: PEGI 3 / ESRB Everyone) |

---

## 7. Submission options

| Field | Set it to |
| --- | --- |
| Publishing hold options | **Publish this submission as soon as it passes certification** |
| Notes for certification | paste the block below |

The notes matter more than usual here: the package declares **`runFullTrust`**, a restricted
capability, and reviewers routinely ask why. This answers it up front.

> Castline is a local-first text-template manager, packaged as a full-trust Win32 desktop app
> (Tauri/Rust). All data is stored as plain JSON in the user's app-data folder. There is no account,
> sign-in, telemetry or analytics of any kind.
>
> **Why runFullTrust is required:** the app is a conventional desktop application inside the package.
> It reads and writes its own JSON library in %APPDATA%, lets the user save and open template files
> anywhere on disk, and can launch the user's own locally-installed Claude Code CLI as a child
> process from the Agent tab.
>
> **Network use is optional and always user-initiated:**
> • The user may paste their own Make/n8n webhook URL to send a filled template to their automation.
> • The user may add their own OpenRouter API key to fill template variables using AI.
> • The Agent tab runs the user's own Claude Code CLI, only if they installed it themselves.
> • The app checks the public GitHub releases API for update notifications.
> • An optional local HTTP endpoint (disabled by default, 127.0.0.1 only, token-protected) lets the
>   user's own automations write data in.
>
> None of these are enabled by default and the app is fully functional with no network connection.
> Castline is free and open source under the MIT licence; the complete source for this build is at
> https://github.com/Ramonvdo/castline

---

## 8. After it's live

- Add the Store badge/link to castline.dev and the README as the warning-free Windows option.
- Keep the `.exe` on GitHub for people who prefer a plain installer.
- On each release: bump the version (see README → Cutting a release), download the
  `microsoft-store-msix` artifact from the tagged CI run, and upload it as a new Store submission.
  The MSIX version must always increase.
