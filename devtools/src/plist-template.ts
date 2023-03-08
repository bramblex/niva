export function generatePlist(config: any) {
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
    <string>${config.icon}</string>
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
  </dict>
</plist>
`;
}
