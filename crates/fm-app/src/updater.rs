//! Verificação assíncrona de novas releases estáveis no GitHub (100% Rust nativo via `ureq` + `rustls`).

use std::time::Duration;

use fm_core::{
    GITHUB_RELEASES_API_URL, ReleaseCandidate, UpdateInfo, is_trusted_release_url,
    select_newer_stable_release,
};
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateCheckState {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(UpdateInfo),
    Error(String),
}

#[derive(Debug, Deserialize)]
struct GithubReleaseDto {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    published_at: Option<String>,
}

/// Consulta a API pública do GitHub em uma thread de worker (`spawn_blocking`) e retorna
/// a versão **estável** mais recente caso seja estritamente superior a `current_version`.
pub async fn fetch_latest_stable_release(
    current_version: String,
    cancel: CancellationToken,
) -> Result<Option<UpdateInfo>, String> {
    if cancel.is_cancelled() {
        return Ok(None);
    }

    let handle = tokio::task::spawn_blocking(move || {
        if cancel.is_cancelled() {
            return Ok(None);
        }

        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(8)))
            .build()
            .into();

        let user_agent = format!("vagnarok-file-manager/{current_version}");
        let mut response = agent
            .get(GITHUB_RELEASES_API_URL)
            .header("User-Agent", &user_agent)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .call()
            .map_err(|err| err.to_string())?;

        if cancel.is_cancelled() {
            return Ok(None);
        }

        let releases: Vec<GithubReleaseDto> = response
            .body_mut()
            .read_json()
            .map_err(|err| err.to_string())?;

        let candidates: Vec<ReleaseCandidate> = releases
            .into_iter()
            .map(|r| ReleaseCandidate {
                tag_name: r.tag_name,
                html_url: r.html_url,
                draft: r.draft,
                prerelease: r.prerelease,
                published_at: r.published_at,
            })
            .collect();

        Ok(select_newer_stable_release(&current_version, &candidates))
    });

    handle
        .await
        .map_err(|err| format!("update task join error: {err}"))?
}

/// Abre a URL da release no navegador padrão somente após validar que pertence ao repositório oficial.
pub fn open_release_page_async(url: String) -> Result<(), String> {
    if !is_trusted_release_url(&url) {
        return Err("URL de release não confiável.".to_owned());
    }
    open::that_detached(&url).map_err(|err| err.to_string())
}
