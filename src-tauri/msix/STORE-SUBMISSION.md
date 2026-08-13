# Microsoft Store submission — checklist and copy

Everything needed for the Partner Center listing. Nothing here is code; it's the text and the
answers to paste in. Work top to bottom.

---

## 1. Account and reservation (one-off)

1. Create a **free** Partner Center developer account at
   <https://partner.microsoft.com/dashboard>. Registration fees were removed — individuals
   since Sept 2025, companies since May 2026.
   Register as the **KvK company** if you want a verified company publisher name on the listing.
2. **Apps and games → New product → MSIX or PWA app.**
   (Not "EXE or MSI app" — that route requires a code-signing certificate you don't have; MSIX is
   the one the Store signs for you.)
3. Reserve the name **Castline**. If it's taken, reserve `Castline — Prompt & Template Library`
   and set the shorter display name in the listing.
4. Copy the three values from **Product → Product identity** and use them when packaging:

   ```
   MSIX_IDENTITY_NAME=<Package/Identity/Name>
   MSIX_PUBLISHER=<Package/Identity/Publisher>          # looks like CN=XXXXXXXX-XXXX-…
   MSIX_PUBLISHER_DISPLAY_NAME=<Package/Properties/PublisherDisplayName>
   ```

5. Build the package with those set (see README → Microsoft Store) and upload the `.msix`.

---

## 2. Listing copy

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

**Copyright:** © 2026 Castline Software

**Website:** https://castline.dev
**Privacy policy:** https://castline.dev/privacy
**Support contact:** https://github.com/Ramonvdo/castline/issues

---

## 3. Screenshots

The Store wants at least one 1366×768 (or larger, 16:9) screenshot. Capture with the window
maximised, using the demo library so no personal data is shown:

1. **Library** — folders on the left, template cards in the grid
2. **Fill & copy** — a template with {{variables}} filled from a profile, live preview visible
3. **Blueprint import** — the preview modal showing what a shared template contains
4. **Quick find** — the Ctrl+K palette open with results
5. **SOP steps** — a multi-step SOP mid-walkthrough

---

## 4. Age rating and questionnaire answers

Run the Store's rating questionnaire with these answers:

| Question | Answer |
| --- | --- |
| Violence / sexual content / profanity / gambling / drugs | **No** to all |
| Does the app collect personal information? | **No** — all data stays on the user's device |
| Does the app share data with third parties? | **No** |
| Does the app access the internet? | **Yes** — only when the user configures a webhook, adds their own AI API key, or uses the built-in agent |
| User-generated content shared between users? | **No** — blueprints are files the user chooses to share themselves |
| Target age | **General audiences** (expected rating: PEGI 3 / ESRB Everyone) |

**Note for the certification reviewer** (paste in "Notes for certification"):

> Castline is a local-first text-template manager. All data is stored in plain JSON in the user's
> app-data folder; there is no account, sign-in, telemetry or analytics.
>
> Network access is optional and user-initiated:
> • The user may paste their own Make/n8n webhook URL to send a filled template.
> • The user may add their own OpenRouter API key to fill template variables with AI.
> • The Agent tab launches the user's own locally-installed Claude Code CLI, if present.
> • The app checks GitHub for release updates.
>
> None of these are enabled by default and the app is fully functional offline. Source code:
> https://github.com/Ramonvdo/castline

---

## 5. After it's live

- Add the Store badge/link to castline.dev and the README as the warning-free Windows option.
- Keep the `.exe` on GitHub for people who prefer a plain installer.
- On each release: bump the version (see README → Cutting a release), download the
  `microsoft-store-msix` artifact from the tagged CI run, and upload it as a new Store submission.
  The MSIX version must always increase.
