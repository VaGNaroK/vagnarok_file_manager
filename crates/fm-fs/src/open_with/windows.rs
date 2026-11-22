//! Descoberta de aplicativos no Registro Win32 e execução personalizada no Windows.

use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::Path;
use std::process::Command;

use compact_str::CompactString;
use fm_core::{FsError, OpenWithApp, OpenWithAssociation, OpenWithCatalog};
use tokio_util::sync::CancellationToken;
use windows::Win32::Foundation::{ERROR_NO_MORE_ITEMS, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, REG_EXPAND_SZ,
    REG_SZ, REG_VALUE_TYPE, RegCloseKey, RegEnumKeyExW, RegEnumValueW, RegOpenKeyExW,
    RegQueryValueExW,
};
use windows::core::{PCWSTR, PWSTR};

use super::split_windows_command_tokens;

struct OwnedHkey(HKEY);
impl Drop for OwnedHkey {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = RegCloseKey(self.0);
            }
        }
    }
}

pub(super) fn discover_open_with_catalog(
    path: &Path,
    mime_hint: Option<&str>,
    cancel: &CancellationToken,
) -> Result<OpenWithCatalog, FsError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_ascii_lowercase()));
    let type_key = ext
        .clone()
        .unwrap_or_else(|| super::open_with_type_key_for_path(path, mime_hint));

    let type_label = ext
        .as_deref()
        .and_then(|e| read_default_string(HKEY_CLASSES_ROOT, e))
        .and_then(|prog_id| read_default_string(HKEY_CLASSES_ROOT, &prog_id))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| match &ext {
            Some(e) => e.trim_start_matches('.').to_ascii_uppercase(),
            None => "Arquivo".to_owned(),
        });

    let mut default_prog_id: Option<String> = None;
    let mut recommended_ids: HashSet<String> = HashSet::new();
    let mut seen_execs: HashMap<String, OpenWithApp> = HashMap::new();

    if let Some(ext_name) = ext.as_deref() {
        let file_exts_base =
            format!(r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\{ext_name}");
        if let Some(prog) = read_string_value(
            HKEY_CURRENT_USER,
            &format!(r"{file_exts_base}\UserChoice"),
            "ProgId",
        ) {
            default_prog_id = Some(prog.clone());
            recommended_ids.insert(prog);
        }

        for val_name in enum_value_names(
            HKEY_CURRENT_USER,
            &format!(r"{file_exts_base}\OpenWithProgids"),
        ) {
            recommended_ids.insert(val_name);
        }
        for app_exe in enum_string_values(
            HKEY_CURRENT_USER,
            &format!(r"{file_exts_base}\OpenWithList"),
        ) {
            recommended_ids.insert(format!(r"Applications\{app_exe}"));
        }
        for val_name in enum_value_names(HKEY_CLASSES_ROOT, &format!(r"{ext_name}\OpenWithProgids"))
        {
            recommended_ids.insert(val_name);
        }
    }

    let mut candidate_keys: Vec<(String, bool)> =
        recommended_ids.iter().map(|k| (k.clone(), true)).collect();

    for app_key in enum_subkeys(HKEY_CLASSES_ROOT, "Applications") {
        if cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        let full = format!(r"Applications\{app_key}");
        let is_rec = recommended_ids.contains(&full);
        candidate_keys.push((full, is_rec));
    }

    for (key_path, is_recommended) in candidate_keys {
        if cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        if let Some(app) = resolve_progid_app(&key_path, &default_prog_id, is_recommended) {
            let dedup = app.exec.to_ascii_lowercase();
            seen_execs
                .entry(dedup)
                .and_modify(|existing| {
                    existing.is_recommended |= app.is_recommended;
                    existing.is_default |= app.is_default;
                })
                .or_insert(app);
        }
    }

    // Adiciona aplicativos registrados em App Paths como fallback extra.
    for app_exe in enum_subkeys(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths",
    ) {
        if cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        let reg_key = format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{app_exe}");
        if let Some(raw_exe) = read_default_string(HKEY_LOCAL_MACHINE, &reg_key) {
            let clean_exe = expand_env_vars(raw_exe.trim().trim_matches('"'));
            if clean_exe.is_empty() || !Path::new(&clean_exe).is_file() {
                continue;
            }
            let exec_tpl = format!("\"{clean_exe}\" \"%1\"");
            let dedup = exec_tpl.to_ascii_lowercase();
            if seen_execs.contains_key(&dedup) {
                continue;
            }
            let name = Path::new(&clean_exe)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| app_exe.clone());
            seen_execs.insert(
                dedup,
                OpenWithApp {
                    id: CompactString::from(format!(r"AppPaths\{app_exe}")),
                    display_name: CompactString::from(name),
                    exec: exec_tpl,
                    icon_hint: Some(clean_exe),
                    is_recommended: false,
                    is_default: false,
                    is_custom: false,
                    runs_in_terminal: false,
                },
            );
        }
    }

    let mut system_default = None;
    let mut recommended = Vec::new();
    let mut others = Vec::new();

    for app in seen_execs.into_values() {
        if app.is_default && system_default.is_none() {
            system_default = Some(app);
        } else if app.is_recommended {
            recommended.push(app);
        } else {
            others.push(app);
        }
    }

    recommended.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });
    others.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });

    Ok(OpenWithCatalog {
        type_key: CompactString::from(type_key),
        type_label: CompactString::from(type_label),
        system_default,
        recommended,
        others,
    })
}

pub(super) fn save_system_default(
    _type_key: &str,
    _assoc: &OpenWithAssociation,
) -> Result<(), FsError> {
    // No Windows 10/11, HKCU\...\UserChoice é protegido por hash proprietário;
    // a preferência do usuário é persistida com segurança em AppConfig::open_with_defaults.
    Ok(())
}

pub(super) fn reset_system_default(_type_key: &str) -> Result<(), FsError> {
    Ok(())
}

pub(super) fn launch_open_with(path: &Path, target: &OpenWithAssociation) -> Result<(), FsError> {
    let expanded_cmd = expand_env_vars(target.exec.trim());
    let tokens = split_windows_command_tokens(&expanded_cmd);
    let Some((exe_raw, rest)) = tokens.split_first() else {
        return Err(FsError::Io {
            path: path.to_path_buf(),
            message: "comando de aplicativo vazio".to_owned(),
        });
    };

    let file_str = path.to_string_lossy().into_owned();
    let mut args = Vec::with_capacity(rest.len() + 1);
    let mut replaced = false;

    for tok in rest {
        if tok == "%1" || tok == "%L" || tok == "%l" || tok == "%*" {
            args.push(file_str.clone());
            replaced = true;
        } else if tok.contains("%1") {
            args.push(tok.replace("%1", &file_str));
            replaced = true;
        } else {
            args.push(tok.clone());
        }
    }
    if !replaced {
        args.push(file_str);
    }

    let mut cmd = Command::new(exe_raw);
    cmd.args(&args);
    crate::windows::spawn_detached(&mut cmd).map_err(|err| FsError::Io {
        path: path.to_path_buf(),
        message: format!("falha ao abrir com '{}': {err}", target.display_name),
    })
}

fn resolve_progid_app(
    prog_id: &str,
    default_prog_id: &Option<String>,
    is_recommended: bool,
) -> Option<OpenWithApp> {
    let cmd_raw =
        read_default_string(HKEY_CLASSES_ROOT, &format!(r"{prog_id}\shell\open\command"))?;
    let cmd_expanded = expand_env_vars(cmd_raw.trim());
    let tokens = split_windows_command_tokens(&cmd_expanded);
    let exe_token = tokens.first()?.clone();
    if exe_token.is_empty() {
        return None;
    }

    let friendly = read_string_value(HKEY_CLASSES_ROOT, prog_id, "FriendlyAppName")
        .or_else(|| {
            read_string_value(
                HKEY_CLASSES_ROOT,
                &format!(r"{prog_id}\shell\open"),
                "FriendlyAppName",
            )
        })
        .or_else(|| read_default_string(HKEY_CLASSES_ROOT, prog_id))
        .filter(|s| !s.trim().is_empty() && !s.starts_with('@'))
        .unwrap_or_else(|| {
            Path::new(&exe_token)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| prog_id.to_owned())
        });

    let icon_hint = read_default_string(HKEY_CLASSES_ROOT, &format!(r"{prog_id}\DefaultIcon"))
        .map(|s| expand_env_vars(&s))
        .filter(|s| !s.trim().is_empty())
        .or(Some(exe_token));

    let is_default = default_prog_id
        .as_deref()
        .is_some_and(|def| def.eq_ignore_ascii_case(prog_id));

    Some(OpenWithApp {
        id: CompactString::from(prog_id),
        display_name: CompactString::from(friendly),
        exec: cmd_expanded,
        icon_hint,
        is_recommended: is_recommended || is_default,
        is_default,
        is_custom: false,
        runs_in_terminal: false,
    })
}

fn to_wide_null(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn open_reg_key(root: HKEY, subkey: &str) -> Option<OwnedHkey> {
    let wide = to_wide_null(subkey);
    let mut handle = HKEY::default();
    let status =
        unsafe { RegOpenKeyExW(root, PCWSTR(wide.as_ptr()), Some(0), KEY_READ, &mut handle) };
    if status == ERROR_SUCCESS && !handle.is_invalid() {
        Some(OwnedHkey(handle))
    } else {
        None
    }
}

fn read_default_string(root: HKEY, subkey: &str) -> Option<String> {
    read_string_value_raw(root, subkey, None)
}

fn read_string_value(root: HKEY, subkey: &str, value_name: &str) -> Option<String> {
    read_string_value_raw(root, subkey, Some(value_name))
}

fn read_string_value_raw(root: HKEY, subkey: &str, value_name: Option<&str>) -> Option<String> {
    let key = open_reg_key(root, subkey)?;
    let wide_val = value_name.map(to_wide_null);
    let val_ptr = wide_val
        .as_ref()
        .map_or(PCWSTR::null(), |w| PCWSTR(w.as_ptr()));

    let mut reg_type = REG_VALUE_TYPE::default();
    let mut byte_len: u32 = 0;
    let probe = unsafe {
        RegQueryValueExW(
            key.0,
            val_ptr,
            None,
            Some(&mut reg_type),
            None,
            Some(&mut byte_len),
        )
    };
    if probe != ERROR_SUCCESS || byte_len == 0 || (reg_type != REG_SZ && reg_type != REG_EXPAND_SZ)
    {
        return None;
    }

    let mut buf = vec![0u16; (byte_len as usize).div_ceil(2)];
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            val_ptr,
            None,
            Some(&mut reg_type),
            Some(buf.as_mut_ptr().cast()),
            Some(&mut byte_len),
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let u16_len = (byte_len as usize / 2).min(buf.len());
    let slice = &buf[..u16_len];
    let trimmed_len = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
    let text = OsString::from_wide(&slice[..trimmed_len])
        .to_string_lossy()
        .into_owned();
    if reg_type == REG_EXPAND_SZ {
        Some(expand_env_vars(&text))
    } else {
        Some(text)
    }
}

fn enum_subkeys(root: HKEY, subkey: &str) -> Vec<String> {
    let Some(key) = open_reg_key(root, subkey) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut idx = 0u32;
    let mut buf = vec![0u16; 260];

    loop {
        let mut len = buf.len() as u32;
        let status = unsafe {
            RegEnumKeyExW(
                key.0,
                idx,
                Some(PWSTR(buf.as_mut_ptr())),
                &mut len,
                None,
                None,
                None,
                None,
            )
        };
        if status == ERROR_NO_MORE_ITEMS || status != ERROR_SUCCESS {
            break;
        }
        let name = OsString::from_wide(&buf[..len as usize])
            .to_string_lossy()
            .into_owned();
        if !name.is_empty() {
            out.push(name);
        }
        idx += 1;
    }
    out
}

fn enum_value_names(root: HKEY, subkey: &str) -> Vec<String> {
    let Some(key) = open_reg_key(root, subkey) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut idx = 0u32;
    let mut buf = vec![0u16; 260];

    loop {
        let mut len = buf.len() as u32;
        let status = unsafe {
            RegEnumValueW(
                key.0,
                idx,
                Some(PWSTR(buf.as_mut_ptr())),
                &mut len,
                None,
                None,
                None,
                None,
            )
        };
        if status == ERROR_NO_MORE_ITEMS || status != ERROR_SUCCESS {
            break;
        }
        let name = OsString::from_wide(&buf[..len as usize])
            .to_string_lossy()
            .into_owned();
        if !name.is_empty() {
            out.push(name);
        }
        idx += 1;
    }
    out
}

fn enum_string_values(root: HKEY, subkey: &str) -> Vec<String> {
    let mut out = Vec::new();
    for val_name in enum_value_names(root, subkey) {
        if val_name.eq_ignore_ascii_case("MRUList") {
            continue;
        }
        if let Some(s) = read_string_value(root, subkey, &val_name)
            && !s.trim().is_empty()
        {
            out.push(s);
        }
    }
    out
}

fn expand_env_vars(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        if let Some(end) = after.find('%') {
            let var_name = &after[..end];
            if !var_name.is_empty()
                && !var_name.chars().all(|c| c.is_ascii_digit())
                && let Some(val) = lookup_env_case_insensitive(var_name)
            {
                out.push_str(&val);
                rest = &after[end + 1..];
                continue;
            }
            out.push('%');
            out.push_str(var_name);
            out.push('%');
            rest = &after[end + 1..];
        } else {
            out.push('%');
            out.push_str(after);
            return out;
        }
    }
    out.push_str(rest);
    out
}

fn lookup_env_case_insensitive(name: &str) -> Option<String> {
    std::env::var(name).ok().or_else(|| {
        std::env::vars_os().find_map(|(k, v)| {
            k.to_str()
                .filter(|ks| ks.eq_ignore_ascii_case(name))
                .map(|_| v.to_string_lossy().into_owned())
        })
    })
}
