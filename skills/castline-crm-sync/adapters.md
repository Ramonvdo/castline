# CRM adapters and the remembered config

## The remembered config

Written on first run to `crm-config.local.json` **beside this skill**, so later runs skip the
interview. The `.local.json` suffix is what keeps it out of git.

```json
{
  "crm": "hubspot",
  "base_url": "https://api.hubapi.com",
  "docs_url": "https://developers.hubspot.com/docs/api/crm/contacts",
  "auth": { "type": "bearer", "env_var": "HUBSPOT_TOKEN" },
  "scope": { "description": "contacts in lifecycle stage customer", "query": "" },
  "match_key": "email",
  "mapping": {
    "properties.firstname": "firstName",
    "properties.company":   "company",
    "properties.email":     "email"
  },
  "excluded_locked": ["personalNote"],
  "configured_at": "2026-09-05"
}
```

Field notes:

- **`auth.env_var` names an environment variable. It never holds the key.** If the variable is not
  set at run time, stop and tell the user which one to set. Do not prompt for the key and hold it
  in the conversation.
- **`mapping`** goes CRM field path to Castline variable name. The right-hand side must be a
  variable the library actually uses; a typo here silently creates a junk variable on every profile.
- **`excluded_locked`** is a record of what was locked at configuration time. It is a note, not the
  enforcement. **Enforcement always re-reads `profiles.json.locked` at run time**, because the user
  may lock a new variable after the config was written.
- **`match_key`** is `email` or `name`. Prefer `email`.

Re-run the interview when the config is missing, when a mapped variable no longer exists in the
library, or when the user asks.

## The adapter contract

Every CRM reduces to one function. Keep the surface this narrow so adding a CRM never touches the
sync logic.

```
fetch_contacts(config, scope) -> list of flat dicts

Each dict:
  - has the CRM's own field names as keys, matching the paths used in `mapping`
  - carries at least the `match_key` field
  - contains raw values, no formatting or inference
```

The sync then applies `mapping`, drops locked variables, drops empty values, and posts the result.

## Adding a CRM

1. **Read the CRM's contacts API docs.** Not a search snippet, the actual docs page. Note the
   pagination scheme, the rate limit, and how custom fields are addressed, because custom fields are
   usually where the interesting variables live.
2. **Pull one real contact and print its field names.** Every mapping decision should be made
   against a real payload, never against a remembered schema.
3. **Handle pagination properly.** A first-page-only sync that silently ignores contacts 101 and up
   looks like it worked, which is worse than failing.
4. **Respect the rate limit.** Back off rather than hammering.

## Notes on common CRMs

Do not treat these as verified. They are starting points; the docs are the source of truth and must
be read before use.

- **HubSpot** nests everything under `properties`, and custom properties need requesting by name.
- **Pipedrive** exposes custom fields as hashed keys, so a field-name lookup is needed to map them.
- **Airtable and Notion**, when used as a CRM, are the easiest case: a flat record with clean field
  names.
- **Attio, Folk, Close** all have modern REST APIs, but each names contact objects differently.

## The CSV fallback

When the CRM has no usable API, or the user would rather not wire credentials, the same mapping
works against an export:

1. User exports contacts to CSV or JSON.
2. Config sets `"crm": "csv"` and `"base_url"` to the file path.
3. `fetch_contacts` reads the file; the mapping, locked filtering and endpoint writes are unchanged.

This works with every CRM on the first run and needs no credentials at all. Offer it early rather
than as a last resort, especially for a one-off backfill.
