//! Cross-platform PE resource assembly.
//!
//! `editpe` edits the PE resource directory as bytes, so this module is usable
//! on macOS/Linux when producing a Windows executable. Resource IDs and the
//! language ID match the former Windows-only packager.

use std::{
    ffi::{OsStr, OsString},
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::{Context, Result, anyhow, bail};
use editpe::{
    DataDirectoryType, Image, ResourceData, ResourceDirectory, ResourceEntry, ResourceEntryName,
    ResourceTable,
};
use serde_json::Value;

const LANG_EN_US: u16 = 1033;
const RT_ICON: u32 = 3;
const RT_RCDATA: u32 = 10;
const RT_GROUP_ICON: u32 = 14;
const RT_VERSION: u32 = 16;
const GROUP_ICON_ID: u32 = 1;
const FIRST_ICON_ID: u16 = 8;
const OLD_ICON_IDS: std::ops::RangeInclusive<u16> = 1..=7;

/// Assemble the runtime PE with application resources and write `output`.
///
/// `icon` points to a PNG file, as in the existing Niva Windows build path.
/// `config` is the parsed `niva.json`; version resource values follow the
/// current `windows-version-info-template.ts` fields.
pub fn assemble(
    runtime: &Path,
    output: &Path,
    config: &Value,
    indexes: &[u8],
    data: &[u8],
    icon: Option<&Path>,
) -> Result<()> {
    ensure_distinct_paths(runtime, output)?;

    let mut image = Image::parse_file(runtime)
        .with_context(|| format!("parse runtime PE {}", runtime.display()))?;
    let mut resources = image.resource_directory().cloned().unwrap_or_default();

    set_resource(
        &mut resources,
        RT_RCDATA,
        ResourceEntryName::from_string("RESOURCE_INDEXES"),
        LANG_EN_US,
        indexes,
    )?;
    set_resource(
        &mut resources,
        RT_RCDATA,
        ResourceEntryName::from_string("RESOURCE_DATA"),
        LANG_EN_US,
        data,
    )?;

    if let Some(icon_path) = icon {
        let png = fs::read(icon_path)
            .with_context(|| format!("read PNG icon {}", icon_path.display()))?;
        let ico = win_packager::icon::png_to_ico_bytes(&png)
            .with_context(|| format!("convert PNG icon {} to ICO", icon_path.display()))?;
        replace_main_icon(&mut resources, &ico)?;
    }

    let version_rc = version_info_rc(config)?;
    let version_info = win_packager::version_info::compile_version_info_rc(&version_rc)
        .context("compile VERSIONINFO resource")?;
    set_resource(
        &mut resources,
        RT_VERSION,
        ResourceEntryName::ID(1),
        LANG_EN_US,
        &version_info,
    )?;

    let strip_signature = image
        .data_directory(DataDirectoryType::CertificateTable)
        .is_some_and(|directory| directory.virtual_address != 0 || directory.size != 0);
    image
        .set_resource_directory(resources)
        .context("write PE resource directory")?;

    // Adding a resource section moves any overlay (including an Authenticode
    // certificate table). Resource edits invalidate the old signature, so
    // clear the PE security-directory pointer before writing the unsigned exe.
    let mut bytes = image.data().to_vec();
    if strip_signature {
        clear_certificate_directory(&mut bytes)?;
    }

    // Parse the final bytes once more before touching the requested output.
    // This catches resource-section/layout errors in the editor on every host.
    let verify = Image::parse(bytes.clone()).context("validate assembled PE")?;
    verify_required_resources(verify.resource_directory(), indexes, data, &version_info)?;

    write_output(output, &bytes)
}

fn ensure_distinct_paths(runtime: &Path, output: &Path) -> Result<()> {
    let runtime_abs = fs::canonicalize(runtime)
        .with_context(|| format!("resolve runtime PE {}", runtime.display()))?;
    let output_abs = if output.exists() {
        fs::canonicalize(output)
            .with_context(|| format!("resolve output PE {}", output.display()))?
    } else {
        let parent = output
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        if parent.exists() {
            let parent = fs::canonicalize(parent)
                .with_context(|| format!("resolve output directory {}", parent.display()))?;
            parent.join(
                output
                    .file_name()
                    .ok_or_else(|| anyhow!("output path has no filename"))?,
            )
        } else {
            // A missing output path cannot alias the existing runtime. Its
            // parent can be created once the PE has passed validation.
            return Ok(());
        }
    };
    if runtime_abs == output_abs {
        bail!("runtime and output PE paths must differ");
    }
    Ok(())
}

fn set_resource(
    resources: &mut ResourceDirectory,
    type_id: u32,
    name: ResourceEntryName,
    lang_id: u16,
    bytes: &[u8],
) -> Result<()> {
    let type_table = ensure_table(resources.root_mut(), ResourceEntryName::ID(type_id))?;
    let name_table = ensure_table(type_table, name)?;
    let mut value = ResourceData::default();
    value.set_data(bytes.to_vec());
    name_table.insert(
        ResourceEntryName::ID(u32::from(lang_id)),
        ResourceEntry::Data(value),
    );
    Ok(())
}

fn ensure_table(table: &mut ResourceTable, name: ResourceEntryName) -> Result<&mut ResourceTable> {
    if table.get(&name).is_none() {
        table.insert(name.clone(), ResourceEntry::Table(ResourceTable::default()));
    }
    match table.get_mut(&name) {
        Some(ResourceEntry::Table(table)) => Ok(table),
        Some(ResourceEntry::Data(_)) => bail!("PE resource path collides with a data entry"),
        None => unreachable!("resource entry was inserted above"),
    }
}

fn replace_main_icon(resources: &mut ResourceDirectory, ico: &[u8]) -> Result<()> {
    let entries = split_ico(ico)?;
    let final_id = FIRST_ICON_ID
        .checked_add(u16::try_from(entries.len() - 1).context("ICO has too many images")?)
        .ok_or_else(|| anyhow!("ICO icon resource IDs exceed 65535"))?;
    let _ = final_id;

    // Match the existing ResourceHacker invocation: it removes the runtime's
    // icon IDs 1..7 and writes the new images beginning at ID 8.
    for old_id in OLD_ICON_IDS {
        remove_resource(
            resources.root_mut(),
            RT_ICON,
            ResourceEntryName::ID(u32::from(old_id)),
            LANG_EN_US,
        )?;
    }
    // Also remove icons referenced by the current group 1. This makes a
    // second assembly idempotent if the prior group used a different ID range.
    for old_id in group_icon_ids(resources, GROUP_ICON_ID, LANG_EN_US)? {
        remove_resource(
            resources.root_mut(),
            RT_ICON,
            ResourceEntryName::ID(u32::from(old_id)),
            LANG_EN_US,
        )?;
    }

    let mut group = Vec::with_capacity(6 + entries.len() * 14);
    group.extend_from_slice(&0u16.to_le_bytes());
    group.extend_from_slice(&1u16.to_le_bytes());
    group.extend_from_slice(
        &u16::try_from(entries.len())
            .context("ICO has too many images for a group icon")?
            .to_le_bytes(),
    );

    for (index, entry) in entries.iter().enumerate() {
        let id = FIRST_ICON_ID
            .checked_add(u16::try_from(index).context("ICO has too many images")?)
            .ok_or_else(|| anyhow!("ICO icon resource ID exceeds 65535"))?;
        set_resource(
            resources,
            RT_ICON,
            ResourceEntryName::ID(u32::from(id)),
            LANG_EN_US,
            entry.data,
        )?;
        group.extend_from_slice(&entry.header);
        group.extend_from_slice(&id.to_le_bytes());
    }

    set_resource(
        resources,
        RT_GROUP_ICON,
        ResourceEntryName::ID(GROUP_ICON_ID),
        LANG_EN_US,
        &group,
    )
}

struct IcoEntry<'a> {
    header: [u8; 12],
    data: &'a [u8],
}

fn split_ico(ico: &[u8]) -> Result<Vec<IcoEntry<'_>>> {
    if ico.len() < 6 || read_u16(ico, 0)? != 0 || read_u16(ico, 2)? != 1 {
        bail!("invalid ICO header");
    }
    let count = usize::from(read_u16(ico, 4)?);
    if count == 0 {
        bail!("ICO has no images");
    }
    let table_end = 6usize
        .checked_add(
            count
                .checked_mul(16)
                .ok_or_else(|| anyhow!("ICO table overflow"))?,
        )
        .filter(|end| *end <= ico.len())
        .ok_or_else(|| anyhow!("ICO entry table is truncated"))?;

    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let start = 6 + index * 16;
        if ico[start + 3] != 0 {
            bail!("ICO entry {index} has a nonzero reserved byte");
        }
        let length = usize::try_from(read_u32(ico, start + 8)?)?;
        let offset = usize::try_from(read_u32(ico, start + 12)?)?;
        let end = offset
            .checked_add(length)
            .filter(|end| offset >= table_end && *end <= ico.len() && length > 0)
            .ok_or_else(|| anyhow!("ICO entry {index} image is outside the file"))?;
        entries.push(IcoEntry {
            header: ico[start..start + 12].try_into().unwrap(),
            data: &ico[offset..end],
        });
    }
    Ok(entries)
}

fn group_icon_ids(resources: &ResourceDirectory, group_id: u32, lang_id: u16) -> Result<Vec<u16>> {
    let Some(bytes) = get_resource(
        resources,
        RT_GROUP_ICON,
        &ResourceEntryName::ID(group_id),
        lang_id,
    )?
    else {
        return Ok(Vec::new());
    };
    if bytes.len() < 6 || read_u16(bytes, 0)? != 0 || read_u16(bytes, 2)? != 1 {
        bail!("existing group icon resource has an invalid header");
    }
    let count = usize::from(read_u16(bytes, 4)?);
    let expected_len = 6usize
        .checked_add(
            count
                .checked_mul(14)
                .ok_or_else(|| anyhow!("group icon size overflow"))?,
        )
        .ok_or_else(|| anyhow!("group icon size overflow"))?;
    if count == 0 || expected_len > bytes.len() {
        bail!("existing group icon resource is truncated");
    }
    (0..count)
        .map(|index| read_u16(bytes, 6 + index * 14 + 12))
        .collect()
}

fn remove_resource(
    root: &mut ResourceTable,
    type_id: u32,
    name: ResourceEntryName,
    lang_id: u16,
) -> Result<()> {
    let type_name = ResourceEntryName::ID(type_id);
    let Some(type_entry) = root.get_mut(&type_name) else {
        return Ok(());
    };
    let Some(type_table) = type_entry.as_table_mut() else {
        bail!("PE resource type {type_id} is not a table");
    };
    let Some(name_entry) = type_table.get_mut(&name) else {
        return Ok(());
    };
    let Some(name_table) = name_entry.as_table_mut() else {
        bail!("PE resource name is not a language table");
    };
    name_table.remove(ResourceEntryName::ID(u32::from(lang_id)));
    if name_table.entries().is_empty() {
        type_table.remove(name);
    }
    if type_table.entries().is_empty() {
        root.remove(type_name);
    }
    Ok(())
}

fn version_info_rc(config: &Value) -> Result<String> {
    let version = config.get("version").and_then(Value::as_str).unwrap_or("");
    let mut components = version
        .chars()
        .filter(|ch| ch.is_ascii_digit() || *ch == '.')
        .collect::<String>()
        .split('.')
        .map(|part| {
            if part.is_empty() {
                Ok(0)
            } else {
                part.parse::<u32>()
                    .context("version component is out of range")
            }
        })
        .collect::<Result<Vec<_>>>()?;
    components.resize(components.len().max(4), 0);
    components.truncate(4);
    let version_numbers = components
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");

    let name = config.get("name").and_then(Value::as_str).unwrap_or("");
    let company = meta_string(config, "companyName").unwrap_or("");
    let description = meta_string(config, "description").unwrap_or("");
    let copyright = meta_string(config, "copyright").unwrap_or("");
    let version_string = quote_rc_string(version)?;
    let name = quote_rc_string(name)?;
    let company = quote_rc_string(company)?;
    let description = quote_rc_string(description)?;
    let copyright = quote_rc_string(copyright)?;

    Ok(format!(
        r#"1 VERSIONINFO
FILEVERSION {version_numbers}
PRODUCTVERSION {version_numbers}
FILEOS 0x40004
FILETYPE 0x1
{{
BLOCK "StringFileInfo"
{{
  BLOCK "040904b0"
  {{
    VALUE "CompanyName", {company}
    VALUE "FileDescription", {description}
    VALUE "FileVersion", {version_string}
    VALUE "InternalName", "niva.exe"
    VALUE "LegalCopyright", {copyright}
    VALUE "OriginalFilename", "niva.exe"
    VALUE "ProductName", {name}
    VALUE "ProductVersion", {version_string}
    VALUE "SquirrelAwareVersion", "1"
  }}
}}
BLOCK "VarFileInfo"
{{
  VALUE "Translation", 0x0409 0x04B0
}}
}}"#
    ))
}

fn meta_string<'a>(config: &'a Value, key: &str) -> Option<&'a str> {
    config
        .get("meta")?
        .get(key)?
        .as_str()
        .filter(|value| !value.is_empty())
}

fn quote_rc_string(value: &str) -> Result<String> {
    serde_json::to_string(value).context("quote VERSIONINFO string")
}

fn get_resource<'a>(
    resources: &'a ResourceDirectory,
    type_id: u32,
    name: &ResourceEntryName,
    lang_id: u16,
) -> Result<Option<&'a [u8]>> {
    let Some(entry) = resources.root().get(ResourceEntryName::ID(type_id)) else {
        return Ok(None);
    };
    let Some(table) = entry.as_table() else {
        bail!("PE resource type {type_id} is not a table");
    };
    let Some(entry) = table.get(name) else {
        return Ok(None);
    };
    let Some(table) = entry.as_table() else {
        bail!("PE resource name is not a language table");
    };
    let Some(entry) = table.get(ResourceEntryName::ID(u32::from(lang_id))) else {
        return Ok(None);
    };
    let Some(data) = entry.as_data() else {
        bail!("PE resource language entry is not raw data");
    };
    Ok(Some(data.data()))
}

fn verify_required_resources(
    resources: Option<&ResourceDirectory>,
    indexes: &[u8],
    data: &[u8],
    version_info: &[u8],
) -> Result<()> {
    let resources = resources.ok_or_else(|| anyhow!("assembled PE has no resource directory"))?;
    for (name, expected) in [("RESOURCE_INDEXES", indexes), ("RESOURCE_DATA", data)] {
        let actual = get_resource(
            resources,
            RT_RCDATA,
            &ResourceEntryName::from_string(name),
            LANG_EN_US,
        )?
        .ok_or_else(|| anyhow!("assembled PE is missing RCDATA {name}"))?;
        if actual != expected {
            bail!("assembled PE RCDATA {name} failed read-back verification");
        }
    }
    let actual = get_resource(resources, RT_VERSION, &ResourceEntryName::ID(1), LANG_EN_US)?
        .ok_or_else(|| anyhow!("assembled PE is missing RT_VERSION 1"))?;
    if actual != version_info {
        bail!("assembled PE RT_VERSION 1 failed read-back verification");
    }
    Ok(())
}

fn clear_certificate_directory(bytes: &mut [u8]) -> Result<()> {
    let pe_offset = usize::try_from(read_u32(bytes, 0x3c)?)?;
    let optional_header = pe_offset
        .checked_add(4 + 20)
        .ok_or_else(|| anyhow!("PE optional-header offset overflow"))?;
    let magic = read_u16(bytes, optional_header)?;
    let data_directories = match magic {
        0x10b => optional_header + 96,
        0x20b => optional_header + 112,
        other => bail!("unsupported PE optional-header magic {other:#x}"),
    };
    let security_entry = data_directories + 4 * 8;
    let entry = bytes
        .get_mut(security_entry..security_entry + 8)
        .ok_or_else(|| anyhow!("PE security data-directory entry is truncated"))?;
    entry.fill(0);
    Ok(())
}

fn write_output(output: &Path, bytes: &[u8]) -> Result<()> {
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("create output directory {}", parent.display()))?;

    let (temporary_path, mut file) = create_sibling_file(output, "tmp")?;
    let write_result = (|| -> Result<()> {
        file.write_all(bytes)
            .with_context(|| format!("write temporary PE {}", temporary_path.display()))?;
        file.sync_all()
            .with_context(|| format!("flush temporary PE {}", temporary_path.display()))?;
        Ok(())
    })();
    drop(file);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }

    if output.exists() {
        let backup = unused_sibling_path(output, "backup")?;
        fs::rename(output, &backup)
            .with_context(|| format!("stage existing output {}", output.display()))?;
        if let Err(error) = fs::rename(&temporary_path, output) {
            return match fs::rename(&backup, output) {
                Ok(()) => {
                    let _ = fs::remove_file(&temporary_path);
                    Err(error).with_context(|| format!("replace output {}", output.display()))
                }
                Err(restore_error) => Err(anyhow!(
                    "replace output {} failed: {error}; restoring previous output also failed: {restore_error}",
                    output.display()
                )),
            };
        }
        fs::remove_file(&backup)
            .with_context(|| format!("remove previous output backup {}", backup.display()))?;
    } else {
        fs::rename(&temporary_path, output)
            .with_context(|| format!("write output PE {}", output.display()))?;
    }
    Ok(())
}

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn sibling_name(output: &Path, suffix: &str, sequence: u64) -> PathBuf {
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut name = OsString::from(".");
    name.push(output.file_name().unwrap_or(OsStr::new("niva.exe")));
    name.push(format!(
        ".niva-packager-{}-{sequence}.{suffix}",
        std::process::id()
    ));
    parent.join(name)
}

fn unused_sibling_path(output: &Path, suffix: &str) -> Result<PathBuf> {
    for _ in 0..64 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = sibling_name(output, suffix, sequence);
        if !path.exists() {
            return Ok(path);
        }
    }
    bail!(
        "could not allocate a temporary sibling path for {}",
        output.display()
    )
}

fn create_sibling_file(output: &Path, suffix: &str) -> Result<(PathBuf, File)> {
    for _ in 0..64 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = sibling_name(output, suffix, sequence);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("create temporary PE {}", path.display()));
            }
        }
    }
    bail!(
        "could not create a temporary PE beside {}",
        output.display()
    )
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let end = offset
        .checked_add(2)
        .ok_or_else(|| anyhow!("PE/ICO offset overflow"))?;
    let field = bytes
        .get(offset..end)
        .ok_or_else(|| anyhow!("PE/ICO field is truncated"))?;
    Ok(u16::from_le_bytes(field.try_into()?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| anyhow!("PE/ICO offset overflow"))?;
    let field = bytes
        .get(offset..end)
        .ok_or_else(|| anyhow!("PE/ICO field is truncated"))?;
    Ok(u32::from_le_bytes(field.try_into()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use editpe::VersionInfo;
    use serde_json::json;

    const TINY_PNG: &[u8] =
        include_bytes!("../../../../niva/packages/examples/simple-project/icon.png");

    #[test]
    fn writes_named_rcdata_icon_and_version_and_can_be_written_again() {
        let base = test_dir("assemble-roundtrip");
        fs::create_dir_all(&base).unwrap();
        let runtime = base.join("runtime.exe");
        let first = base.join("first.exe");
        let second = base.join("second.exe");
        let icon = base.join("icon.png");
        fs::write(&runtime, pe_with_existing_manifest()).unwrap();
        fs::write(&icon, TINY_PNG).unwrap();
        let config = json!({
            "name": "Niva Unit Test",
            "version": "1.2.3-beta",
            "meta": {
                "companyName": "Test Company",
                "description": "Sample application",
                "copyright": "Copyright Test"
            }
        });

        assemble(
            &runtime,
            &first,
            &config,
            b"indexes bytes",
            b"compressed data",
            Some(&icon),
        )
        .unwrap();
        assert_resources(&first, b"indexes bytes", b"compressed data");
        assert_existing_manifest(&first);
        assert_icon_resources(&first);
        assert_version_resources(&first);

        assemble(
            &first,
            &second,
            &config,
            b"updated indexes",
            b"updated data",
            Some(&icon),
        )
        .unwrap();
        assert_resources(&second, b"updated indexes", b"updated data");
        assert_existing_manifest(&second);
        assert_icon_resources(&second);
        assert_version_resources(&second);

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn rejects_runtime_as_output_path() {
        let base = test_dir("same-path");
        fs::create_dir_all(&base).unwrap();
        let runtime = base.join("runtime.exe");
        fs::write(&runtime, minimal_pe64()).unwrap();
        let error = assemble(&runtime, &runtime, &json!({}), b"i", b"d", None).unwrap_err();
        assert!(error.to_string().contains("paths must differ"));
        let _ = fs::remove_dir_all(base);
    }

    fn assert_resources(exe: &Path, indexes: &[u8], data: &[u8]) {
        let image = Image::parse_file(exe).unwrap();
        let resources = image.resource_directory().unwrap();
        assert_eq!(
            get_resource(
                resources,
                RT_RCDATA,
                &ResourceEntryName::from_string("RESOURCE_INDEXES"),
                LANG_EN_US,
            )
            .unwrap(),
            Some(indexes)
        );
        assert_eq!(
            get_resource(
                resources,
                RT_RCDATA,
                &ResourceEntryName::from_string("RESOURCE_DATA"),
                LANG_EN_US,
            )
            .unwrap(),
            Some(data)
        );
    }

    fn assert_existing_manifest(exe: &Path) {
        let image = Image::parse_file(exe).unwrap();
        assert_eq!(
            get_resource(
                image.resource_directory().unwrap(),
                24,
                &ResourceEntryName::ID(1),
                LANG_EN_US,
            )
            .unwrap(),
            Some(b"existing runtime manifest".as_slice())
        );
    }

    fn assert_icon_resources(exe: &Path) {
        let image = Image::parse_file(exe).unwrap();
        let resources = image.resource_directory().unwrap();
        let group = get_resource(
            resources,
            RT_GROUP_ICON,
            &ResourceEntryName::ID(GROUP_ICON_ID),
            LANG_EN_US,
        )
        .unwrap()
        .unwrap();
        assert_eq!(read_u16(group, 0).unwrap(), 0);
        assert_eq!(read_u16(group, 2).unwrap(), 1);
        let count = usize::from(read_u16(group, 4).unwrap());
        assert_eq!(count, win_packager::icon::ICON_SIZES.len());
        for index in 0..count {
            let id = read_u16(group, 6 + index * 14 + 12).unwrap();
            assert_eq!(id, FIRST_ICON_ID + index as u16);
            let bytes = get_resource(
                resources,
                RT_ICON,
                &ResourceEntryName::ID(u32::from(id)),
                LANG_EN_US,
            )
            .unwrap()
            .unwrap();
            assert!(bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]));
        }
    }

    fn assert_version_resources(exe: &Path) {
        let image = Image::parse_file(exe).unwrap();
        let resources = image.resource_directory().unwrap();
        let bytes = get_resource(resources, RT_VERSION, &ResourceEntryName::ID(1), LANG_EN_US)
            .unwrap()
            .unwrap();
        let version = VersionInfo::parse(bytes).unwrap();
        assert_eq!(
            version.strings[0].strings.get("ProductName").unwrap(),
            "Niva Unit Test"
        );
        assert_eq!(
            version.strings[0].strings.get("CompanyName").unwrap(),
            "Test Company"
        );
        assert_eq!(version.info.file_version.major, (1 << 16) | 2);
        assert_eq!(version.info.file_version.minor, 3 << 16);
    }

    fn test_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("niva-packager-{name}-{}", std::process::id()))
    }

    fn minimal_pe64() -> Vec<u8> {
        // A small well-formed PE32+ image: DOS header at 0, PE header at 0x80,
        // one .text section at raw offset 0x200, and room for a new section
        // header. This provides a platform-independent writer/reader fixture.
        const PE_OFFSET: usize = 0x80;
        const OPTIONAL_OFFSET: usize = PE_OFFSET + 4 + 20;
        const SECTION_OFFSET: usize = OPTIONAL_OFFSET + 0xf0;
        const RAW_OFFSET: usize = 0x200;
        let mut bytes = vec![0u8; RAW_OFFSET + 0x200];
        bytes[0..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&(PE_OFFSET as u32).to_le_bytes());
        bytes[PE_OFFSET..PE_OFFSET + 4].copy_from_slice(b"PE\0\0");

        let coff = PE_OFFSET + 4;
        put_u16(&mut bytes, coff, 0x8664); // IMAGE_FILE_MACHINE_AMD64
        put_u16(&mut bytes, coff + 2, 1);
        put_u16(&mut bytes, coff + 16, 0xf0);
        put_u16(&mut bytes, coff + 18, 0x0022);

        put_u16(&mut bytes, OPTIONAL_OFFSET, 0x20b);
        put_u32(&mut bytes, OPTIONAL_OFFSET + 16, 0x1000); // entry point
        put_u64(&mut bytes, OPTIONAL_OFFSET + 24, 0x140000000);
        put_u32(&mut bytes, OPTIONAL_OFFSET + 32, 0x1000); // section alignment
        put_u32(&mut bytes, OPTIONAL_OFFSET + 36, 0x200); // file alignment
        put_u16(&mut bytes, OPTIONAL_OFFSET + 40, 6); // OS version
        put_u16(&mut bytes, OPTIONAL_OFFSET + 48, 6); // subsystem version
        put_u32(&mut bytes, OPTIONAL_OFFSET + 56, 0x2000); // SizeOfImage
        put_u32(&mut bytes, OPTIONAL_OFFSET + 60, 0x200); // SizeOfHeaders
        put_u16(&mut bytes, OPTIONAL_OFFSET + 68, 3); // console subsystem
        put_u64(&mut bytes, OPTIONAL_OFFSET + 72, 0x100000);
        put_u64(&mut bytes, OPTIONAL_OFFSET + 80, 0x1000);
        put_u64(&mut bytes, OPTIONAL_OFFSET + 88, 0x100000);
        put_u64(&mut bytes, OPTIONAL_OFFSET + 96, 0x1000);
        put_u32(&mut bytes, OPTIONAL_OFFSET + 108, 16);

        bytes[SECTION_OFFSET..SECTION_OFFSET + 8].copy_from_slice(b".text\0\0\0");
        put_u32(&mut bytes, SECTION_OFFSET + 8, 0x100); // virtual size
        put_u32(&mut bytes, SECTION_OFFSET + 12, 0x1000); // virtual address
        put_u32(&mut bytes, SECTION_OFFSET + 16, 0x200); // raw size
        put_u32(&mut bytes, SECTION_OFFSET + 20, RAW_OFFSET as u32);
        put_u32(&mut bytes, SECTION_OFFSET + 36, 0x60000020); // code, execute, read
        bytes[RAW_OFFSET] = 0xc3; // ret
        bytes
    }

    fn pe_with_existing_manifest() -> Vec<u8> {
        let mut image = Image::parse(minimal_pe64()).unwrap();
        let mut resources = ResourceDirectory::default();
        set_resource(
            &mut resources,
            24,
            ResourceEntryName::ID(1),
            LANG_EN_US,
            b"existing runtime manifest",
        )
        .unwrap();
        image.set_resource_directory(resources).unwrap();
        image.data().to_vec()
    }

    fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
}
