# win_packager

`win_packager` contains Windows PE resource helpers. The GUI and CLI application
build path is [`niva-packager`](../../docs/packager-usage.md); it uses this crate's
PNG-to-ICO and `VERSIONINFO` helpers while assembling Windows resources in its
own cross-platform backend. The `win_packager` executable remains useful for
low-level Windows resource smoke fixtures, but Devtools and the release scripts
do not stage or invoke it.

## Library and CLI

The library provides:

- `bundle`: the low-level `RESOURCE_INDEXES` / `RESOURCE_DATA` format builder;
- `icon`: PNG-to-multisize-ICO conversion and ICO parsing;
- `version_info`: conversion of the supported `.rc` `VERSIONINFO` subset;
- `pack`: a template-executable resource writer using Win32 resource APIs.

The CLI accepts a template executable and an output path, then optional bundle,
icon, RCDATA, and version resources:

```powershell
cargo run --release -p win_packager -- `
  --exe target\release\niva.exe `
  --save-as dist\smoke.exe `
  --resource-dir packages\devtools\build `
  --config packages\devtools\niva.json `
  --icon-png packages\devtools\build\icon.png
```

`pack()` stages its output beside the destination and publishes it only after a
successful resource update. The template and destination must resolve to
different files, including through symlinks, hard links, and missing parent
components. Windows file identity checks prefer the full 128-bit ID and fail
closed when no valid identity is available.

Resource preparation and the encoding helpers can be tested on macOS. Writing
PE resources through `pack()` requires Windows. The production `niva-packager`
uses its separate PE editor, so a Windows target check for this crate does not
verify the production packager or Windows runtime behavior.

## Verification

```bash
cargo check -p win_packager
cargo test -p win_packager
cargo run -p win_packager -- --help
cargo check -p win_packager --target x86_64-pc-windows-msvc
```

These checks do not replace Windows device validation of generated PE resource
contents or application startup.
