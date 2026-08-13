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
| Website | **https://castline.dev** |
| Support contact info | **https://github.com/Ramonvdo/castline/issues** |
| "Functions with limited or no internet connectivity" | **Yes** — Castline is fully usable offline |
| "Tested to meet accessibility guidelines" | **No** — don't claim this until it's actually been tested |
| Minimum OS | **Windows 10 version 1809 (10.0.17763.0)** |

The minimum OS **must** match `TargetDeviceFamily MinVersion` in `Package.appxmanifest`. If they
disagree, the listing offers the app to machines the package refuses to install on.

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

**Search terms:** prompt manager · prompt library · AI prompts · text templates · email templates ·
SOP · snippets · clipboard · productivity · local-first

**Category:** Productivity

**Copyright:** © 2026 Ravando

**Website:** https://castline.dev
**Privacy policy:** https://castline.dev/privacy/
**Support contact:** https://github.com/Ramonvdo/castline/issues

---

## 5. Screenshots

The Store wants at least one **1366×768** (or larger, 16:9) screenshot. Capture with the window
maximised, using the demo library so no personal data is shown:

1. **Library** — folders on the left, template cards in the grid
2. **Fill & copy** — a template with {{variables}} filled from a profile, live preview visible
3. **Blueprint import** — the preview modal showing what a shared template contains
4. **Quick find** — the Ctrl+K palette open with results
5. **SOP steps** — a multi-step SOP mid-walkthrough

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
