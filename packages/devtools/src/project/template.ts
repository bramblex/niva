export function plistTemplate(config: any) {
  return `
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleDisplayName</key>
    <string>${config.name}</string>
    <key>CFBundleExecutable</key>
    <string>${config.name}</string>
    <key>CFBundleIconFile</key>
    <string>icon.icns</string>
    <key>CFBundleIdentifier</key>
    <string>${config.name}.${config.uuid}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>${config.name}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${config.version || "0.0.0"}</string>
    <key>CFBundleVersion</key>
    <string>${config.version || "0.0.0"}</string>
    <key>CSResourcesFileMapped</key>
    <true />
    <key>LSRequiresCarbon</key>
    <true />
    <key>NSHighResolutionCapable</key>
    <true />
    <key>NSHumanReadableCopyright</key>
    <string>${config.copyright}</string>
  </dict>
</plist>
`;
}

function parseVersion(versionString: string): number[] {
  const versionDigits = versionString.replace(/[^0-9.]/g, '').split('.').map(Number);
  while (versionDigits.length < 4) {
    versionDigits.push(0);
  }
  return versionDigits.slice(0, 4);
}

export function versionInfoTemplate(config: any) {
  const numberVersion = parseVersion(config.version).join(',');

  return `
1 VERSIONINFO
FILEVERSION ${numberVersion}
PRODUCTVERSION ${numberVersion}
FILEOS 0x40004
FILETYPE 0x1
{
BLOCK "StringFileInfo"
{
  BLOCK "040904b0"
  {
    VALUE "CompanyName", ${JSON.stringify(config.companyName)}
    VALUE "FileDescription", ${JSON.stringify(config.description)}
    VALUE "FileVersion", ${JSON.stringify(config.version)}
    VALUE "InternalName", "tauri_lite.exe"
    VALUE "LegalCopyright", ${JSON.stringify(config.copyright)}
    VALUE "OriginalFilename", "tauri_lite.exe"
    VALUE "ProductName", ${JSON.stringify(config.name)}
    VALUE "ProductVersion", ${JSON.stringify(config.version)}
    VALUE "SquirrelAwareVersion", "1"
  }
}

BLOCK "VarFileInfo"
{
  VALUE "Translation", 0x0409 0x04B0  
}
}`;
}
