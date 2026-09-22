//! CLI：给 devtools `build-windows.ts` 调用的 ResourceHacker 平替。
//!
//! 高层模式（推荐，一次调用代替备料 + `icon_creator.exe` + ResourceHacker）：
//! ```text
//! win_packager --exe <template> --save-as <target>
//!   --resource-dir <build输出目录> --config <niva.json>
//!   [--icon-png <icon.png>] [--delete-icon-ids 1,2,3,4,5,6,7]
//!   [--version-info VERSION_INFO] [--lang 1033]
//! ```
//!
//! 低层模式（调试/兼容旧备料）：
//! ```text
//! win_packager --exe <template> --save-as <target>
//!   [--rcdata NAME=FILE]... [--icon icon.ico] ...
//! ```
//!
//! 与旧脚本的对应关系见 `crates/win_packager/README.md`。

use std::path::PathBuf;

use win_packager::{DEFAULT_ICON_GROUP_ID, DEFAULT_LANG, IconEntry, PackRequest, RcDataEntry};

fn usage() -> &'static str {
    "usage: win_packager --exe <template> --save-as <target> [--resource-dir DIR --config niva.json] [--icon-png icon.png] [--rcdata NAME=FILE]... [--icon icon.ico] [--group-id 1] [--delete-icon-ids 1,2,...] [--version-info VERSION_INFO] [--lang 1033]"
}

fn parse_args(args: &[String]) -> anyhow::Result<PackRequest> {
    let mut template_exe: Option<PathBuf> = None;
    let mut output_exe: Option<PathBuf> = None;
    let mut resource_dir: Option<PathBuf> = None;
    let mut config_file: Option<PathBuf> = None;
    let mut icon_png: Option<PathBuf> = None;
    let mut rcdata: Vec<RcDataEntry> = Vec::new();
    let mut icon_file: Option<PathBuf> = None;
    let mut group_id = DEFAULT_ICON_GROUP_ID;
    let mut delete_icon_ids: Vec<u16> = Vec::new();
    let mut version_info_rc: Option<PathBuf> = None;
    let mut lang = DEFAULT_LANG;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--exe" => {
                i += 1;
                template_exe = Some(PathBuf::from(arg_value(&args, i, "--exe")?));
            }
            "--save-as" => {
                i += 1;
                output_exe = Some(PathBuf::from(arg_value(&args, i, "--save-as")?));
            }
            "--resource-dir" => {
                i += 1;
                resource_dir = Some(PathBuf::from(arg_value(&args, i, "--resource-dir")?));
            }
            "--config" => {
                i += 1;
                config_file = Some(PathBuf::from(arg_value(&args, i, "--config")?));
            }
            "--icon-png" => {
                i += 1;
                icon_png = Some(PathBuf::from(arg_value(&args, i, "--icon-png")?));
            }
            "--rcdata" => {
                i += 1;
                let v = arg_value(&args, i, "--rcdata")?;
                let (name, file) = v
                    .split_once('=')
                    .ok_or_else(|| anyhow::anyhow!("--rcdata must be NAME=FILE, got {v:?}"))?;
                rcdata.push(RcDataEntry {
                    name: name.to_string(),
                    file: PathBuf::from(file),
                });
            }
            "--icon" => {
                i += 1;
                icon_file = Some(PathBuf::from(arg_value(&args, i, "--icon")?));
            }
            "--group-id" => {
                i += 1;
                group_id = arg_value(&args, i, "--group-id")?.parse()?;
            }
            "--delete-icon-ids" => {
                i += 1;
                let v = arg_value(&args, i, "--delete-icon-ids")?;
                for s in v.split(',') {
                    let s = s.trim();
                    if !s.is_empty() {
                        delete_icon_ids.push(s.parse()?);
                    }
                }
            }
            "--version-info" => {
                i += 1;
                version_info_rc = Some(PathBuf::from(arg_value(&args, i, "--version-info")?));
            }
            "--lang" => {
                i += 1;
                lang = arg_value(&args, i, "--lang")?.parse()?;
            }
            "--help" | "-h" => {
                println!("{}", usage());
                std::process::exit(0);
            }
            other => anyhow::bail!("unknown arg {other:?}\n{}", usage()),
        }
        i += 1;
    }

    Ok(PackRequest {
        template_exe: template_exe.ok_or_else(|| anyhow::anyhow!("missing --exe\n{}", usage()))?,
        output_exe: output_exe.ok_or_else(|| anyhow::anyhow!("missing --save-as\n{}", usage()))?,
        resource_dir,
        config_file,
        icon_png,
        rcdata,
        icon: icon_file.map(|ico_file| IconEntry { ico_file, group_id }),
        delete_icon_ids,
        version_info_rc,
        lang,
    })
}

fn arg_value(args: &[String], i: usize, flag: &str) -> anyhow::Result<String> {
    args.get(i)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("{flag} needs a value\n{}", usage()))
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let req = parse_args(&args)?;
    win_packager::pack(&req)?;
    eprintln!("[win_packager] packed {}", req.output_exe.display());
    Ok(())
}
