//! Persistência de preferências do usuário em formato TOML.

use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use fm_core::{AppConfig, FsError};

use crate::map_io_err;

/// Resolve o caminho padrão de `config.toml` conforme convenções XDG (Linux) e AppData (Windows).
pub fn config_file_path() -> Option<PathBuf> {
    ProjectDirs::from("dev", "vagnarok", "vagnarok_file_manager")
        .map(|dirs| dirs.config_dir().join("config.toml"))
}

/// Carrega `AppConfig` do caminho padrão do sistema ou retorna padrões seguros.
pub fn load_config() -> AppConfig {
    let Some(path) = config_file_path() else {
        return AppConfig::default();
    };
    load_config_from(&path)
}

/// Persiste `AppConfig` no caminho padrão do sistema de forma atômica.
pub async fn save_config(config: AppConfig) -> Result<(), FsError> {
    let Some(path) = config_file_path() else {
        return Ok(());
    };
    tokio::task::spawn_blocking(move || save_config_to(&path, &config))
        .await
        .map_err(|err| FsError::Io {
            path: PathBuf::new(),
            message: format!("falha ao salvar configuração: {err}"),
        })?
}

/// Carrega configuração a partir de um arquivo TOML específico.
pub fn load_config_from(path: &Path) -> AppConfig {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return AppConfig::default();
    };
    toml::from_str::<AppConfig>(&raw).unwrap_or_default()
}

/// Salva configuração em um arquivo específico via escrita temporária + rename atômico.
pub fn save_config_to(path: &Path, config: &AppConfig) -> Result<(), FsError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| map_io_err(parent, e))?;
    }
    let serialized = toml::to_string_pretty(config).map_err(|err| FsError::Io {
        path: path.to_path_buf(),
        message: format!("erro ao serializar TOML: {err}"),
    })?;

    let tmp_path = path.with_extension("toml.tmp");
    std::fs::write(&tmp_path, serialized).map_err(|e| map_io_err(&tmp_path, e))?;
    std::fs::rename(&tmp_path, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        map_io_err(path, e)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fm_core::{Language, ThemePreset, ViewMode, WindowDecorationsMode};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn saves_and_loads_toml_config_roundtrip() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("vagnarok_cfg_test_{nanos}"));
        let path = dir.join("config.toml");

        let cfg = AppConfig {
            theme: ThemePreset::ObsidianFlow,
            theme_customized: true,
            language: Language::EsEs,
            language_customized: true,
            show_hero_card: false,
            show_inspector: true,
            sidebar_collapsed: true,
            sidebar_width: Some(312.0),
            window_decorations: WindowDecorationsMode::IntegratedCsd,
            default_view_mode: ViewMode::Columns,
            grid_group_folders: false,
            grid_card_width: 172.0,
            dirs_first: false,
            favorites: vec![PathBuf::from("/tmp/work")],
        };

        save_config_to(&path, &cfg).expect("save config ok");
        let loaded = load_config_from(&path);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(loaded.theme, ThemePreset::ObsidianFlow);
        assert_eq!(loaded.language, Language::EsEs);
        assert!(loaded.language_customized);
        assert!(!loaded.show_hero_card);
        assert!(loaded.show_inspector);
        assert!(loaded.sidebar_collapsed);
        assert_eq!(loaded.sidebar_width, Some(312.0));
        assert_eq!(
            loaded.window_decorations,
            WindowDecorationsMode::IntegratedCsd
        );
        assert_eq!(loaded.default_view_mode, ViewMode::Columns);
        assert!(!loaded.grid_group_folders);
        assert!(!loaded.dirs_first);
        assert_eq!(loaded.favorites, vec![PathBuf::from("/tmp/work")]);
    }

    #[test]
    fn deserializes_legacy_config_without_theme_customized() {
        let legacy_toml = r#"
theme = "AetherLight"
language = "PtBr"
show_hero_card = true
show_inspector = false
sidebar_collapsed = false
window_decorations = "NativeWm"
default_view_mode = "Grid"
favorites = []
"#;
        let parsed: AppConfig = toml::from_str(legacy_toml).expect("legacy config parses");
        assert_eq!(parsed.theme, ThemePreset::AetherLight);
        assert!(!parsed.theme_customized);
        assert_eq!(parsed.sidebar_width, None);
        assert!(parsed.grid_group_folders);
        assert!(parsed.dirs_first);
    }
}
