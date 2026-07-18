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
