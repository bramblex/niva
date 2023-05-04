import { parseVersion } from "../common/utils";

export function versionInfoTemplate(config: any) {
  const numberVersion = parseVersion(config.version || "").join(",");

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
    VALUE "CompanyName", ${JSON.stringify(config.companyName || "")}
    VALUE "FileDescription", ${JSON.stringify(config.description || "")}
    VALUE "FileVersion", ${JSON.stringify(config.version)}
    VALUE "InternalName", "niva.exe"
    VALUE "LegalCopyright", ${JSON.stringify(config.copyright || "")}
    VALUE "OriginalFilename", "niva.exe"
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
