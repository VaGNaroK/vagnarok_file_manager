//! Leitura e escrita nativa de ACLs NTFS no Windows via APIs Win32 (`advapi32`),
//! sem PowerShell nem subprocessos externos.

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use fm_core::{FsError, NtfsPermissionBits, NtfsPrincipalAcl, NtfsSecurityInfo};
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSidToSidW, DENY_ACCESS, EXPLICIT_ACCESS_W, GRANT_ACCESS,
    GetNamedSecurityInfoW, SE_FILE_OBJECT, SetEntriesInAclW, SetNamedSecurityInfoW, TRUSTEE_IS_SID,
    TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
};
use windows::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACCESS_DENIED_ACE, ACE_FLAGS, ACE_HEADER, ACL, ACL_SIZE_INFORMATION,
    AclSizeInformation, CONTAINER_INHERIT_ACE, DACL_SECURITY_INFORMATION, GetAce,
    GetAclInformation, GetSecurityDescriptorControl, INHERITED_ACE, LookupAccountNameW,
    LookupAccountSidW, NO_INHERITANCE, OBJECT_INHERIT_ACE, OWNER_SECURITY_INFORMATION,
    PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, SE_DACL_PROTECTED,
    SID_NAME_USE, SidTypeAlias, SidTypeGroup, SidTypeWellKnownGroup,
    UNPROTECTED_DACL_SECURITY_INFORMATION,
};
use windows::core::{PCWSTR, PWSTR};

const ACCESS_ALLOWED_ACE_TYPE_U8: u8 = 0;
const ACCESS_DENIED_ACE_TYPE_U8: u8 = 1;

struct LocalMemGuard(*mut c_void);

impl Drop for LocalMemGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: ponteiro alocado pelas APIs Win32 (`GetNamedSecurityInfoW`, `SetEntriesInAclW`, `ConvertSidToStringSidW`, `ConvertStringSidToSidW`).
            let _ = unsafe { LocalFree(Some(HLOCAL(self.0))) };
        }
    }
}

fn to_wide_nul(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn path_to_wide_nul(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn sid_to_string(sid: PSID) -> Option<String> {
    let mut raw_ptr = PWSTR::null();
    // SAFETY: `sid` é um `PSID` válido pertencente ao descriptor/ACE atual.
    unsafe { ConvertSidToStringSidW(sid, &mut raw_ptr) }.ok()?;
    let _guard = LocalMemGuard(raw_ptr.0.cast());
    // SAFETY: `raw_ptr` é uma string UTF-16 terminada em NUL alocada pelo Windows.
    let s = unsafe { raw_ptr.to_string() }.ok()?;
    Some(s)
}

fn lookup_sid_account(sid: PSID) -> (String, String, bool) {
    let mut name_len = 0u32;
    let mut domain_len = 0u32;
    let mut use_kind = SID_NAME_USE::default();
    // SAFETY: primeira chamada com buffers nulos apenas consulta o tamanho necessário.
    let _ = unsafe {
        LookupAccountSidW(
            PCWSTR::null(),
            sid,
            None,
            &mut name_len,
            None,
            &mut domain_len,
            &mut use_kind,
        )
    };
    if name_len == 0 {
        let fallback = sid_to_string(sid).unwrap_or_else(|| "SID".to_owned());
        return (fallback, String::new(), false);
    }
    let mut name_buf = vec![0u16; name_len as usize];
    let mut domain_buf = vec![0u16; domain_len.max(1) as usize];
    // SAFETY: buffers dimensionados exatamente conforme retornado na consulta anterior.
    let ok = unsafe {
        LookupAccountSidW(
            PCWSTR::null(),
            sid,
            Some(PWSTR(name_buf.as_mut_ptr())),
            &mut name_len,
            Some(PWSTR(domain_buf.as_mut_ptr())),
            &mut domain_len,
            &mut use_kind,
        )
    }
    .is_ok();
    if !ok {
        let fallback = sid_to_string(sid).unwrap_or_else(|| "SID".to_owned());
        return (fallback, String::new(), false);
    }
    let name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
    let domain = String::from_utf16_lossy(&domain_buf[..domain_len as usize]);
    let is_group =
        use_kind == SidTypeGroup || use_kind == SidTypeAlias || use_kind == SidTypeWellKnownGroup;
    (name, domain, is_group)
}

/// Lê o proprietário, estado de herança e lista de usuários/grupos da DACL NTFS do caminho.
pub fn read_ntfs_security_blocking(path: &Path) -> Result<NtfsSecurityInfo, FsError> {
    let is_dir = path.is_dir();
    let wide = path_to_wide_nul(path);
    let mut owner_sid = PSID::default();
    let mut dacl_ptr: *mut ACL = std::ptr::null_mut();
    let mut sd = PSECURITY_DESCRIPTOR::default();

    // SAFETY: `wide` é UTF-16 null-terminated e todos os ponteiros de saída são válidos na stack.
    let status = unsafe {
        GetNamedSecurityInfoW(
            PCWSTR(wide.as_ptr()),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            Some(&mut owner_sid),
            None,
            Some(&mut dacl_ptr),
            None,
            &mut sd,
        )
    };
    if status.is_err() {
        return Err(FsError::Io {
            path: path.to_path_buf(),
            message: format!("falha ao ler segurança NTFS ({status:?})"),
        });
    }
    let _sd_guard = LocalMemGuard(sd.0);

    let owner = if !owner_sid.is_invalid() {
        let (name, domain, _) = lookup_sid_account(owner_sid);
        if domain.is_empty() {
            name
        } else {
            format!("{domain}\\{name}")
        }
    } else {
        "—".to_owned()
    };

    let mut control = 0u16;
    let mut revision = 0u32;
    // SAFETY: `sd` é um `PSECURITY_DESCRIPTOR` válido retornado por `GetNamedSecurityInfoW`.
    let inheritance_enabled =
        unsafe { GetSecurityDescriptorControl(sd, &mut control, &mut revision) }.is_ok()
            && (control & SE_DACL_PROTECTED.0) == 0;

    let mut principals_map: BTreeMap<String, NtfsPrincipalAcl> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();

    if !dacl_ptr.is_null() {
        let mut acl_info = ACL_SIZE_INFORMATION::default();
        // SAFETY: `dacl_ptr` pertence a `sd` que permanece vivo até o fim da função.
        let got_info = unsafe {
            GetAclInformation(
                dacl_ptr,
                (&raw mut acl_info).cast(),
                std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
                AclSizeInformation,
            )
        }
        .is_ok();

        if got_info {
            for idx in 0..acl_info.AceCount {
                let mut ace_ptr: *mut c_void = std::ptr::null_mut();
                // SAFETY: `idx < acl_info.AceCount`.
                if unsafe { GetAce(dacl_ptr, idx, &mut ace_ptr) }.is_err() || ace_ptr.is_null() {
                    continue;
                }
                // SAFETY: toda ACE começa com `ACE_HEADER`.
                let header = unsafe { &*(ace_ptr as *const ACE_HEADER) };
                let is_inherited = (header.AceFlags & INHERITED_ACE.0 as u8) != 0;

                let (is_allow, mask, sid_ptr) = match header.AceType {
                    ACCESS_ALLOWED_ACE_TYPE_U8 => {
                        // SAFETY: `AceType == ACCESS_ALLOWED_ACE_TYPE`.
                        let ace = unsafe { &*(ace_ptr as *const ACCESS_ALLOWED_ACE) };
                        let sid = PSID((&raw const ace.SidStart) as *mut c_void);
                        (true, ace.Mask, sid)
                    }
                    ACCESS_DENIED_ACE_TYPE_U8 => {
                        // SAFETY: `AceType == ACCESS_DENIED_ACE_TYPE`.
                        let ace = unsafe { &*(ace_ptr as *const ACCESS_DENIED_ACE) };
                        let sid = PSID((&raw const ace.SidStart) as *mut c_void);
                        (false, ace.Mask, sid)
                    }
                    _ => continue,
                };

                let Some(sid_str) = sid_to_string(sid_ptr) else {
                    continue;
                };
                let bits = NtfsPermissionBits::from_win32_mask(mask, is_dir);
                if !principals_map.contains_key(&sid_str) {
                    let (display_name, domain, is_group) = lookup_sid_account(sid_ptr);
                    order.push(sid_str.clone());
                    principals_map.insert(
                        sid_str.clone(),
                        NtfsPrincipalAcl {
                            sid: sid_str.clone(),
                            display_name,
                            domain,
                            is_group,
                            is_inherited,
                            allow: NtfsPermissionBits::default(),
                            deny: NtfsPermissionBits::default(),
                        },
                    );
                }
                if let Some(entry) = principals_map.get_mut(&sid_str) {
                    if !is_inherited {
                        entry.is_inherited = false;
                    }
                    if is_allow {
                        entry.allow.merge_from(bits);
                    } else {
                        entry.deny.merge_from(bits);
                    }
                }
            }
        }
    }

    let principals = order
        .into_iter()
        .filter_map(|sid| principals_map.remove(&sid))
        .collect();

    Ok(NtfsSecurityInfo {
        owner,
        inheritance_enabled,
        principals,
    })
}

/// Resolve um nome de conta/grupo do Windows (ex.: `Usuários`, `Everyone`, `Administradores`) em um `NtfsPrincipalAcl`.
pub fn lookup_ntfs_account_blocking(account_name: &str) -> Result<NtfsPrincipalAcl, FsError> {
    let trimmed = account_name.trim();
    if trimmed.is_empty() {
        return Err(FsError::Io {
            path: std::path::PathBuf::new(),
            message: "informe um nome de usuário ou grupo válido".to_owned(),
        });
    }
    let wide = to_wide_nul(trimmed);
    let mut sid_size = 0u32;
    let mut domain_len = 0u32;
    let mut use_kind = SID_NAME_USE::default();
    // SAFETY: consulta inicial de tamanho dos buffers.
    let _ = unsafe {
        LookupAccountNameW(
            PCWSTR::null(),
            PCWSTR(wide.as_ptr()),
            None,
            &mut sid_size,
            None,
            &mut domain_len,
            &mut use_kind,
        )
    };
    if sid_size == 0 {
        return Err(FsError::Io {
            path: std::path::PathBuf::new(),
            message: format!("usuário ou grupo não encontrado: {trimmed}"),
        });
    }
    let mut sid_buf = vec![0u8; sid_size as usize];
    let mut domain_buf = vec![0u16; domain_len.max(1) as usize];
    // SAFETY: buffers dimensionados conforme consulta anterior.
    unsafe {
        LookupAccountNameW(
            PCWSTR::null(),
            PCWSTR(wide.as_ptr()),
            Some(PSID(sid_buf.as_mut_ptr().cast())),
            &mut sid_size,
            Some(PWSTR(domain_buf.as_mut_ptr())),
            &mut domain_len,
            &mut use_kind,
        )
    }
    .map_err(|e| FsError::Io {
        path: std::path::PathBuf::new(),
        message: format!("usuário ou grupo não encontrado ({trimmed}): {e}"),
    })?;

    let psid = PSID(sid_buf.as_mut_ptr().cast());
    let sid = sid_to_string(psid).ok_or_else(|| FsError::Io {
        path: std::path::PathBuf::new(),
        message: format!("falha ao converter SID de {trimmed}"),
    })?;
    let (display_name, domain, is_group) = lookup_sid_account(psid);

    Ok(NtfsPrincipalAcl {
        sid,
        display_name,
        domain,
        is_group,
        is_inherited: false,
        allow: NtfsPermissionBits {
            read: true,
            write: false,
            execute: true,
            list_dir: true,
            delete: false,
        },
        deny: NtfsPermissionBits::default(),
    })
}

/// Aplica uma nova DACL NTFS e estado de herança no caminho informado via `SetEntriesInAclW` + `SetNamedSecurityInfoW`.
pub fn apply_ntfs_security_blocking(path: &Path, info: &NtfsSecurityInfo) -> Result<(), FsError> {
    let is_dir = path.is_dir();
    let inheritance_flags: ACE_FLAGS = if is_dir {
        OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE
    } else {
        NO_INHERITANCE
    };

    let mut sid_guards: Vec<LocalMemGuard> = Vec::new();
    let mut explicit_entries: Vec<EXPLICIT_ACCESS_W> = Vec::new();

    for principal in &info.principals {
        if info.inheritance_enabled && principal.is_inherited {
            // Mantém ACEs puramente herdadas para serem propagadas pelo diretório pai.
            continue;
        }
        let sid_wide = to_wide_nul(&principal.sid);
        let mut psid = PSID::default();
        // SAFETY: `sid_wide` é uma string SID válida terminada em NUL.
        if unsafe { ConvertStringSidToSidW(PCWSTR(sid_wide.as_ptr()), &mut psid) }.is_err()
            || psid.is_invalid()
        {
            continue;
        }
        sid_guards.push(LocalMemGuard(psid.0));

        let allow_mask = principal.allow.to_win32_mask(is_dir);
        if allow_mask != 0 {
            explicit_entries.push(EXPLICIT_ACCESS_W {
                grfAccessPermissions: allow_mask,
                grfAccessMode: GRANT_ACCESS,
                grfInheritance: inheritance_flags,
                Trustee: TRUSTEE_W {
                    pMultipleTrustee: std::ptr::null_mut(),
                    MultipleTrusteeOperation: Default::default(),
                    TrusteeForm: TRUSTEE_IS_SID,
                    TrusteeType: TRUSTEE_IS_UNKNOWN,
                    ptstrName: PWSTR(psid.0.cast()),
                },
            });
        }

        let deny_mask = principal.deny.to_win32_mask(is_dir);
        if deny_mask != 0 {
            explicit_entries.push(EXPLICIT_ACCESS_W {
                grfAccessPermissions: deny_mask,
                grfAccessMode: DENY_ACCESS,
                grfInheritance: inheritance_flags,
                Trustee: TRUSTEE_W {
                    pMultipleTrustee: std::ptr::null_mut(),
                    MultipleTrusteeOperation: Default::default(),
                    TrusteeForm: TRUSTEE_IS_SID,
                    TrusteeType: TRUSTEE_IS_UNKNOWN,
                    ptstrName: PWSTR(psid.0.cast()),
                },
            });
        }
    }

    let mut new_dacl: *mut ACL = std::ptr::null_mut();
    // SAFETY: `explicit_entries` referencia SIDs mantidos vivos em `sid_guards`.
    let acl_status = unsafe {
        SetEntriesInAclW(
            (!explicit_entries.is_empty()).then_some(&explicit_entries),
            None,
            &mut new_dacl,
        )
    };
    if acl_status.is_err() {
        return Err(FsError::Io {
            path: path.to_path_buf(),
            message: format!("falha ao construir ACL NTFS ({acl_status:?})"),
        });
    }
    let _dacl_guard = LocalMemGuard(new_dacl.cast());

    let sec_info = DACL_SECURITY_INFORMATION
        | if info.inheritance_enabled {
            UNPROTECTED_DACL_SECURITY_INFORMATION
        } else {
            PROTECTED_DACL_SECURITY_INFORMATION
        };

    let wide = path_to_wide_nul(path);
    // SAFETY: `wide` é UTF-16 null-terminated e `new_dacl` é válido até `_dacl_guard` cair.
    let set_status = unsafe {
        SetNamedSecurityInfoW(
            PCWSTR(wide.as_ptr()),
            SE_FILE_OBJECT,
            sec_info,
            None,
            None,
            Some(new_dacl),
            None,
        )
    };
    if set_status.is_err() {
        return Err(FsError::PermissionDenied(path.to_path_buf()));
    }

    Ok(())
}
