use std::collections::{HashMap, HashSet};

use anyhow::{Result, anyhow};
use url::Url;

/// API grants for one native window, keyed by exact web origin.
#[derive(Default)]
pub struct WindowPermissions(HashMap<String, HashSet<String>>);

impl WindowPermissions {
    pub fn new(config: &HashMap<String, Vec<String>>) -> Result<Self> {
        let mut grants = HashMap::new();
        for (origin, methods) in config {
            let url = Url::parse(origin)
                .map_err(|err| anyhow!("invalid permission origin {origin:?}: {err}"))?;
            if !matches!(url.scheme(), "http" | "https")
                || url.username() != ""
                || url.password().is_some()
                || url.path() != "/"
                || url.query().is_some()
                || url.fragment().is_some()
            {
                return Err(anyhow!(
                    "permission key must be an exact HTTP origin: {origin}"
                ));
            }
            let canonical = url.origin().ascii_serialization();
            if canonical == "null" {
                return Err(anyhow!("opaque permission origin: {origin}"));
            }
            let entry = grants.entry(canonical).or_insert_with(HashSet::new);
            for method in methods {
                let valid = method.split_once('.').is_some_and(|(namespace, name)| {
                    !namespace.is_empty()
                        && !name.is_empty()
                        && !namespace.contains('*')
                        && (name == "*" || !name.contains('*'))
                });
                if !valid {
                    return Err(anyhow!("invalid API permission {method:?}"));
                }
                entry.insert(method.clone());
            }
        }
        Ok(Self(grants))
    }

    pub fn allows(&self, source_url: &str, method: &str) -> bool {
        let Ok(source) = Url::parse(source_url) else {
            return false;
        };
        if !matches!(source.scheme(), "http" | "https") {
            return false;
        }
        let origin = source.origin().ascii_serialization();
        self.0.get(&origin).is_some_and(|methods| {
            methods.contains(method)
                || method
                    .split_once('.')
                    .is_some_and(|(namespace, _)| methods.contains(&format!("{namespace}.*")))
        })
    }

    /// Whether an exact HTTP(S) origin has at least one configured API grant.
    /// IPC lifecycle messages use this to scope control of a granted origin;
    /// an empty methods list grants no API or lifecycle access.
    pub fn allows_origin(&self, source_url: &str) -> bool {
        let Ok(source) = Url::parse(source_url) else {
            return false;
        };
        if !matches!(source.scheme(), "http" | "https") {
            return false;
        }
        let origin = source.origin().ascii_serialization();
        self.0
            .get(&origin)
            .is_some_and(|methods| !methods.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_origin_and_method_only() {
        let permissions = WindowPermissions::new(&HashMap::from([
            (
                "https://example.test:8443".to_string(),
                vec!["dialog.*".to_string()],
            ),
            (
                "https://other.test".to_string(),
                vec!["os.info".to_string()],
            ),
        ]))
        .unwrap();
        assert!(permissions.allows("https://example.test:8443/page", "dialog.alert"));
        assert!(!permissions.allows("https://example.test/page", "dialog.alert"));
        assert!(!permissions.allows("https://example.test.evil:8443", "dialog.alert"));
        assert!(!permissions.allows("https://other.test", "os.dirs"));
        assert!(permissions.allows("https://other.test", "os.info"));
        assert!(permissions.allows_origin("https://other.test/page"));
    }

    #[test]
    fn empty_method_list_does_not_grant_origin_lifecycle_access() {
        let permissions = WindowPermissions::new(&HashMap::from([(
            "https://empty.test".to_string(),
            Vec::new(),
        )]))
        .unwrap();

        assert!(!permissions.allows("https://empty.test/page", "os.info"));
        assert!(!permissions.allows_origin("https://empty.test/page"));
    }

    #[test]
    fn origin_lifecycle_access_requires_a_real_method_grant() {
        let permissions = WindowPermissions::new(&HashMap::from([
            ("https://empty.test".to_string(), Vec::new()),
            (
                "https://granted.test".to_string(),
                vec!["os.info".to_string()],
            ),
        ]))
        .unwrap();

        assert!(!permissions.allows_origin("https://empty.test"));
        assert!(permissions.allows_origin("https://granted.test/page"));
        assert!(permissions.allows("https://granted.test/page", "os.info"));
    }

    #[test]
    fn rejects_non_origin_permission_keys() {
        for origin in [
            "https://example.test/path",
            "file:///tmp/page.html",
            "https://user@example.test",
        ] {
            let config = HashMap::from([(origin.to_string(), vec!["os.info".to_string()])]);
            assert!(WindowPermissions::new(&config).is_err(), "{origin}");
        }
    }
}
