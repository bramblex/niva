use std::{fs, path::Path};

use anyhow::{Context, Result, bail, ensure};
use apple_codesign::{
    BundleSigner, CodeSignatureFlags, MachFile, SigningSettings, VerificationProblemType,
    verify_macho_data,
};
use icns::{IconFamily, IconType, Image as IcnsImage, PixelFormat};
use plist::{Dictionary, Value as PlistValue};

/// Assemble and ad-hoc sign a macOS app bundle on any supported host.
///
/// The final app is written to `output`; signing runs into a separate sibling
/// directory so an error cannot leave a partly signed bundle at that path.
pub fn assemble(
    runtime: &Path,
    output: &Path,
    config: &serde_json::Value,
    indexes: &[u8],
    data: &[u8],
    icon: Option<&Path>,
) -> Result<()> {
    let runtime_metadata = fs::metadata(runtime)
        .with_context(|| format!("read macOS runtime {}", runtime.display()))?;
    ensure!(
        runtime_metadata.is_file(),
        "macOS runtime is not a file: {}",
        runtime.display()
    );

    let name = config
        .get("name")
        .and_then(serde_json::Value::as_str)
        .context("niva.json requires a string name")?;
    validate_executable_name(name)?;

    let uuid = config
        .get("uuid")
        .and_then(serde_json::Value::as_str)
        .context("niva.json requires a string uuid")?;
    ensure!(!uuid.trim().is_empty(), "niva.json uuid must not be empty");

    ensure!(
        output
            .extension()
            .is_some_and(|extension| extension == "app"),
        "macOS output path must end in .app: {}",
        output.display()
    );
    ensure_path_does_not_exist(output)?;

    let output_parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(output_parent)
        .with_context(|| format!("create output parent {}", output_parent.display()))?;

    // Build the unsigned source bundle at the requested staging path. The
    // caller owns a larger staging directory and only publishes it after this
    // function and the archive step both succeed.
    fs::create_dir(output).with_context(|| format!("create app bundle {}", output.display()))?;
    let contents = output.join("Contents");
    let macos_dir = contents.join("MacOS");
    let resources_dir = contents.join("Resources");
    fs::create_dir_all(&macos_dir)?;
    fs::create_dir_all(&resources_dir)?;

    let executable = macos_dir.join(name);
    fs::copy(runtime, &executable).with_context(|| {
        format!(
            "copy runtime {} to {}",
            runtime.display(),
            executable.display()
        )
    })?;
    set_executable_permissions(&executable)?;

    fs::write(resources_dir.join("RESOURCE_INDEXES"), indexes).context("write RESOURCE_INDEXES")?;
    fs::write(resources_dir.join("RESOURCE_DATA"), data).context("write RESOURCE_DATA")?;

    if let Some(icon_path) = icon {
        write_icns(icon_path, &resources_dir.join("icon.icns"))?;
    }

    write_info_plist(
        &contents.join("Info.plist"),
        config,
        name,
        uuid,
        icon.is_some(),
    )?;

    let mut signer = BundleSigner::new_from_path(output)
        .with_context(|| format!("open app bundle for signing: {}", output.display()))?;
    signer
        .collect_nested_bundles()
        .context("collect nested code bundles")?;

    let signing_stage = tempfile::Builder::new()
        .prefix(".niva-macos-sign-")
        .tempdir_in(output_parent)
        .context("create signing staging directory")?;
    let signed_bundle = signing_stage
        .path()
        .join(output.file_name().context("app output has no file name")?);
    signer
        .write_signed_bundle(&signed_bundle, &SigningSettings::default())
        .context("write ad-hoc signed app bundle")?;

    ensure!(
        signed_bundle.join("Contents/Info.plist").is_file(),
        "signer did not write an app bundle"
    );
    ensure!(
        signed_bundle
            .join("Contents/_CodeSignature/CodeResources")
            .is_file(),
        "signer did not write bundle resources seal"
    );

    let signed_indexes = fs::read(signed_bundle.join("Contents/Resources/RESOURCE_INDEXES"))
        .context("read signed RESOURCE_INDEXES")?;
    ensure!(
        signed_indexes.as_slice() == indexes,
        "signing changed Contents/Resources/RESOURCE_INDEXES"
    );
    let signed_data = fs::read(signed_bundle.join("Contents/Resources/RESOURCE_DATA"))
        .context("read signed RESOURCE_DATA")?;
    ensure!(
        signed_data.as_slice() == data,
        "signing changed Contents/Resources/RESOURCE_DATA"
    );

    let signed_executable = signed_bundle.join("Contents/MacOS").join(name);
    verify_signed_executable(&signed_executable)?;

    fs::remove_dir_all(output)
        .with_context(|| format!("remove unsigned staging bundle {}", output.display()))?;
    fs::rename(&signed_bundle, output)
        .with_context(|| format!("install signed app bundle at {}", output.display()))?;

    Ok(())
}

fn verify_signed_executable(path: &Path) -> Result<()> {
    let data = fs::read(path).with_context(|| format!("read signed Mach-O {}", path.display()))?;
    let problems = verify_macho_data(&data);

    // apple-codesign 0.29.0 represents an ad-hoc signature with an empty CMS
    // slot. Its verifier reports one CmsError while parsing that empty slot,
    // even when the CodeDirectory and all code digests are valid. Ignore only
    // that exact case: every architecture must have an empty CMS slot and the
    // ADHOC flag, and the verifier must report no other problems.
    let expected_adhoc_cms_problem = is_adhoc_with_empty_cms(&data)
        && problems.len() == 1
        && matches!(&problems[0].problem, VerificationProblemType::CmsError(_));

    if !problems.is_empty() && !expected_adhoc_cms_problem {
        let details = problems
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n  ");
        bail!(
            "apple-codesign Mach-O verification failed for {}:\n  {details}",
            path.display()
        );
    }

    Ok(())
}

fn is_adhoc_with_empty_cms(data: &[u8]) -> bool {
    let Ok(file) = MachFile::parse(data) else {
        return false;
    };
    let binaries = file.iter_macho().collect::<Vec<_>>();
    !binaries.is_empty()
        && binaries.into_iter().all(|binary| {
            let Ok(Some(signature)) = binary.code_signature() else {
                return false;
            };
            let Ok(Some(cms)) = signature.signature_data() else {
                return false;
            };
            if !cms.is_empty() {
                return false;
            }
            matches!(signature.code_directory(), Ok(Some(directory)) if directory.flags.contains(CodeSignatureFlags::ADHOC))
        })
}

fn validate_executable_name(name: &str) -> Result<()> {
    ensure!(!name.trim().is_empty(), "niva.json name must not be empty");
    ensure!(
        name != "."
            && name != ".."
            && !name
                .chars()
                .any(|character| character.is_control() || "/\\:".contains(character)),
        "niva.json name cannot be used as an executable file name"
    );
    Ok(())
}

fn ensure_path_does_not_exist(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => bail!("output already exists: {}", path.display()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("inspect output {}", path.display())),
    }
}

fn set_executable_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .with_context(|| format!("set executable mode on {}", path.display()))?;
    }

    // Windows filesystems do not expose Unix mode bits. The archive stage
    // writes the macOS executable entry with mode 0755 explicitly.
    #[cfg(not(unix))]
    let _ = path;

    Ok(())
}

fn write_info_plist(
    path: &Path,
    config: &serde_json::Value,
    name: &str,
    uuid: &str,
    has_icon: bool,
) -> Result<()> {
    let version = app_version(config)?;
    let identifier = format!(
        "com.niva.{}.{}",
        identifier_segment(name),
        identifier_segment(uuid)
    );

    let mut dictionary = Dictionary::new();
    insert_string(&mut dictionary, "CFBundleDevelopmentRegion", "en");
    insert_string(&mut dictionary, "CFBundleDisplayName", name);
    insert_string(&mut dictionary, "CFBundleExecutable", name);
    if has_icon {
        insert_string(&mut dictionary, "CFBundleIconFile", "icon.icns");
    }
    insert_string(&mut dictionary, "CFBundleIdentifier", identifier);
    insert_string(&mut dictionary, "CFBundleInfoDictionaryVersion", "6.0");
    insert_string(&mut dictionary, "CFBundleName", name);
    insert_string(&mut dictionary, "CFBundlePackageType", "APPL");
    insert_string(&mut dictionary, "CFBundleShortVersionString", &version);
    insert_string(&mut dictionary, "CFBundleVersion", &version);
    dictionary.insert(
        "NSHighResolutionCapable".to_string(),
        PlistValue::Boolean(true),
    );
    insert_string(&mut dictionary, "NSPrincipalClass", "NSApplication");

    if let Some(copyright) = config
        .get("meta")
        .and_then(|meta| meta.get("copyright"))
        .and_then(serde_json::Value::as_str)
    {
        insert_string(&mut dictionary, "NSHumanReadableCopyright", copyright);
    }

    PlistValue::Dictionary(dictionary)
        .to_file_xml(path)
        .with_context(|| format!("write Info.plist {}", path.display()))
}

fn insert_string(dictionary: &mut Dictionary, key: &str, value: impl Into<String>) {
    dictionary.insert(key.to_string(), PlistValue::String(value.into()));
}

fn app_version(config: &serde_json::Value) -> Result<String> {
    let raw = match config.get("version") {
        None | Some(serde_json::Value::Null) => "",
        Some(serde_json::Value::String(version)) => version,
        Some(_) => bail!("niva.json version must be a string"),
    };

    let raw = raw
        .trim()
        .strip_prefix('v')
        .or_else(|| raw.trim().strip_prefix('V'))
        .unwrap_or(raw.trim());
    let mut components = Vec::with_capacity(3);
    for component in raw.split('.') {
        if components.len() == 3 {
            break;
        }
        let digits: String = component.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            break;
        }
        let number = digits
            .parse::<u32>()
            .with_context(|| format!("invalid version component {digits:?}"))?;
        let max_digits = if components.is_empty() { 4 } else { 2 };
        ensure!(
            number.to_string().len() <= max_digits,
            "version component {digits:?} exceeds Apple's bundle version limit"
        );
        components.push(number);
    }
    while components.len() < 3 {
        components.push(0);
    }

    Ok(components
        .into_iter()
        .map(|component| component.to_string())
        .collect::<Vec<_>>()
        .join("."))
}

fn identifier_segment(value: &str) -> String {
    let mut result = String::new();
    let mut pending_separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if pending_separator && !result.is_empty() {
                result.push('-');
            }
            result.push(character.to_ascii_lowercase());
            pending_separator = false;
        } else {
            pending_separator = true;
        }
    }

    if result.is_empty() {
        "app".to_string()
    } else {
        result
    }
}

fn write_icns(source: &Path, output: &Path) -> Result<()> {
    let source_image =
        image::open(source).with_context(|| format!("decode PNG app icon {}", source.display()))?;
    let mut family = IconFamily::new();
    let icon_types = [
        IconType::RGBA32_16x16,
        IconType::RGBA32_16x16_2x,
        IconType::RGBA32_32x32,
        IconType::RGBA32_32x32_2x,
        IconType::RGBA32_64x64,
        IconType::RGBA32_128x128,
        IconType::RGBA32_128x128_2x,
        IconType::RGBA32_256x256,
        IconType::RGBA32_256x256_2x,
        IconType::RGBA32_512x512,
        IconType::RGBA32_512x512_2x,
    ];

    for icon_type in icon_types {
        let width = icon_type.pixel_width();
        let height = icon_type.pixel_height();
        let rgba = source_image
            .resize_exact(width, height, image::imageops::FilterType::Lanczos3)
            .to_rgba8();
        let icon_image = IcnsImage::from_data(PixelFormat::RGBA, width, height, rgba.into_raw())
            .with_context(|| format!("prepare {width}x{height} ICNS image"))?;
        family
            .add_icon_with_type(&icon_image, icon_type)
            .with_context(|| format!("encode {width}x{height} ICNS image"))?;
    }

    let file = fs::File::create(output).with_context(|| format!("create {}", output.display()))?;
    family
        .write(file)
        .with_context(|| format!("write {}", output.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_values_are_valid_and_bundle_identifiers_are_safe() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Info.plist");
        let config = serde_json::json!({
            "version": "v1.2.3-beta.4",
            "meta": {"copyright": "© Example & Friends"}
        });

        write_info_plist(
            &path,
            &config,
            "My & App",
            "38b646a2-209e-45ed-a6c9-1817bd15aa4f",
            false,
        )
        .unwrap();

        let plist = PlistValue::from_file(&path).unwrap();
        let dictionary = plist.as_dictionary().unwrap();
        assert_eq!(
            dictionary.get("CFBundleExecutable").unwrap().as_string(),
            Some("My & App")
        );
        assert_eq!(
            dictionary
                .get("CFBundleShortVersionString")
                .unwrap()
                .as_string(),
            Some("1.2.3")
        );
        assert_eq!(
            dictionary.get("CFBundleVersion").unwrap().as_string(),
            Some("1.2.3")
        );
        assert_eq!(
            dictionary.get("CFBundleIdentifier").unwrap().as_string(),
            Some("com.niva.my-app.38b646a2-209e-45ed-a6c9-1817bd15aa4f")
        );
        assert!(dictionary.get("CFBundleIconFile").is_none());
    }

    #[test]
    fn icon_bundle_contains_native_retina_sizes() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.png");
        let output = directory.path().join("icon.icns");
        let image = image::RgbaImage::from_pixel(32, 32, image::Rgba([10, 20, 30, 128]));
        image.save(&source).unwrap();

        write_icns(&source, &output).unwrap();
        let family = IconFamily::read(fs::File::open(output).unwrap()).unwrap();
        assert!(family.has_icon_with_type(IconType::RGBA32_16x16));
        assert!(family.has_icon_with_type(IconType::RGBA32_512x512_2x));
    }
}
