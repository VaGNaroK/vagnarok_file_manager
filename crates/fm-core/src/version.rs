//! Comparação SemVer e seleção de releases estáveis do GitHub.
//!
//! Módulo puro sem I/O nem dependências de rede ou sistema operacional.

use std::cmp::Ordering;

pub const GITHUB_OWNER: &str = "VaGNaroK";
pub const GITHUB_REPO: &str = "vagnarok_file_manager";
pub const GITHUB_RELEASES_API_URL: &str =
    "https://api.github.com/repos/VaGNaroK/vagnarok_file_manager/releases?per_page=10";
pub const GITHUB_RELEASES_PAGE_PREFIX: &str =
    "https://github.com/VaGNaroK/vagnarok_file_manager/releases/";
pub const GITHUB_RELEASES_FALLBACK_URL: &str =
    "https://github.com/VaGNaroK/vagnarok_file_manager/releases";

/// Intervalo mínimo (em segundos) entre verificações automáticas ao iniciar (6 horas).
pub const UPDATE_CHECK_COOLDOWN_SECS: u64 = 6 * 60 * 60;

/// Versão SemVer (`MAJOR.MINOR.PATCH[-PRERELEASE]`) para ordenação determinística.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Option<String>,
}

impl SemVer {
    /// Faz parsing de tags como `"v1.2.3"`, `"1.2.3"` ou `"v0.1.0-alpha.9"`.
    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        let no_prefix = trimmed
            .strip_prefix('v')
            .or_else(|| trimmed.strip_prefix('V'))
            .unwrap_or(trimmed);
        // Ignora metadados de build (`+...`) conforme SemVer 2.0.0.
        let without_build = no_prefix.split('+').next().unwrap_or(no_prefix);
        let (core, pre) = match without_build.split_once('-') {
            Some((c, p)) if !p.is_empty() => (c, Some(p.to_owned())),
            Some(_) => return None,
            None => (without_build, None),
        };
        let mut parts = core.split('.');
        let major = parts.next()?.parse::<u64>().ok()?;
        let minor = parts.next()?.parse::<u64>().ok()?;
        let patch = parts.next()?.parse::<u64>().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            major,
            minor,
            patch,
            pre,
        })
    }

    /// Retorna `true` apenas para versões estáveis (sem sufixo `-alpha`, `-beta`, `-rc`, etc.).
    pub fn is_stable(&self) -> bool {
        self.pre.is_none()
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
            .then_with(|| match (&self.pre, &other.pre) {
                (None, None) => Ordering::Equal,
                // Versão estável é sempre maior que qualquer pré-release do mesmo MAJOR.MINOR.PATCH.
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(a), Some(b)) => compare_prerelease(a, b),
            })
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn compare_prerelease(a: &str, b: &str) -> Ordering {
    let mut ia = a.split('.');
    let mut ib = b.split('.');
    loop {
        match (ia.next(), ib.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(pa), Some(pb)) => {
                let ord = match (pa.parse::<u64>(), pb.parse::<u64>()) {
                    (Ok(na), Ok(nb)) => na.cmp(&nb),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => pa.cmp(pb),
                };
                if ord != Ordering::Equal {
                    return ord;
                }
            }
        }
    }
}

/// Metadados normalizados de uma release candidata retornada pela API do GitHub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseCandidate {
    pub tag_name: String,
    pub html_url: String,
    pub draft: bool,
    pub prerelease: bool,
    pub published_at: Option<String>,
}

/// Informações prontas para exibição quando há uma versão estável mais recente disponível.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    pub version: String,
    pub tag_name: String,
    pub release_url: String,
    pub published_at: Option<String>,
}

/// Valida que a URL da release pertence estritamente ao repositório oficial no GitHub.
pub fn is_trusted_release_url(url: &str) -> bool {
    url == GITHUB_RELEASES_FALLBACK_URL || url.starts_with(GITHUB_RELEASES_PAGE_PREFIX)
}

/// Seleciona a release **estável** mais recente estritamente superior a `current_version`.
///
/// Rejeita rascunhos (`draft`), pré-releases (`prerelease: true`), tags com sufixo SemVer
/// (`-alpha`, `-beta`, `-rc`, etc.) e URLs fora do repositório oficial.
pub fn select_newer_stable_release(
    current_version: &str,
    candidates: &[ReleaseCandidate],
) -> Option<UpdateInfo> {
    let current = SemVer::parse(current_version)?;

    let mut best: Option<(SemVer, &ReleaseCandidate)> = None;
    for candidate in candidates {
        if candidate.draft || candidate.prerelease {
            continue;
        }
        if !is_trusted_release_url(&candidate.html_url) {
            continue;
        }
        let Some(parsed) = SemVer::parse(&candidate.tag_name) else {
            continue;
        };
        if !parsed.is_stable() {
            continue;
        }
        if parsed <= current {
            continue;
        }
        if best.as_ref().is_none_or(|(best_ver, _)| parsed > *best_ver) {
            best = Some((parsed, candidate));
        }
    }

    best.map(|(ver, candidate)| UpdateInfo {
        version: format!("{}.{}.{}", ver.major, ver.minor, ver.patch),
        tag_name: candidate.tag_name.clone(),
        release_url: candidate.html_url.clone(),
        published_at: candidate.published_at.clone(),
    })
}

/// Decide se a checagem automática ao iniciar deve rodar respeitando o cooldown.
pub fn should_auto_check_updates(
    enabled: bool,
    last_check_unix: Option<u64>,
    now_unix: u64,
) -> bool {
    if !enabled {
        return false;
    }
    match last_check_unix {
        None => true,
        Some(prev) => now_unix.saturating_sub(prev) >= UPDATE_CHECK_COOLDOWN_SECS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(tag: &str, prerelease: bool, draft: bool) -> ReleaseCandidate {
        ReleaseCandidate {
            tag_name: tag.to_owned(),
            html_url: format!("{GITHUB_RELEASES_PAGE_PREFIX}tag/{tag}"),
            draft,
            prerelease,
            published_at: Some("2026-11-20T20:00:00Z".to_owned()),
        }
    }

    #[test]
    fn semver_orders_prereleases_and_stable_versions_correctly() {
        let a9 = SemVer::parse("0.1.0-alpha.9").unwrap();
        let a10 = SemVer::parse("v0.1.0-alpha.10").unwrap();
        let b1 = SemVer::parse("v0.1.0-beta.1").unwrap();
        let stable = SemVer::parse("v0.1.0").unwrap();
        let next = SemVer::parse("v0.2.0").unwrap();

        assert!(a9 < a10);
        assert!(a10 < b1);
        assert!(b1 < stable);
        assert!(stable < next);
        assert!(!a9.is_stable());
        assert!(stable.is_stable());
    }

    #[test]
    fn ignores_alphas_betas_drafts_and_untrusted_urls() {
        let mut untrusted = candidate("v9.9.9", false, false);
        untrusted.html_url = "https://example.com/malicious.exe".to_owned();

        let candidates = vec![
            candidate("v0.1.0-alpha.10", true, false),
            candidate("v0.2.0-beta.1", false, false), // mesmo sem flag prerelease, tag beta é ignorada
            candidate("v1.0.0-rc.1", true, false),
            candidate("v2.0.0", false, true), // draft ignorado
            untrusted,
        ];

        assert!(select_newer_stable_release("0.1.0-alpha.9", &candidates).is_none());
    }

    #[test]
    fn selects_highest_newer_stable_release_from_alpha_or_stable() {
        let candidates = vec![
            candidate("v0.1.0-alpha.12", true, false),
            candidate("v0.1.0", false, false),
            candidate("v1.0.0", false, false),
            candidate("v1.2.0-beta.1", true, false),
            candidate("v1.1.0", false, false),
        ];

        let found_from_alpha =
            select_newer_stable_release("0.1.0-alpha.9", &candidates).expect("finds v1.1.0");
        assert_eq!(found_from_alpha.tag_name, "v1.1.0");
        assert_eq!(found_from_alpha.version, "1.1.0");

        let found_from_stable =
            select_newer_stable_release("1.0.0", &candidates).expect("finds v1.1.0");
        assert_eq!(found_from_stable.tag_name, "v1.1.0");

        assert!(select_newer_stable_release("1.1.0", &candidates).is_none());
        assert!(select_newer_stable_release("1.2.0", &candidates).is_none());
    }

    #[test]
    fn respects_auto_check_cooldown_and_disabled_flag() {
        assert!(!should_auto_check_updates(false, None, 10_000));
        assert!(should_auto_check_updates(true, None, 10_000));
        assert!(!should_auto_check_updates(
            true,
            Some(10_000),
            10_000 + 3600
        ));
        assert!(should_auto_check_updates(
            true,
            Some(10_000),
            10_000 + UPDATE_CHECK_COOLDOWN_SECS
        ));
    }
}
