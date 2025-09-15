use serde_json::Value;

/// Keys whose values should be redacted in logs (case-insensitive)
const SENSITIVE_KEYS: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "x-api-key",
    "x-javelin-apikey",
    "api_key",
    "apikey",
    "token",
    "access_token",
    "refresh_token",
    "password",
    "secret",
    "cookie",
    "set-cookie",
];

/// Redact sensitive values and truncate long strings in a JSON value for logging.
pub fn sanitize_json_for_log(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (k, v) in map {
                let redacted = if is_sensitive_key(k) {
                    Value::String("***REDACTED***".into())
                } else {
                    sanitize_json_for_log(v)
                };
                out.insert(k.clone(), redacted);
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(sanitize_json_for_log).collect()),
        Value::String(s) => Value::String(truncate_for_log(s)),
        other => other.clone(),
    }
}

/// Return a short, safe preview of a string for logs.
pub fn truncate_for_log(input: &str) -> String {
    const MAX: usize = 128;
    if input.len() <= MAX {
        input.to_string()
    } else {
        format!("{}… ({} chars)", &input[..MAX], input.len())
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    SENSITIVE_KEYS.iter().any(|s| *s == k)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_redacts_and_truncates() {
        let v = json!({
            "Authorization": "Bearer abcdefghijklmnopqrstuvwxyz0123456789",
            "token": "secret-token",
            "nested": {"x-api-key": "123", "ok": "a".repeat(200)}
        });
        let s = sanitize_json_for_log(&v);
        assert_eq!(s["Authorization"], "***REDACTED***");
        assert_eq!(s["token"], "***REDACTED***");
        assert_eq!(s["nested"]["x-api-key"], "***REDACTED***");
        assert!(s["nested"]["ok"].as_str().unwrap().len() <= 160);
    }
}
