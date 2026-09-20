use std::env;
use std::fs;

pub const ZAPRET_REPO: &str = "bol-van/zapret";
pub const ZAPRET_REC_VER: &str = "v72.13";
pub const STRAT_REC_VER: &str = "9503dc045133000af8075e066f09bb469008e530";

const STRAT_REPO_ZIP: &str = "https://github.com/Flowseal/zapret-discord-youtube/archive/refs/heads/main.zip";

fn detect_platform_dir() -> Result<&'static str, String> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    match os {
        "linux" => match arch {
            "x86_64" => Ok("linux-x86_64"),
            "x86" => Ok("linux-x86"),
            "aarch64" => Ok("linux-arm64"),
            "arm" => Ok("linux-arm"),
            "mips64" => Ok("linux-mips64"),
            "mips" => Ok("linux-mips"),
            _ => Err(format!("Unsupported Linux architecture: {}", arch)),
        },
        "macos" => Ok("mac64"),
        "freebsd" => Ok("freebsd-x86_64"),
        "windows" => match arch {
            "x86_64" => Ok("windows-x86_64"),
            "x86" => Ok("windows-x86"),
            _ => Err(format!("Unsupported Windows architecture: {}", arch)),
        },
        _ => Err(format!("Unsupported OS: {}", os)),
    }
}

pub const ZAPRET2_STRAT_ARCHIVE: &str =
    "https://git.zapret.moe/zapretdiscordyoutube/zapret2-youtube-discord/archive/main.tar.gz";

const ZAPRET2_UNPACK_DIRS: &[&str] = &["bin", "lists", "lua", "presets", "scripts"];

#[cfg(unix)]
fn set_exec(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(mut perms) = fs::metadata(path).map(|m| m.permissions()) {
        perms.set_mode(0o755);
        let _ = fs::set_permissions(path, perms);
    }
}

#[cfg(not(unix))]
fn set_exec(_path: &std::path::Path) {}

fn is_unpack_dir(name: &str) -> bool {
    ZAPRET2_UNPACK_DIRS.contains(&name)
}

fn strip_archive_root(path: &std::path::Path) -> Option<std::path::PathBuf> {
    let names: Vec<String> = path
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();

    let idx = names
        .iter()
        .take(2)
        .position(|name| is_unpack_dir(name))
        .or_else(|| names.iter().position(|name| is_unpack_dir(name)))?;

    let mut out = std::path::PathBuf::new();
    for name in &names[idx..] {
        out.push(name);
    }
    Some(out)
}

fn resolve_link_source(
    relative: &std::path::Path,
    link: &std::path::Path,
    hard: bool,
) -> Option<std::path::PathBuf> {
    if hard {
        return strip_archive_root(link);
    }

    let mut out = std::path::PathBuf::new();
    if let Some(parent) = relative.parent() {
        out.push(parent);
    }
    for component in link.components() {
        match component {
            std::path::Component::Normal(name) => out.push(name),
            std::path::Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            _ => {}
        }
    }

    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

fn count_entry(per_dir: &mut std::collections::BTreeMap<String, usize>, relative: &std::path::Path) {
    if let Some(first) = relative.components().next() {
        let key = first.as_os_str().to_string_lossy().into_owned();
        *per_dir.entry(key).or_insert(0) += 1;
    }
}

fn report_missing_preset_lists(workspace: &std::path::Path) {
    let Ok(entries) = fs::read_dir(workspace.join("presets")) else {
        return;
    };

    let mut missing: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !crate::strategy::discovery::is_valid_preset_name(name) {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let parsed = crate::strategy::parse_preset_content(&content, None);
        for reference in crate::strategy::missing_list_refs(workspace, &parsed.args) {
            if !missing.contains(&reference.path) {
                missing.push(reference.path);
            }
        }
    }

    if missing.is_empty() {
        return;
    }
    missing.sort();
    println!("{}{}", rust_i18n::t!("msg_z2_missing_lists"), missing.join(", "));
}

pub fn download_zapret2_strategies() -> Result<(), String> {
    let target_dir = crate::config::ZapretEngine::Zapret2.workspace_dir();

    println!("{}", rust_i18n::t!("msg_dl_strat"));
    let req = ureq::get(ZAPRET2_STRAT_ARCHIVE)
        .set("User-Agent", "zapret-rust")
        .call()
        .map_err(|e| format!("{}{}", rust_i18n::t!("err_dl_strat_zip"), e))?;
    let mut body = req.into_reader();

    let tmp_archive = crate::config::get_cache_dir().join(".tmp_zapret2_strategies.tar.gz");
    let mut file =
        fs::File::create(&tmp_archive).map_err(|e| format!("{}{}", rust_i18n::t!("err_create_tmp_zip"), e))?;
    std::io::copy(&mut body, &mut file).map_err(|e| format!("{}{}", rust_i18n::t!("err_write_arc"), e))?;
    drop(file);

    println!("{}", rust_i18n::t!("msg_ext_strat"));
    let tar_gz = fs::File::open(&tmp_archive).map_err(|e| format!("{}{}", rust_i18n::t!("err_open_arc"), e))?;
    let decoder = flate2::read::GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(decoder);

    fs::create_dir_all(&target_dir).map_err(|e| format!("{}{}", rust_i18n::t!("err_mkdir"), e))?;

    let mut extracted = 0usize;
    let mut per_dir: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut deferred_links: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();

    for entry in archive
        .entries()
        .map_err(|e| format!("{}{}", rust_i18n::t!("err_unpack_tar"), e))?
    {
        let mut entry = entry.map_err(|e| format!("{}{}", rust_i18n::t!("err_unpack_tar"), e))?;
        let path = entry
            .path()
            .map_err(|e| format!("{}{}", rust_i18n::t!("err_unpack_tar"), e))?
            .into_owned();

        let Some(relative) = strip_archive_root(&path) else {
            continue;
        };
        let full_path = target_dir.join(&relative);

        let entry_type = entry.header().entry_type();
        if entry_type.is_dir() {
            fs::create_dir_all(&full_path).map_err(|e| format!("{}{}", rust_i18n::t!("err_mkdir"), e))?;
            continue;
        }

        if entry_type.is_hard_link() || entry_type.is_symlink() {
            let hard = entry_type.is_hard_link();
            let link = entry.link_name().ok().flatten().map(|l| l.into_owned());
            let source = link.and_then(|l| resolve_link_source(&relative, &l, hard));
            if let Some(source) = source {
                deferred_links.push((relative, source));
            }
            continue;
        }

        if !entry_type.is_file() {
            continue;
        }

        let mode_exec = entry.header().mode().map(|m| m & 0o111 != 0).unwrap_or(false);

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}{}", rust_i18n::t!("err_mkdir"), e))?;
        }

        let mut outfile =
            fs::File::create(&full_path).map_err(|e| format!("{}{}", rust_i18n::t!("err_extract"), e))?;
        std::io::copy(&mut entry, &mut outfile)
            .map_err(|e| format!("{}{}", rust_i18n::t!("err_copy_content"), e))?;
        drop(outfile);

        if relative.starts_with("bin")
            || relative.starts_with("lua")
            || relative.starts_with("scripts")
            || mode_exec
        {
            set_exec(&full_path);
        }
        count_entry(&mut per_dir, &relative);
        extracted += 1;
    }

    for (relative, source) in deferred_links {
        let from = target_dir.join(&source);
        let to = target_dir.join(&relative);
        if !from.is_file() {
            continue;
        }
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}{}", rust_i18n::t!("err_mkdir"), e))?;
        }
        fs::copy(&from, &to).map_err(|e| format!("{}{}", rust_i18n::t!("err_copy_content"), e))?;
        count_entry(&mut per_dir, &relative);
        extracted += 1;
    }

    let _ = fs::remove_file(&tmp_archive);

    if extracted == 0 {
        return Err(format!(
            "zapret2 archive contained no {} entries",
            ZAPRET2_UNPACK_DIRS.join("/")
        ));
    }

    for dir in ["bin", "lists", "lua", "presets"] {
        let count = per_dir.get(dir).copied().unwrap_or(0);
        println!("  {:<8} {}", dir, count);
        if count == 0 {
            println!("{}{}", rust_i18n::t!("msg_z2_dir_empty"), dir);
        }
    }
    report_missing_preset_lists(&target_dir);

    println!("{}", rust_i18n::t!("msg_strat_ok"));
    Ok(())
}

pub fn download_zapret2(version: &str) -> Result<(), String> {
    if version == "skip" {
        return Ok(());
    }

    let url = "https://api.github.com/repos/bol-van/zapret2/releases/latest";
    let req = ureq::get(url)
        .set("User-Agent", "zapret-rust")
        .call()
        .map_err(|e| e.to_string())?;

    let json_str = req.into_string().unwrap_or_else(|_| "[]".to_string());
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap_or_default();

    let mut download_url = String::new();
    if let Some(assets) = parsed.get("assets").and_then(|a| a.as_array()) {
        for asset in assets {
            if let Some(name) = asset.get("name").and_then(|n| n.as_str()) {
                if name.starts_with("zapret2-") && name.ends_with(".tar.gz") {
                    if let Some(d_url) = asset.get("browser_download_url").and_then(|u| u.as_str()) {
                        download_url = d_url.to_string();
                        break;
                    }
                }
            }
        }
    }

    if download_url.is_empty() {
        return Err("Zapret2 archive not found".to_string());
    }

    let platform = detect_platform_dir()?;
    let platform_marker = format!("binaries/{}/", platform);

    let tmp_archive = crate::config::get_cache_dir().join(".tmp_zapret2_download.tar.gz");
    let mut response = ureq::get(&download_url)
        .call()
        .map_err(|e| e.to_string())?
        .into_reader();
    let mut file = fs::File::create(&tmp_archive).map_err(|e| e.to_string())?;
    std::io::copy(&mut response, &mut file).map_err(|e| e.to_string())?;
    drop(file);

    let tar_gz = fs::File::open(&tmp_archive).map_err(|e| e.to_string())?;
    let tar = flate2::read::GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(tar);

    let cache_dir = crate::config::get_cache_dir();
    let bin_dir = cache_dir.join("bin");
    let engine = crate::config::ZapretEngine::Zapret2;
    let ws_dir = engine.workspace_dir();
    let ws_bin_dir = ws_dir.join("bin");
    fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&ws_bin_dir).map_err(|e| e.to_string())?;

    let target_bin = bin_dir.join(engine.binary_name());
    let mut found_daemon = false;

    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        let path_str = path.to_string_lossy().replace('\\', "/");

        if !entry.header().entry_type().is_file() {
            continue;
        }

        if path_str.contains(&platform_marker) {
            let Some(file_name) = path.file_name() else {
                continue;
            };
            let name = file_name.to_string_lossy().to_string();
            let dest = if name == "nfqws" || name == "nfqws2" || name == "winws.exe" || name == "winws2.exe" {
                found_daemon = true;
                target_bin.clone()
            } else {
                bin_dir.join(&name)
            };
            let _ = fs::remove_file(&dest);
            entry.unpack(&dest).map_err(|e| e.to_string())?;
            set_exec(&dest);
        } else if path_str.contains("files/fake/") {
            let Some(file_name) = path.file_name() else {
                continue;
            };
            let dest = ws_bin_dir.join(file_name);
            if !dest.exists() {
                entry.unpack(&dest).map_err(|e| e.to_string())?;
            }
        }
    }

    let _ = fs::remove_file(&tmp_archive);

    if !found_daemon || !target_bin.exists() {
        return Err(format!(
            "Could not find the {} binary for {} in the zapret2 release",
            engine.binary_name(),
            platform
        ));
    }

    apply_net_admin_cap(&target_bin);

    Ok(())
}

pub fn apply_net_admin_cap(binary: &std::path::Path) -> bool {
    #[cfg(target_os = "linux")]
    {
        match std::process::Command::new("setcap")
            .args(["cap_net_admin+ep", &binary.to_string_lossy()])
            .output()
        {
            Ok(output) if output.status.success() => {
                println!("{} {}", rust_i18n::t!("msg_setcap_ok"), binary.display());
                true
            }
            Ok(_) | Err(_) => {
                println!("{} {}", rust_i18n::t!("msg_setcap_skip"), binary.display());
                false
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = binary;
        true
    }
}

pub fn install_zapret2(version: &str) -> Result<(), String> {
    download_zapret2_strategies()?;
    download_zapret2(version)
}

pub fn install_everything() -> Result<(), String> {
    println!("=======================================================");
    println!("{}", rust_i18n::t!("msg_inst_all"));
    println!("=======================================================");

    let mut errors: Vec<String> = Vec::new();

    println!();
    println!("[1/4] {}", rust_i18n::t!("msg_step_z1_bin"));
    if let Err(e) = download_nfqws(ZAPRET_REC_VER) {
        errors.push(format!("zapret1 nfqws: {}", e));
    }

    println!();
    println!("[2/4] {}", rust_i18n::t!("msg_step_z1_strat"));
    if let Err(e) = download_strategies("recommended") {
        errors.push(format!("zapret1 strategies: {}", e));
    }

    println!();
    println!("[3/4] {}", rust_i18n::t!("msg_step_z2_payload"));
    if let Err(e) = download_zapret2_strategies() {
        errors.push(format!("zapret2 payload: {}", e));
    }

    println!();
    println!("[4/4] {}", rust_i18n::t!("msg_step_z2_bin"));
    if let Err(e) = download_zapret2("latest") {
        errors.push(format!("zapret2 nfqws2: {}", e));
    }

    let nfqws2_bin = crate::config::ZapretEngine::Zapret2.binary_path();
    if nfqws2_bin.exists() {
        apply_net_admin_cap(&nfqws2_bin);
    }
    let nfqws_bin = crate::config::ZapretEngine::Zapret1.binary_path();
    if nfqws_bin.exists() {
        apply_net_admin_cap(&nfqws_bin);
    }

    println!();
    println!("--- {} ---", rust_i18n::t!("msg_inst_summary"));
    print_provisioning_summary();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn mark(ok: bool) -> &'static str {
    if ok {
        "OK"
    } else {
        "MISSING"
    }
}

pub fn print_provisioning_summary() {
    let z1 = crate::config::ZapretEngine::Zapret1;
    let z2 = crate::config::ZapretEngine::Zapret2;
    let ws2 = z2.workspace_dir();

    println!("  nfqws            : {}", mark(z1.binary_path().exists()));
    println!(
        "  zapret1 strategies: {}",
        mark(check_strategies_installed_for(&z1))
    );
    println!("  nfqws2           : {}", mark(z2.binary_path().exists()));
    println!("  zapret2 presets  : {}", mark(dir_has_entries(&ws2.join("presets"))));
    println!("  zapret2 lists    : {}", mark(dir_has_entries(&ws2.join("lists"))));
    println!("  zapret2 lua      : {}", mark(dir_has_entries(&ws2.join("lua"))));
    println!("  zapret2 bin blobs: {}", mark(dir_has_entries(&ws2.join("bin"))));
    println!("  zapret2 scripts  : {}", mark(dir_has_entries(&ws2.join("scripts"))));
    println!("  workspace        : {}", ws2.display());
}

pub fn dir_has_entries(dir: &std::path::Path) -> bool {
    std::fs::read_dir(dir)
        .map(|mut e| e.next().is_some())
        .unwrap_or(false)
}

pub fn check_zapret2_payload_installed() -> bool {
    let ws = crate::config::ZapretEngine::Zapret2.workspace_dir();
    dir_has_entries(&ws.join("presets")) && dir_has_entries(&ws.join("lists")) && dir_has_entries(&ws.join("lua"))
}

pub fn download_nfqws(version: &str) -> Result<(), String> {
    if version == "skip" {
        return Ok(());
    }

    println!("{}", rust_i18n::t!("msg_chk_nfqws"));

    let bin_dir = crate::config::get_cache_dir().join("bin");
    let _ = fs::create_dir_all(&bin_dir);

    let platform = detect_platform_dir()?;
    println!("{}{}", rust_i18n::t!("msg_det_plat"), platform);

    let tag = if version == "latest" {
        println!("{}", rust_i18n::t!("msg_fetch_rel"));
        let latest_url = format!("https://api.github.com/repos/{}/releases/latest", ZAPRET_REPO);
        let req = ureq::get(&latest_url)
            .set("User-Agent", "zapret-rust")
            .call()
            .map_err(|e| format!("{}{}", rust_i18n::t!("err_fetch_rel"), e))?;

        let json_str = req.into_string().unwrap_or_else(|_| "{}".to_string());
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap_or_default();
        parsed
            .get("tag_name")
            .and_then(|t| t.as_str())
            .unwrap_or(ZAPRET_REC_VER)
            .to_string()
    } else {
        version.to_string()
    };

    println!("{}{}", rust_i18n::t!("msg_using_tag"), tag);
    let archive = format!("zapret-{}.tar.gz", tag);
    let url = format!(
        "https://github.com/{}/releases/download/{}/{}",
        ZAPRET_REPO, tag, archive
    );

    let tmp_dir = crate::config::get_cache_dir().join(".tmp_zapret_download");
    let _ = fs::remove_dir_all(&tmp_dir);
    let _ = fs::create_dir_all(&tmp_dir);
    let tmp_archive = tmp_dir.join(&archive);

    println!("{}{}", rust_i18n::t!("msg_dl_arc"), url);
    let mut response = ureq::get(&url)
        .call()
        .map_err(|e| format!("{}{}", rust_i18n::t!("err_dl_arc"), e))?
        .into_reader();
    let mut file = fs::File::create(&tmp_archive).map_err(|e| format!("{}{}", rust_i18n::t!("err_create_file"), e))?;
    std::io::copy(&mut response, &mut file).map_err(|e| format!("{}{}", rust_i18n::t!("err_write_arc"), e))?;

    println!("{}", rust_i18n::t!("msg_ext_arc"));
    let tar_gz = fs::File::open(&tmp_archive).map_err(|e| format!("{}{}", rust_i18n::t!("err_open_arc"), e))?;
    let tar = flate2::read::GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(tar);
    archive
        .unpack(&tmp_dir)
        .map_err(|e| format!("{}{}", rust_i18n::t!("err_unpack_tar"), e))?;

    let expected_bin_path = tmp_dir.join(format!("zapret-{}", tag)).join("binaries").join(platform);

    if expected_bin_path.exists() {
        if env::consts::OS == "windows" {
            for entry in fs::read_dir(&expected_bin_path)
                .map_err(|e| format!("{}{}", rust_i18n::t!("err_read_bin"), e))?
                .flatten()
            {
                let file_name = entry.file_name();
                fs::copy(entry.path(), bin_dir.join(&file_name))
                    .map_err(|e| format!("{}{:?}: {}", rust_i18n::t!("err_copy_file"), file_name, e))?;
            }
            println!("{}", rust_i18n::t!("msg_inst_win_ok"));
        } else {
            let bin_name = "nfqws";
            let bin_file = expected_bin_path.join(bin_name);
            if bin_file.exists() {
                let dest = bin_dir.join(bin_name);
                let _ = fs::remove_file(&dest);
                fs::copy(&bin_file, &dest).map_err(|e| format!("{}{}", rust_i18n::t!("err_copy_bin"), e))?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(mut perms) = fs::metadata(bin_dir.join(bin_name)).map(|m| m.permissions()) {
                        perms.set_mode(0o755);
                        let _ = fs::set_permissions(bin_dir.join(bin_name), perms);
                    }
                }
                println!("{}{}", rust_i18n::t!("msg_inst_nux_ok"), bin_name);

                apply_net_admin_cap(&bin_dir.join(bin_name));
            } else {
                return Err(format!("Could not find {} in {:?}", bin_name, expected_bin_path));
            }
        }
    } else {
        return Err(format!("Could not find binaries path {:?}", expected_bin_path));
    }

    let _ = fs::remove_dir_all(tmp_dir);
    Ok(())
}

pub fn download_strategies(version: &str) -> Result<(), String> {
    if version == "skip" {
        return Ok(());
    }

    let target_dir = crate::config::get_cache_dir().join("zapret-discord-youtube-linux");

    let url = if version == "latest" {
        STRAT_REPO_ZIP.to_string()
    } else if version == "recommended" {
        format!(
            "https://github.com/Flowseal/zapret-discord-youtube/archive/{}.zip",
            STRAT_REC_VER
        )
    } else {
        format!(
            "https://github.com/Flowseal/zapret-discord-youtube/archive/{}.zip",
            version
        )
    };

    println!("{}", rust_i18n::t!("msg_dl_strat"));
    let req = ureq::get(&url)
        .call()
        .map_err(|e| format!("{}{}", rust_i18n::t!("err_dl_strat_zip"), e))?;
    let mut body = req.into_reader();

    let tmp_zip = crate::config::get_cache_dir().join(".tmp_strategies.zip");
    let mut file = fs::File::create(&tmp_zip).map_err(|e| format!("{}{}", rust_i18n::t!("err_create_tmp_zip"), e))?;
    std::io::copy(&mut body, &mut file).map_err(|e| format!("{}{}", rust_i18n::t!("err_write_zip"), e))?;

    println!("{}", rust_i18n::t!("msg_ext_strat"));
    let zip_file = fs::File::open(&tmp_zip).map_err(|e| format!("{}{}", rust_i18n::t!("err_open_zip"), e))?;
    let mut archive = zip::ZipArchive::new(zip_file).map_err(|e| format!("{}{}", rust_i18n::t!("err_read_zip"), e))?;

    let _ = fs::create_dir_all(&target_dir);

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => {
                let mut components = path.components();
                components.next();
                components.as_path().to_owned()
            }
            None => continue,
        };

        if outpath.as_os_str().is_empty() {
            continue;
        }

        let full_path = target_dir.join(outpath);

        if (*file.name()).ends_with('/') {
            fs::create_dir_all(&full_path).map_err(|e| format!("{}{}", rust_i18n::t!("err_mkdir"), e))?;
        } else {
            if let Some(p) = full_path.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).map_err(|e| format!("{}{}", rust_i18n::t!("err_mkdir"), e))?;
                }
            }
            let mut outfile =
                fs::File::create(&full_path).map_err(|e| format!("{}{}", rust_i18n::t!("err_extract"), e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("{}{}", rust_i18n::t!("err_copy_content"), e))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&full_path, fs::Permissions::from_mode(mode)).ok();
                }
            }
        }
    }

    let _ = fs::remove_file(tmp_zip);
    println!("{}", rust_i18n::t!("msg_strat_ok"));
    Ok(())
}

pub fn install_dependencies(nfqws_ver: &str, strat_ver: &str) -> Result<(), String> {
    println!("=======================================================");
    println!("{}", rust_i18n::t!("msg_inst_deps"));
    println!("=======================================================");

    let mut errors = Vec::new();
    if nfqws_ver != "skip" {
        if let Err(e) = download_nfqws(nfqws_ver) {
            errors.push(format!("nfqws error: {}", e));
        }
    }
    if strat_ver != "skip" {
        if let Err(e) = download_strategies(strat_ver) {
            errors.push(format!("strategies error: {}", e));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

pub fn check_nfqws_installed_for(engine: &crate::config::ZapretEngine) -> bool {
    engine.binary_path().exists()
}

pub fn check_nfqws_installed() -> bool {
    check_nfqws_installed_for(&crate::runner::active_engine())
}

fn has_bat_files(dir: &std::path::Path) -> bool {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                e.file_name()
                    .into_string()
                    .map(|n| n.ends_with(".bat"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

pub fn check_strategies_installed_for(engine: &crate::config::ZapretEngine) -> bool {
    match engine {
        crate::config::ZapretEngine::Zapret2 => {
            !crate::strategy::zapret2_presets().is_empty() && check_zapret2_payload_installed()
        }
        crate::config::ZapretEngine::Zapret1 => {
            let repo_dir = engine.workspace_dir();
            if !repo_dir.exists() {
                return false;
            }
            has_bat_files(&repo_dir)
                || has_bat_files(&repo_dir.join("custom-strategies"))
                || has_bat_files(&crate::config::get_cache_dir().join("custom-strategies"))
        }
    }
}

pub fn check_strategies_installed() -> bool {
    check_strategies_installed_for(&crate::runner::active_engine())
}

pub fn fetch_repo_tags(repo: &str) -> Result<Vec<String>, String> {
    let url = format!("https://api.github.com/repos/{}/tags", repo);
    let req = ureq::get(&url)
        .set("User-Agent", "zapret-rust-tui")
        .call()
        .map_err(|e| format!("{}{}: {}", rust_i18n::t!("err_fetch_tags"), repo, e))?;

    let json_str = req
        .into_string()
        .map_err(|e| format!("{}{}", rust_i18n::t!("err_read_tags"), e))?;
    let tags_json: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| format!("{}{}", rust_i18n::t!("err_parse_tags"), e))?;
    let mut tags = Vec::new();

    if let Some(arr) = tags_json.as_array() {
        for item in arr {
            if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                tags.push(name.to_string());
            }
        }
    }

    Ok(tags)
}
