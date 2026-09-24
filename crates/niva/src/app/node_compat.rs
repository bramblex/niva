use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, anyhow};
use serde_json::{Map, Value, json};

use super::options::{NodeCompatConfig, NodeCompatOption};

// Keep this selection list aligned with runtime-files.json and package exports.
const AVAILABLE_MODULES: &[&str] = &[
    "path",
    "os",
    "fs",
    "child_process",
    "events",
    "util",
    "querystring",
    "buffer",
    "url",
    "crypto",
    "zlib",
    "http",
    "https",
    "assert",
    "stream",
    "process",
    "net",
    "dgram",
    "tls",
    "dns",
    "string_decoder",
    "timers",
];
const PREFIX: &str = "__niva_compat/";
const MARKER: &str = "<!-- niva-node-compat -->";

include!(concat!(env!("OUT_DIR"), "/node_compat_assets.rs"));
static NODE_COMPAT_ASSET_DATA: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/node_compat_assets.deflate"));

#[derive(Clone, Debug)]
pub struct NodeCompat {
    modules: BTreeSet<String>,
    importmap: bool,
}

impl NodeCompat {
    pub fn from_option(option: &Option<NodeCompatOption>) -> Result<Option<Self>> {
        let (modules, importmap) = match option {
            None => (None, true),
            Some(NodeCompatOption::Switch(false)) => return Ok(None),
            Some(NodeCompatOption::Switch(true)) => (None, true),
            Some(NodeCompatOption::Config(NodeCompatConfig { modules, importmap })) => {
                (modules.as_ref(), importmap.unwrap_or(true))
            }
        };
        let mut selected = BTreeSet::new();
        let selected_names = modules.cloned().unwrap_or_else(|| {
            AVAILABLE_MODULES
                .iter()
                .map(|module| (*module).to_string())
                .collect()
        });
        for module in selected_names {
            if !AVAILABLE_MODULES.contains(&module.as_str()) {
                return Err(anyhow!("unknown nodeCompat module {module:?}"));
            }
            selected.insert(module);
        }
        Ok(Some(Self {
            modules: selected,
            importmap,
        }))
    }

    pub fn allows_asset(&self, path: &str) -> bool {
        let Some(relative) = path.strip_prefix(PREFIX) else {
            return false;
        };
        if relative == "node-compat.js" {
            return true;
        }
        (!self.modules.is_empty() && NODE_COMPAT_SHARED_ASSETS.contains(&relative))
            || NODE_COMPAT_MODULE_ASSETS.iter().any(|(module, assets)| {
                self.modules.contains(*module) && assets.contains(&relative)
            })
    }

    /// Return one checked-in NodeCompat asset from the compressed build-time
    /// archive. The caller must enforce `allows_asset` before serving it.
    pub fn embedded_asset(path: &str) -> Option<Vec<u8>> {
        use std::io::Read;

        let path = path.trim_start_matches('/');
        let relative = path.strip_prefix(PREFIX).unwrap_or(path);
        let entry = EMBEDDED_NODE_COMPAT_ASSETS
            .iter()
            .find(|entry| entry.path == relative)?;
        let end = entry.offset.checked_add(entry.compressed_len)?;
        let compressed = NODE_COMPAT_ASSET_DATA.get(entry.offset..end)?;
        let mut decoder = flate2::read::DeflateDecoder::new(compressed);
        let mut asset = Vec::with_capacity(entry.raw_len);
        decoder.read_to_end(&mut asset).ok()?;
        (asset.len() == entry.raw_len).then_some(asset)
    }

    pub fn imports(&self) -> BTreeMap<String, String> {
        let mut imports = BTreeMap::new();
        for module in &self.modules {
            let url = format!("/{PREFIX}src/{module}.js");
            imports.insert(module.clone(), url.clone());
            imports.insert(format!("node:{module}"), url.clone());
            if module == "fs" {
                let promises = format!("/{PREFIX}src/fs-promises.js");
                imports.insert("fs/promises".into(), promises.clone());
                imports.insert("node:fs/promises".into(), promises);
            }
            if module == "assert" {
                let strict = format!("/{PREFIX}src/assert-strict.js");
                imports.insert("assert/strict".into(), strict.clone());
                imports.insert("node:assert/strict".into(), strict);
            }
            if module == "stream" {
                let promises = format!("/{PREFIX}src/stream-promises.js");
                imports.insert("stream/promises".into(), promises.clone());
                imports.insert("node:stream/promises".into(), promises);
            }
            if module == "dns" {
                let promises = format!("/{PREFIX}src/dns-promises.js");
                imports.insert("dns/promises".into(), promises.clone());
                imports.insert("node:dns/promises".into(), promises);
            }
            if module == "timers" {
                let promises = format!("/{PREFIX}src/timers-promises.js");
                imports.insert("timers/promises".into(), promises.clone());
                imports.insert("node:timers/promises".into(), promises);
            }
        }
        imports
    }

    pub fn classic_script(&self, original: &[u8]) -> Result<Vec<u8>> {
        let mut script = format!(
            "window.__niva_node_compat_modules={};window.__niva_compat_imports={};\n",
            serde_json::to_string(&self.modules)?,
            serde_json::to_string(&self.imports())?,
        )
        .into_bytes();
        script.extend_from_slice(original);
        Ok(script)
    }

    pub fn rewrite_html(&self, original: &[u8]) -> Result<Vec<u8>> {
        let mut html = String::from_utf8(original.to_vec())
            .map_err(|_| anyhow!("nodeCompat requires UTF-8 HTML"))?;
        if html.contains(MARKER) {
            return Ok(html.into_bytes());
        }

        let existing_map = if self.importmap {
            remove_existing_importmap(&mut html)?
        } else {
            None
        };
        let mut injection = String::from(MARKER);
        if self.importmap {
            let mut map = existing_map.unwrap_or_else(|| json!({}));
            let object = map
                .as_object_mut()
                .ok_or_else(|| anyhow!("importmap must be an object"))?;
            let imports = object
                .entry("imports")
                .or_insert_with(|| Value::Object(Map::new()));
            let imports = imports
                .as_object_mut()
                .ok_or_else(|| anyhow!("importmap imports must be an object"))?;
            for (name, url) in self.imports() {
                imports.entry(name).or_insert(Value::String(url));
            }
            let serialized = serde_json::to_string(&map)?.replace('<', "\\u003c");
            injection.push_str(&format!("<script type=\"importmap\">{serialized}</script>"));
        }
        injection.push_str(&format!(
            "<script src=\"/{PREFIX}node-compat.js\"></script>"
        ));

        let lower = html.to_ascii_lowercase();
        if let Some(start) = lower.find("<head") {
            if let Some(end) = lower[start..].find('>') {
                html.insert_str(start + end + 1, &injection);
                return Ok(html.into_bytes());
            }
        }
        if let Some(start) = lower.find("<html") {
            if let Some(end) = lower[start..].find('>') {
                html.insert_str(start + end + 1, &format!("<head>{injection}</head>"));
                return Ok(html.into_bytes());
            }
        }
        html.insert_str(0, &format!("<head>{injection}</head>"));
        Ok(html.into_bytes())
    }
}

/// Extract one author importmap and move the merged map before any module
/// scripts. This intentionally fails on malformed maps instead of silently
/// changing module resolution.
fn remove_existing_importmap(html: &mut String) -> Result<Option<Value>> {
    let lower = html.to_ascii_lowercase();
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find("<script") {
        let start = cursor + relative;
        let open_end = lower[start..]
            .find('>')
            .map(|index| start + index)
            .ok_or_else(|| anyhow!("unterminated script tag"))?;
        let importmap = script_type_is_importmap(&lower[start..=open_end]);
        if importmap {
            let close_start = lower[open_end + 1..]
                .find("</script")
                .map(|index| open_end + 1 + index)
                .ok_or_else(|| anyhow!("unterminated importmap script"))?;
            let close_end = lower[close_start..]
                .find('>')
                .map(|index| close_start + index + 1)
                .ok_or_else(|| anyhow!("unterminated importmap closing tag"))?;
            let map = serde_json::from_str(&html[open_end + 1..close_start])?;
            html.replace_range(start..close_end, "");
            return Ok(Some(map));
        }
        cursor = open_end + 1;
    }
    Ok(None)
}

fn script_type_is_importmap(tag: &str) -> bool {
    let Some(attributes) = tag.strip_prefix("<script") else {
        return false;
    };
    let bytes = attributes.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] == b'>' {
            break;
        }
        let name_start = cursor;
        while cursor < bytes.len()
            && !bytes[cursor].is_ascii_whitespace()
            && !matches!(bytes[cursor], b'=' | b'>')
        {
            cursor += 1;
        }
        if cursor == name_start {
            cursor += 1;
            continue;
        }
        let name = &attributes[name_start..cursor];
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'=' {
            continue;
        }
        cursor += 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        let quote = bytes
            .get(cursor)
            .copied()
            .filter(|byte| matches!(byte, b'\'' | b'"'));
        if quote.is_some() {
            cursor += 1;
        }
        let value_start = cursor;
        while cursor < bytes.len()
            && if let Some(quote) = quote {
                bytes[cursor] != quote
            } else {
                !bytes[cursor].is_ascii_whitespace() && bytes[cursor] != b'>'
            }
        {
            cursor += 1;
        }
        let value = &attributes[value_start..cursor];
        if quote.is_some() && cursor < bytes.len() {
            cursor += 1;
        }
        if name == "type" && value == "importmap" {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_only() -> NodeCompat {
        NodeCompat::from_option(&Some(NodeCompatOption::Config(NodeCompatConfig {
            modules: Some(vec!["path".into()]),
            importmap: Some(true),
        })))
        .unwrap()
        .unwrap()
    }

    #[test]
    fn subsets_assets_and_aliases() {
        let compat = path_only();
        assert!(compat.allows_asset("__niva_compat/node-compat.js"));
        assert!(compat.allows_asset("__niva_compat/src/path.js"));
        assert!(compat.allows_asset("__niva_compat/src/runtime/vendor.js"));
        assert!(!compat.allows_asset("__niva_compat/src/fs.js"));
        assert_eq!(compat.imports()["node:path"], "/__niva_compat/src/path.js");
    }

    #[test]
    fn absent_option_enables_all_available_modules_and_embedded_bootstrap() {
        let compat = NodeCompat::from_option(&None).unwrap().unwrap();
        assert_eq!(compat.modules.len(), AVAILABLE_MODULES.len());
        for module in AVAILABLE_MODULES {
            assert!(
                compat.modules.contains(*module),
                "missing default module {module}"
            );
        }
        assert_eq!(AVAILABLE_MODULES.len(), 22);
        assert!(compat.allows_asset("__niva_compat/src/net.js"));
        assert!(compat.allows_asset("__niva_compat/src/runtime/vendor.js"));

        let classic = NodeCompat::embedded_asset("__niva_compat/node-compat.js").unwrap();
        let classic = String::from_utf8(classic).unwrap();
        assert!(classic.contains("root.NivaNodeCompatReady"));
        assert!(classic.contains("runtime.vendor"));

        let vendor = NodeCompat::embedded_asset("__niva_compat/src/runtime/vendor.js").unwrap();
        let vendor = String::from_utf8(vendor).unwrap();
        assert!(vendor.contains("niva.node-compat.runtime"));
    }

    #[test]
    fn explicit_false_disables_default_node_compat() {
        assert!(
            NodeCompat::from_option(&Some(NodeCompatOption::Switch(false)))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn subpath_modules_map_to_dedicated_wrappers() {
        let compat = NodeCompat::from_option(&Some(NodeCompatOption::Config(NodeCompatConfig {
            modules: Some(vec![
                "fs".into(),
                "assert".into(),
                "stream".into(),
                "dns".into(),
                "timers".into(),
            ]),
            importmap: Some(true),
        })))
        .unwrap()
        .unwrap();
        let imports = compat.imports();
        assert_eq!(
            imports["node:fs/promises"],
            "/__niva_compat/src/fs-promises.js"
        );
        assert_eq!(
            imports["node:assert/strict"],
            "/__niva_compat/src/assert-strict.js"
        );
        assert_eq!(
            imports["node:stream/promises"],
            "/__niva_compat/src/stream-promises.js"
        );
        assert_eq!(
            imports["node:dns/promises"],
            "/__niva_compat/src/dns-promises.js"
        );
        assert_eq!(
            imports["node:timers/promises"],
            "/__niva_compat/src/timers-promises.js"
        );
        for asset in [
            "fs-promises.js",
            "assert-strict.js",
            "stream-promises.js",
            "dns-promises.js",
            "timers-promises.js",
        ] {
            assert!(compat.allows_asset(&format!("__niva_compat/src/{asset}")));
        }
    }

    #[test]
    fn recognizes_importmap_attribute_without_matching_data_type() {
        assert!(script_type_is_importmap(
            "<script defer type = 'importmap'>"
        ));
        assert!(!script_type_is_importmap(
            "<script data-type=\"importmap\">"
        ));
    }

    #[test]
    fn merges_user_importmap_before_modules_and_is_idempotent() {
        let compat = path_only();
        let input = br#"<html><head><script type="module" src="/app.js"></script><script type="importmap">{"imports":{"path":"/mine.js"}}</script></head></html>"#;
        let once = compat.rewrite_html(input).unwrap();
        let text = String::from_utf8(once.clone()).unwrap();
        assert!(text.find("type=\"importmap\"").unwrap() < text.find("type=\"module\"").unwrap());
        let map_start =
            text.find("<script type=\"importmap\">").unwrap() + "<script type=\"importmap\">".len();
        let map_end = text[map_start..].find("</script>").unwrap() + map_start;
        let map: Value = serde_json::from_str(&text[map_start..map_end]).unwrap();
        assert_eq!(map["imports"]["path"], "/mine.js");
        assert_eq!(map["imports"]["node:path"], "/__niva_compat/src/path.js");
        assert_eq!(compat.rewrite_html(&once).unwrap(), once);
    }
}
