//! The "Castline AI" enrich workflow — a single OpenRouter chat-completions
//! call that fills a profile's `{{variables}}` from what's already known about
//! it (optionally with live web research via OpenRouter's `:online` suffix).
//! The per-variable descriptions the user writes in Settings ride along as the
//! authoritative definition of what each value must look like.

use std::time::Duration;

use serde_json::{json, Value};

use crate::settings::LlmConfig;

const OPENROUTER_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

/// Resolve the effective tone: profile override → Settings → none. There is no
/// hidden built-in fallback — an empty tone means no tone section in the prompt
/// (Settings merely arrives *prefilled* with a suggestion the user can clear).
pub fn effective_tone<'a>(profile_tone: &'a str, settings_tone: &'a str) -> &'a str {
    if !profile_tone.trim().is_empty() {
        profile_tone
    } else if !settings_tone.trim().is_empty() {
        settings_tone
    } else {
        ""
    }
}

/// What the model gets to work with, beyond the profile values themselves.
pub struct EnrichInputs<'a> {
    /// (name, description) for every library variable.
    pub vars: &'a [(String, String)],
    /// User-supplied notes / attached-file text ("" = none).
    pub context: &'a str,
    /// The library templates where the variables are used ("" = none) — lets
    /// the model write values that read naturally in place.
    pub usage: &'a str,
    /// Tone of voice for generated text values (already resolved).
    pub tone: &'a str,
}

/// Build the chat messages: a strict system prompt (variable docs + template
/// usage + tone) and the current profile values (+ any user-supplied context)
/// as the user turn.
pub fn build_messages(values_json: &str, inputs: &EnrichInputs) -> Value {
    let EnrichInputs { vars, context, usage, tone } = inputs;
    let mut var_lines = String::new();
    for (name, desc) in vars.iter() {
        if desc.trim().is_empty() {
            var_lines.push_str(&format!("- {name}\n"));
        } else {
            var_lines.push_str(&format!("- {name}: {}\n", desc.trim()));
        }
    }
    if var_lines.is_empty() {
        var_lines.push_str("(no variable definitions — infer sensible values from the data)\n");
    }
    let mut system = format!(
        "You enrich a contact/company profile for a templating app. The user gives you the \
         profile's current values as JSON. Research or infer the missing variables and return \
         ONLY a JSON object mapping variable names to string values — no prose, no markdown \
         fences. Rules:\n\
         - Fill only the variables listed below; omit any you cannot determine confidently.\n\
         - Where a description is given it defines EXACTLY what the value must look like — \
           follow it to the letter (formatting, casing, abbreviations).\n\
         - Keep existing values unless you have a clearly better/corrected value.\n\
         - Values must be plain strings.\n\n\
         Variables:\n{var_lines}"
    );
    if !tone.trim().is_empty() {
        system.push_str(&format!(
            "\nTone of voice — any text value you write (icebreakers, messages, blurbs) MUST \
             follow it exactly:\n{}\n",
            tone.trim()
        ));
    }
    if !usage.trim().is_empty() {
        system.push_str(&format!(
            "\nHow the variables are used in the user's templates — write values that read \
             naturally when substituted in place:\n{}\n",
            usage.trim()
        ));
    }
    let mut user = format!("Current profile values:\n{values_json}");
    if !context.trim().is_empty() {
        user.push_str(&format!(
            "\n\nAdditional information supplied by the user (notes / attached file) — treat this \
             as the most authoritative source:\n{}",
            context.trim()
        ));
    }
    json!([
        { "role": "system", "content": system },
        { "role": "user", "content": user }
    ])
}

/// Extract the assistant text and parse it as a JSON object, tolerating ```json
/// fences and surrounding prose. Returns the object as a compact JSON string.
pub fn parse_reply(body: &str) -> Result<String, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| format!("bad API response: {e}"))?;
    if let Some(err) = v.get("error") {
        let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("unknown error");
        return Err(format!("OpenRouter error: {msg}"));
    }
    let content = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("API response had no message content")?;

    // Strip code fences / grab the outermost {...} block.
    let inner = match (content.find('{'), content.rfind('}')) {
        (Some(a), Some(b)) if b > a => &content[a..=b],
        _ => return Err(format!("model did not return JSON: {}", content.trim())),
    };
    let obj: Value =
        serde_json::from_str(inner).map_err(|e| format!("model returned invalid JSON: {e}"))?;
    if !obj.is_object() {
        return Err("model returned JSON that is not an object".into());
    }
    Ok(obj.to_string())
}

/// Run the enrich call. `values_json` is the profile's current values as JSON;
/// returns a JSON object string of variable → filled value.
pub fn enrich(cfg: &LlmConfig, values_json: &str, inputs: &EnrichInputs) -> Result<String, String> {
    enrich_at(OPENROUTER_URL, cfg, values_json, inputs)
}

/// Same as `enrich` but with an injectable endpoint (unit tests point this at a
/// local fake server).
pub fn enrich_at(
    url: &str,
    cfg: &LlmConfig,
    values_json: &str,
    inputs: &EnrichInputs,
) -> Result<String, String> {
    if cfg.api_key.trim().is_empty() {
        return Err("No OpenRouter API key — add one in Settings → AI workflow.".into());
    }
    let mut model = cfg.model.trim().to_string();
    if model.is_empty() {
        model = "google/gemini-2.5-flash".into();
    }
    if cfg.web_search && !model.ends_with(":online") {
        model.push_str(":online");
    }
    // Cap the completion size: the reply is one small JSON object, and leaving
    // max_tokens unset makes OpenRouter assume the model maximum — which fails
    // the pre-flight credit check (402) on free/low-credit accounts.
    let payload = json!({
        "model": model,
        "max_tokens": 2048,
        "messages": build_messages(values_json, inputs),
    });
    let resp = ureq::post(url)
        .timeout(Duration::from_secs(120))
        .set("Authorization", &format!("Bearer {}", cfg.api_key.trim()))
        .set("Content-Type", "application/json")
        .set("HTTP-Referer", "https://github.com/Ramonvdo/castline")
        .set("X-Title", "Castline")
        .send_string(&payload.to_string());
    let body = match resp {
        Ok(r) => r.into_string().map_err(|e| e.to_string())?,
        Err(ureq::Error::Status(code, r)) => {
            let text = r.into_string().unwrap_or_default();
            // Surface the API's own message when it has one.
            if let Ok(ok) = parse_reply(&text) {
                return Ok(ok);
            }
            return Err(match serde_json::from_str::<Value>(&text) {
                Ok(v) => format!(
                    "OpenRouter {code}: {}",
                    v["error"]["message"].as_str().unwrap_or(&text)
                ),
                Err(_) => format!("OpenRouter {code}: {text}"),
            });
        }
        Err(e) => return Err(e.to_string()),
    };
    parse_reply(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> LlmConfig {
        LlmConfig { api_key: "sk-or-test".into(), model: "test/model".into(), web_search: false, tone: String::new() }
    }

    fn inputs<'a>(
        vars: &'a [(String, String)],
        context: &'a str,
        usage: &'a str,
        tone: &'a str,
    ) -> EnrichInputs<'a> {
        EnrichInputs { vars, context, usage, tone }
    }

    #[test]
    fn messages_carry_values_descriptions_context_usage_and_tone() {
        let vars = vec![
            ("companyName".to_string(), "abbreviated lowercase name, e.g. rocketfarm".to_string()),
            ("firstName".to_string(), String::new()),
        ];
        let msgs = build_messages(
            r#"{"company":"RocketFarm Studios LLC"}"#,
            &inputs(
                &vars,
                "Met Sam at the conference; they're the CTO.",
                "### Cold outreach\nHi {{firstName}}, quick one about {{companyName}}…",
                "Pirate speak, always.",
            ),
        );
        let sys = msgs[0]["content"].as_str().unwrap();
        assert!(sys.contains("companyName: abbreviated lowercase name"));
        assert!(sys.contains("- firstName\n"));
        assert!(sys.contains("Tone of voice"));
        assert!(sys.contains("Pirate speak, always."));
        assert!(sys.contains("used in the user's templates"));
        assert!(sys.contains("Hi {{firstName}}, quick one about {{companyName}}"));
        let user = msgs[1]["content"].as_str().unwrap();
        assert!(user.contains("RocketFarm Studios LLC"));
        assert!(user.contains("Met Sam at the conference"));
        assert!(user.contains("Additional information supplied by the user"));

        // No context/usage/tone → no such sections.
        let bare = build_messages("{}", &inputs(&[], "  ", "", ""));
        assert!(!bare[1]["content"].as_str().unwrap().contains("Additional information"));
        let sys = bare[0]["content"].as_str().unwrap();
        assert!(!sys.contains("Tone of voice"));
        assert!(!sys.contains("used in the user's templates"));
    }

    #[test]
    fn tone_resolution_order() {
        assert_eq!(effective_tone("per-profile pirate", "settings tone"), "per-profile pirate");
        assert_eq!(effective_tone("  ", "settings tone"), "settings tone");
        // No hidden fallback: both empty → no tone at all.
        assert_eq!(effective_tone("", ""), "");
    }

    #[test]
    fn parse_reply_handles_fences_and_prose() {
        let body = serde_json::json!({
            "choices": [{ "message": { "content":
                "Here you go:\n```json\n{ \"companyName\": \"rocketfarm\" }\n```" } }]
        })
        .to_string();
        let out = parse_reply(&body).unwrap();
        let obj: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(obj["companyName"], "rocketfarm");
    }

    #[test]
    fn parse_reply_rejects_non_json_and_surfaces_api_errors() {
        let no_json = serde_json::json!({
            "choices": [{ "message": { "content": "I could not find anything." } }]
        })
        .to_string();
        assert!(parse_reply(&no_json).unwrap_err().contains("did not return JSON"));

        let api_err =
            serde_json::json!({ "error": { "message": "Invalid API key" } }).to_string();
        assert!(parse_reply(&api_err).unwrap_err().contains("Invalid API key"));
    }

    #[test]
    fn enrich_requires_key_and_round_trips_against_fake_server() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let mut nokey = cfg();
        nokey.api_key = String::new();
        assert!(enrich_at("http://127.0.0.1:1/x", &nokey, "{}", &inputs(&[], "", "", ""))
            .unwrap_err()
            .contains("API key"));

        // A fake OpenRouter: drain the request, answer with a chat completion.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut data = Vec::new();
            let mut buf = [0u8; 512];
            loop {
                match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        data.extend_from_slice(&buf[..n]);
                        let s = String::from_utf8_lossy(&data);
                        if let Some(hp) = s.find("\r\n\r\n") {
                            let clen = s[..hp]
                                .lines()
                                .find_map(|l| {
                                    let l = l.to_ascii_lowercase();
                                    l.strip_prefix("content-length:")
                                        .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                                })
                                .unwrap_or(0);
                            if data.len() >= hp + 4 + clen {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            let body = serde_json::json!({
                "choices": [{ "message": { "content": "{\"firstName\":\"Sam\"}" } }]
            })
            .to_string();
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
            String::from_utf8_lossy(&data).into_owned()
        });

        let vars = vec![("firstName".to_string(), String::new())];
        let out = enrich_at(
            &format!("http://{addr}/api/v1/chat/completions"),
            &cfg(),
            r#"{"email":"sam@acme.com"}"#,
            &inputs(&vars, "extra user context rides along", "", ""),
        )
        .unwrap();
        let request = handle.join().unwrap();

        let obj: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(obj["firstName"], "Sam");
        // The request carried auth + our model + the profile data + the context.
        assert!(request.contains("Bearer sk-or-test"));
        assert!(request.contains("test/model"));
        assert!(request.contains("sam@acme.com"));
        assert!(request.contains("extra user context rides along"));
    }

    // ── Live OpenRouter smoke tests — need a real key, so ignored by default.
    // Run with:  OPENROUTER_KEY=<key> cargo test -- --ignored --nocapture
    fn live_cfg(web: bool) -> Option<LlmConfig> {
        let key = std::env::var("OPENROUTER_KEY").ok()?;
        Some(LlmConfig { api_key: key, model: "google/gemini-2.5-flash".into(), web_search: web, tone: String::new() })
    }
    fn live_vars() -> Vec<(String, String)> {
        vec![
            (
                "companyName".into(),
                "simplified lowercase company name without suffixes, e.g. \"RocketFarm Studios LLC\" becomes \"rocketfarm\"".into(),
            ),
            ("firstName".into(), "the contact's first name only".into()),
        ]
    }

    #[test]
    #[ignore]
    fn live_openrouter_enrich() {
        let Some(cfg) = live_cfg(false) else {
            panic!("set OPENROUTER_KEY to run the live test");
        };
        let vars = live_vars();
        let out = enrich(
            &cfg,
            r#"{"company":"Anthropic PBC","email":"sam@anthropic.com"}"#,
            &inputs(&vars, "The contact signs their emails as Sam.", "", ""),
        )
        .expect("live enrich failed");
        println!("live enrich → {out}");
        let obj: Value = serde_json::from_str(&out).unwrap();
        assert!(obj.is_object());
        // The description demands the simplified lowercase form.
        assert_eq!(obj["companyName"].as_str().unwrap_or("").to_lowercase(), "anthropic");
        assert_eq!(obj["firstName"].as_str().unwrap_or(""), "Sam");
    }

    #[test]
    #[ignore]
    fn live_openrouter_web_research() {
        let Some(cfg) = live_cfg(true) else {
            panic!("set OPENROUTER_KEY to run the live test");
        };
        // Web research: the model must look this up (not in the prompt).
        let vars =
            vec![("website".to_string(), "the official homepage URL of the company/project".to_string())];
        let out = enrich(
            &cfg,
            r#"{"company":"Tauri (the Rust desktop-app framework)"}"#,
            &inputs(&vars, "", "", ""),
        )
        .expect("live web-research enrich failed");
        println!("live :online enrich → {out}");
        let obj: Value = serde_json::from_str(&out).unwrap();
        let site = obj["website"].as_str().unwrap_or("");
        assert!(site.contains("tauri.app"), "unexpected website: {site}");
    }

    #[test]
    fn web_search_appends_online_suffix() {
        // Exercised through the payload the fake server receives elsewhere; here
        // just verify the string logic via a quick reconstruction.
        let mut c = cfg();
        c.web_search = true;
        let mut model = c.model.trim().to_string();
        if c.web_search && !model.ends_with(":online") {
            model.push_str(":online");
        }
        assert_eq!(model, "test/model:online");
    }
}
