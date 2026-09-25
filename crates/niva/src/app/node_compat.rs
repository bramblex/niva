use std::{collections::BTreeMap, io::Read, sync::OnceLock};

use anyhow::{Result, anyhow};
use serde_json::{Map, Value, json};

const PREFIX: &str = "__niva_runtime/";
const BOOTSTRAP_ENTRY: &str = "__bootstrap__/bootstrap.js";
const MARKER: &str = "<!-- niva-runtime-importmap -->";

include!(concat!(env!("OUT_DIR"), "/node_compat_assets.rs"));
static RUNTIME_ASSET_DATA: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/node_compat_assets.deflate"));

#[derive(Clone, Debug, Default)]
pub struct NodeCompat {
    inject_esm: bool,
}

impl NodeCompat {
    /// A CSP execution nonce, independent of bridge authentication credentials.
    /// Shared by native initialization and local HTML policy preparation.
    pub fn csp_nonce() -> Result<&'static str> {
        static NONCE: OnceLock<std::result::Result<String, String>> = OnceLock::new();
        match NONCE.get_or_init(|| {
            let mut bytes = [0u8; 16];
            getrandom::fill(&mut bytes).map_err(|error| error.to_string())?;
            Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
        }) {
            Ok(nonce) => Ok(nonce),
            Err(error) => Err(anyhow!("cannot create runtime CSP nonce: {error}")),
        }
    }

    pub fn new(inject_esm: bool) -> Self {
        Self { inject_esm }
    }

    /// Decompress the single bootstrap copy for trusted WebView initialization.
    /// It is intentionally not exposed through the runtime asset route.
    pub fn bootstrap_script() -> Result<String> {
        let bytes = Self::decompress_entry(BOOTSTRAP_ENTRY)?;
        String::from_utf8(bytes)
            .map_err(|error| anyhow!("embedded runtime bootstrap is not UTF-8: {error}"))
    }

    pub fn allows_asset(&self, path: &str) -> bool {
        if !self.inject_esm {
            return false;
        }
        let Some(relative) = runtime_asset_relative(path) else {
            return false;
        };
        NODE_COMPAT_SHARED_ASSETS.contains(&relative)
            || NODE_COMPAT_MODULE_ASSETS
                .iter()
                .any(|(_, assets)| assets.contains(&relative))
    }

    /// Return one embedded ESM facade/chunk. Callers must check `allows_asset`
    /// first; bootstrap bytes remain accessible only via `bootstrap_script`.
    pub fn embedded_asset(path: &str) -> Option<Vec<u8>> {
        let relative = runtime_asset_relative(path)?;
        Self::decompress_entry(relative).ok()
    }

    fn decompress_entry(path: &str) -> Result<Vec<u8>> {
        let entry = EMBEDDED_NODE_COMPAT_ASSETS
            .iter()
            .find(|entry| entry.path == path)
            .ok_or_else(|| anyhow!("embedded runtime asset is missing: {path}"))?;
        let end = entry
            .offset
            .checked_add(entry.compressed_len)
            .ok_or_else(|| anyhow!("embedded runtime asset range overflow: {path}"))?;
        let compressed = RUNTIME_ASSET_DATA
            .get(entry.offset..end)
            .ok_or_else(|| anyhow!("embedded runtime asset is truncated: {path}"))?;
        let mut decoder = flate2::read::DeflateDecoder::new(compressed);
        let mut bytes = Vec::with_capacity(entry.raw_len);
        decoder.read_to_end(&mut bytes)?;
        if bytes.len() != entry.raw_len {
            return Err(anyhow!("embedded runtime asset size mismatch: {path}"));
        }
        Ok(bytes)
    }

    pub fn imports(&self) -> BTreeMap<String, String> {
        let mut imports = BTreeMap::new();
        if !self.inject_esm {
            return imports;
        }
        for (module, assets) in NODE_COMPAT_MODULE_ASSETS {
            if assets.is_empty() {
                continue;
            }
            let facade = format!("/{PREFIX}{}", assets[0]);
            imports.insert((*module).to_owned(), facade.clone());
            imports.insert(format!("node:{module}"), facade);
        }
        imports
    }

    pub fn rewrite_html(&self, original: &[u8]) -> Result<Vec<u8>> {
        if !self.inject_esm {
            return Ok(original.to_vec());
        }
        let mut html = String::from_utf8(original.to_vec())
            .map_err(|_| anyhow!("Niva ESM runtime requires UTF-8 HTML"))?;
        if html.contains(MARKER) {
            return Ok(html.into_bytes());
        }
        let existing_map = remove_existing_importmap(&mut html)?;
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
        let injection = format!("{MARKER}<script type=\"importmap\">{serialized}</script>");
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

fn runtime_asset_relative(path: &str) -> Option<&str> {
    let normalized = path.trim_start_matches('/');
    let relative = normalized.strip_prefix(PREFIX)?;
    if !relative.starts_with("esm/")
        || !relative.ends_with(".mjs")
        || relative.contains("..")
        || relative.contains('\\')
    {
        return None;
    }
    Some(relative)
}

fn remove_existing_importmap(html: &mut String) -> Result<Option<Value>> {
    let lower = html.to_ascii_lowercase();
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find("<script") {
        let start = cursor + relative;
        if let Some(comment) = lower[cursor..start].find("<!--") {
            let comment = cursor + comment;
            cursor = lower[comment + 4..]
                .find("-->")
                .map(|end| comment + 4 + end + 3)
                .unwrap_or(lower.len());
            continue;
        }
        if lower
            .as_bytes()
            .get(start + 7)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'>')
        {
            cursor = start + 7;
            continue;
        }
        let open_end = lower[start..]
            .find('>')
            .map(|index| start + index)
            .ok_or_else(|| anyhow!("unterminated script tag"))?;
        if script_type_is_importmap(&lower[start..=open_end]) {
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
        // Literal markup inside a script body is not another HTML element.
        cursor = lower[open_end + 1..]
            .find("</script>")
            .map(|end| open_end + 1 + end + "</script>".len())
            .unwrap_or(lower.len());
    }
    Ok(None)
}

fn script_type_is_importmap(tag: &str) -> bool {
    let Some(attributes) = tag.strip_prefix("<script") else {
        return false;
    };
    let bytes = attributes.as_bytes();
    if bytes
        .first()
        .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'>')
    {
        return false;
    }
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
        if cursor >= bytes.len() {
            break;
        }
        let quote = if matches!(bytes[cursor], b'\'' | b'"') {
            let quote = bytes[cursor];
            cursor += 1;
            Some(quote)
        } else {
            None
        };
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

    #[test]
    fn bootstrap_is_embedded_but_never_served_as_an_asset() {
        let bootstrap = NodeCompat::bootstrap_script().unwrap();
        assert!(!bootstrap.is_empty());
        let compat = NodeCompat::new(true);
        assert!(!compat.allows_asset("/__niva_runtime/bootstrap.js"));
        assert!(NodeCompat::embedded_asset("/__niva_runtime/__bootstrap__/bootstrap.js").is_none());
        assert!(
            NodeCompat::embedded_asset("/__niva_runtime/../__bootstrap__/bootstrap.js").is_none()
        );
        assert!(compat.allows_asset("/__niva_runtime/esm/path.mjs"));
    }

    #[test]
    fn inject_esm_is_opt_in_and_preserves_user_importmap_entries() {
        let off = NodeCompat::new(false);
        let source = b"<html><head></head><body>x</body></html>";
        assert_eq!(off.rewrite_html(source).unwrap(), source);
        let compat = NodeCompat::new(true);
        let source = br#"<html><head><script type="module" src="/app.mjs"></script><script type="importmap">{"imports":{"path":"/mine.mjs"}}</script></head></html>"#;
        let once = compat.rewrite_html(source).unwrap();
        let text = String::from_utf8(once.clone()).unwrap();
        assert!(text.find("type=\"importmap\"").unwrap() < text.find("type=\"module\"").unwrap());
        assert!(text.contains("/mine.mjs"));
        assert!(text.contains("/__niva_runtime/esm/path.mjs"));
        assert_eq!(compat.rewrite_html(&once).unwrap(), once);
    }

    #[test]
    fn rejects_non_utf8_and_malformed_importmaps() {
        assert!(NodeCompat::new(true).rewrite_html(&[0xff]).is_err());
        assert!(
            NodeCompat::new(true)
                .rewrite_html(br#"<script type="importmap">not-json</script>"#)
                .is_err()
        );
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
    fn importmap_rewrite_preserves_comments_and_javascript_literals() {
        let source = br#"<html><head><!-- <script type="importmap">invalid</script> --><script>const tag = '<script type="importmap">';</script><script type="importmap">{"imports":{"fs":"/mine.mjs"}}</script></head></html>"#;
        let result =
            String::from_utf8(NodeCompat::new(true).rewrite_html(source).unwrap()).unwrap();
        assert!(result.contains("<!-- <script type=\"importmap\">invalid</script> -->"));
        assert!(result.contains("const tag = '<script type=\"importmap\">';"));
        assert!(result.contains("\"fs\":\"/mine.mjs\""));
        assert!(!script_type_is_importmap("<scripture type=\"importmap\">"));
    }
}
