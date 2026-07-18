# Security Policy

## Supported versions

Only the [latest release](https://github.com/Ramonvdo/castline/releases/latest) receives
security fixes.

## Reporting a vulnerability

Please use GitHub's **private vulnerability reporting** for this repository
(**Security → Report a vulnerability**) rather than opening a public issue.

Castline is local-first: your data lives in plain JSON files on your machine, and the app
only makes outbound requests to webhook URLs / API endpoints you configure yourself. Reports
about the inbound HTTP endpoint (token handling, `127.0.0.1` binding) and the connector
send paths are particularly appreciated.

## Secrets at rest

Your data folder (**Settings → Data & backups → Open folder**) stores secrets **in plaintext**,
which is standard for a local-first desktop app:

- `settings.json` holds your OpenRouter API key and the inbound-endpoint bearer token.
- `CLAUDE.md` (generated for the AI agent) embeds the endpoint token while the endpoint is on.

Treat backups and exports of this folder as sensitive — don't commit it to a repo, put it in a
shared drive, or paste its contents anywhere. If a secret is exposed, rotate it: regenerate the
endpoint token in the Connectors tab, and revoke/replace the OpenRouter key at
[openrouter.ai/keys](https://openrouter.ai/keys).

## Inbound HTTP endpoint

The endpoint is off by default, binds `127.0.0.1` only, uses a random 256-bit token, rejects
browser cross-site requests, and rate-limits repeated bad tokens. Exposing it to the internet via a
tunnel is possible but at your own risk — see the README. For internet-facing automation, prefer the
forthcoming hosted Castline Cloud API over tunnelling a loopback server.
