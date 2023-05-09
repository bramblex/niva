import { parseVersion } from "../common/utils";

export function plistTemplate(config: any) {
  const version = parseVersion(config.meta?.version || "").join(".");
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
    <string>${version}</string>
    <key>CFBundleVersion</key>
    <string>${version}</string>
    <key>CSResourcesFileMapped</key>
    <true />
    <key>LSRequiresCarbon</key>
    <true />
    <key>NSHighResolutionCapable</key>
    <true />
    <key>NSHumanReadableCopyright</key>
    <string>${config.meta?.copyright || ""}</string>
  </dict>
</plist>
`;
}
