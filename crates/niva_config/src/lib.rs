//! The same configuration preflight is used by runtime, packager and editor.
use anyhow::{Result, anyhow, ensure};
use serde_json::Value;

pub fn canonical_uuid(uuid: &str) -> Result<String> {
    ensure!(
        uuid.len() == 36
            && uuid
                .bytes()
                .enumerate()
                .all(|(i, b)| if matches!(i, 8 | 13 | 18 | 23) {
                    b == b'-'
                } else {
                    b.is_ascii_hexdigit()
                }),
        "uuid must be a canonical UUID"
    );
    Ok(uuid.to_ascii_lowercase())
}

pub fn merge(dest: Value, src: Value) -> Value {
    match (dest, src) {
        (dest, Value::Null) => dest,
        (Value::Object(mut dest), Value::Object(src)) => {
            for (key, value) in src {
                let old = dest.remove(&key).unwrap_or(Value::Null);
                dest.insert(key, merge(old, value));
            }
            Value::Object(dest)
        }
        (_, src) => src,
    }
}

pub fn for_platform(config: &Value, platform: &str) -> Result<Value> {
    ensure!(config.is_object(), "configuration must be an object");
    let mut effective = config.clone();
    if let Some(overrides) = config.get(platform) {
        ensure!(
            overrides.is_object(),
            "{platform} overrides must be an object"
        );
        effective = merge(effective, overrides.clone());
    }
    validate(&effective)?;
    Ok(effective)
}

pub fn validate_all(config: &Value) -> Result<()> {
    for platform in ["windows", "macos", "linux"] {
        for_platform(config, platform)?;
    }
    Ok(())
}

fn fields(
    value: &Value,
    prefix: &str,
    names: &str,
    valid: fn(&Value) -> bool,
    kind: &str,
) -> Result<()> {
    for name in names.split_whitespace() {
        if let Some(field) = value.get(name) {
            ensure!(
                field.is_null() || valid(field),
                "{prefix}{name} must be {kind}"
            );
        }
    }
    Ok(())
}

fn validate(config: &Value) -> Result<()> {
    let name = config
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("name must be a string"))?;
    ensure!(!name.trim().is_empty(), "name must not be empty");
    canonical_uuid(
        config
            .get("uuid")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("uuid must be a string"))?,
    )?;
    fields(config, "", "icon version", Value::is_string, "a string")?;
    fields(
        config,
        "",
        "injectCommonJs injectEsm",
        Value::is_boolean,
        "a boolean",
    )?;
    fields(
        config,
        "",
        "window api debug build",
        Value::is_object,
        "an object",
    )?;
    if let Some(api) = config.get("api").filter(|v| !v.is_null()) {
        for (field, max) in [("maxQueue", 4096u64), ("timeoutMs", 86_400_000)] {
            if let Some(value) = api.get(field).filter(|v| !v.is_null()) {
                ensure!(
                    value.as_u64().is_some_and(|v| v > 0 && v <= max),
                    "api.{field} must be an integer between 1 and {max}"
                );
            }
        }
    }
    if let Some(window) = config.get("window").filter(|v| !v.is_null()) {
        fields(
            window,
            "window.",
            "entry title icon theme taskbarIcon tabbingIdentifier",
            Value::is_string,
            "a string",
        )?;
        fields(
            window,
            "window.",
            "devtools resizable minimizable maximizable closable fullscreen maximized visible transparent decorations alwaysOnTop alwaysOnBottom visibleOnAllWorkspaces focused contentProtection skipTaskbar undecoratedShadow movableByWindowBackground titleBarTransparent titleBarHidden titleBarButtonsHidden titleHidden fullSizeContentView disallowHiDpi hasShadow automaticWindowTabbing",
            Value::is_boolean,
            "a boolean",
        )?;
        for key in [
            "size",
            "minSize",
            "maxSize",
            "resizeIncrements",
            "position",
            "trafficLightInset",
        ] {
            if let Some(size) = window.get(key).filter(|v| !v.is_null()) {
                let dimension = matches!(key, "size" | "minSize" | "maxSize" | "resizeIncrements");
                for field in if dimension {
                    ["width", "height"]
                } else {
                    ["x", "y"]
                } {
                    ensure!(
                        size.get(field)
                            .and_then(Value::as_f64)
                            .is_some_and(|n| n.is_finite() && (!dimension || n > 0.0)),
                        "window.{key}.{field} is invalid"
                    );
                }
            }
        }
        for key in ["parentWindow", "ownerWindow"] {
            if let Some(value) = window.get(key).filter(|v| !v.is_null()) {
                ensure!(
                    value.as_u64().is_some_and(|id| id <= 255),
                    "window.{key} must be a window ID"
                );
            }
        }
        ensure!(
            !(window.get("parentWindow").is_some_and(|v| !v.is_null())
                && window.get("ownerWindow").is_some_and(|v| !v.is_null())),
            "parentWindow and ownerWindow are mutually exclusive"
        );
        if let Some(permissions) = window.get("permissions") {
            for (origin, methods) in permissions
                .as_object()
                .ok_or_else(|| anyhow!("window.permissions must be an object"))?
            {
                let url = url::Url::parse(origin)?;
                ensure!(
                    matches!(url.scheme(), "http" | "https")
                        && url.username().is_empty()
                        && url.password().is_none()
                        && url.path() == "/"
                        && url.query().is_none()
                        && url.fragment().is_none(),
                    "permission key must be an exact HTTP origin: {origin}"
                );
                for method in methods
                    .as_array()
                    .ok_or_else(|| anyhow!("permission methods must be an array"))?
                {
                    ensure!(
                        method.as_str().and_then(|s| s.split_once('.')).is_some_and(
                            |(ns, name)| !ns.is_empty()
                                && !ns.contains('*')
                                && !name.is_empty()
                                && (name == "*" || !name.contains('*'))
                        ),
                        "invalid permission method"
                    );
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn all_entrypoints_reject_invalid_effective_configuration() {
        let base = json!({"name":"App", "uuid":"a51c1728-d174-42d4-8f57-7d296c966b51"});
        for patch in [
            json!({"uuid":"test"}),
            json!({"api":{"maxQueue":0}}),
            json!({"api":{"timeoutMs":-1}}),
            json!({"window":{"visible":"false"}}),
            json!({"window":{"size":{"width":0,"height":10}}}),
            json!({"window":{"permissions":{"https://example.org/path":["fs.*"]}}}),
        ] {
            assert!(validate_all(&merge(base.clone(), patch.clone())).is_err());
            assert!(validate_all(&merge(base.clone(), json!({"windows":patch}))).is_err());
        }
        assert!(validate_all(&base).is_ok());
    }
}
