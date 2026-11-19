# Changelog — Vagnarok File Manager

Todas as mudanças notáveis do projeto **Vagnarok File Manager** (gerenciador de arquivos desktop nativo em Rust com `iced` 0.14 + `wgpu` para Linux e Windows) estão documentadas neste arquivo.

> **Nota sobre a linha do tempo**: o repositório Git (ramo `main`, tag `v0.1.0-alpha.1`, com `.gitignore` configurando exclusão de `/target/`) foi ativado e populado cronologicamente com commits por onda de desenvolvimento a partir dos marcos registrados do projeto, da árvore do workspace (`0.1.0-alpha.1`, Rust `edition = "2024"`, licença `GPL-3.0-or-later`) e das verificações executadas entre **2026-11-14** e **2026-11-16 (UTC)**.

---

## Resumo Cronológico dos Marcos

| Data (UTC) | Marco / Fase | Principais Entregas | Testes Workspace |
| :--- | :--- | :--- | :--- |
| 2026-11-14 | **Spec Kit & Arquitetura** | `ARCHITECTURE.md`, `AGENTS.md`, `specs/00`–`05` | — |
| 2026-11-14 | **Fase P0** | Workspace com 7 crates, `fm-theme`, fontes embutidas, shell 3 painéis | 6 passando |
| 2026-11-15 | **Fase P1** | `LocalFs`, volumes Linux/Windows, `rayon`, viewports virtualizados | 15 passando |
| 2026-11-15 | **Fase P2** | Watcher `notify`, busca `nucleo`, Git `gix`, MIME/SHA-256, thumbnails SIMD + QuickLook | 23 passando |
| 2026-11-15 | **Fase P3** | `CopyEngine`, lixeira `trash`, TOML config, favoritos, Miller Columns | 26 passando |
| 2026-11-15 | **Fase P4** | `rustix` `getdents64`/`ioctl_ficlone`, `FindFirstFileExW`, SVG `resvg`, `redb` thumbs, terminal | 29 passando |
| 2026-11-15 | **Fase P5** | `created_at` (`statx`), tema do SO, menu de contexto, ordenação por coluna, DnD | 31 passando |
| 2026-11-15 | **Fase P6** | `CopyFileExW`/`ShellExecuteExW`, `longPathAware`, Mica/Acrylic, `ArchiveVfs`, `redb` telemetria, CI | 34 passando |
| 2026-11-15 | **Polimento Técnico** | Grid agrupado/unificado, tamanho recursivo Hero Card, QuickLook `512 KB`, restaurar/esvaziar lixeira, benchmarks | 36 passando |
| 2026-11-15 | **Pós-Polimento UX & Multi-Janela** | Duplo clique, anti-flicker, navegação 2D, sanitização `\n`, janela flutuante de Propriedades em 5 abas | 41 passando |
| 2026-11-16 | **Git, UX Sidebar & Zoom Ctrl+Scroll** | Ativação do Git (`main` + `.gitignore`), menu de contexto em favoritos/barra lateral, Propriedades via `stat_entry` e zoom `Ctrl + Scroll` no Grid | 42 passando |
| 2026-11-16 | **Internacionalização Multi-Idiomas (Fluent)** | 6 locales `.ftl` (`pt-BR`, `en-US`, `es-ES`, `fr-FR`, `de-DE`, `it-IT`), detecção do SO (`sys-locale`), pluralização CLDR, `LocaleFormatSpec` e seletores multi-idioma | 44 passando |
| 2026-11-16 | **Alpha 1 (`v0.1.0-alpha.1`), Sobre & GPLv3** | Aba e menu **Sobre**, versão `0.1.0-alpha.1`, troca de licença para `GPL-3.0-or-later` (`LICENSE`) e tag `v0.1.0-alpha.1` | 44 passando |
| 2026-11-16 | **Alpha 2 (`v0.1.0-alpha.2`), Empacotamento & CI/CD** | Dropdown de idiomas, pipeline `.deb`/Flatpak/Inno Setup + ZIP portátil, `winresource`, correções de testes no Windows e `README.md` | 44 passando (Linux) |
| 2026-11-16 | **Alpha 3 (`v0.1.0-alpha.3`), Polimento Windows & Lixeira** | Lixeira virtual no Windows (`trash://`) com restauração, remoção do CMD em release, correção de controles duplicados e limpeza do mockup `code.html` | 45 unitários |
| 2026-11-16 | **Alpha 4 (`v0.1.0-alpha.4`), Paridade de Atalhos & Janela no Windows** | Zoom `Ctrl + Scroll` no Windows, rastreamento de modificadores via `KeyPressed`/`KeyReleased`, alternância ao vivo de decorações da janela e `rustfmt` | 45 unitários |
| 2026-11-16 | **Alpha 5 (`v0.1.0-alpha.5`), Barra Lateral Estilo Nemo, Discos & Propriedades** | Cards compactos no Grid, `Propriedades` no fundo do diretório (conteúdo + espaço livre), `PageUp`/`PageDown`, `Ctrl+Q`, tooltips localizados, barra lateral estilo Nemo redimensionável, montar/desmontar discos e `Abrir como root` no Linux | 51 unitários |
| 2026-11-17 | **Alpha 6 (`v0.1.0-alpha.6`), Seleção em Caixa, Alinhamento Grid, Correções Flatpak & Ícones do Sistema** | Seleção em caixa (marquee), alinhamento do Grid esquerda→direita por padrão + alternância, preservação de scroll no menu de contexto, filtro de bind-mounts no Flatpak, divisor/scrollbar da barra lateral e ícones híbridos do sistema hospedeiro | 58 unitários |
| 2026-11-19 | **Alpha 7 (`v0.1.0-alpha.7`), Abrir Com, Renomeação em Lote, 7z/RAR & Compactação Minimizável** | Recurso **Abrir com...** + persistência por tipo, modal de **Propriedades** arrastável/redimensionável com bloqueio de cliques ao fundo, **Renomeação em Lote** visual (EXIF/ID3/Regex/Desfazer), extração (`.zip`, `.tar`, `.tar.gz`, `.7z`, `.rar`), janela **Compactar como...** minimizável em segundo plano, lixeira virtual unificada e permissão executável rápida | 92 unitários |

---

## [0.1.0-alpha.7] — 2026-11-19

### Adicionado
- **Abrir com... (`Open With`) e associação por tipo de arquivo**:
  - Descoberta nativa de aplicativos no Linux (`.desktop` / `mimeapps.list`) e Windows, execução personalizada com seletor de executável integrado, definição de aplicativo padrão por MIME/extensão e restauração para os padrões do sistema na aba **Abrir com** de Propriedades e no menu de contexto.
- **Renomeação em Lote (`Bulk Rename`) nativa e amigável**:
  - Acionada via `F2` / toolbar (`sel_count > 1`) ou menu de contexto (**Renomear em Lote...**), com cartões visuais marcáveis (**Procurar e Substituir** com escopo por início/fim de nome ou palavra, **Remover**, **Inserir**, **Maiúsculas/Minúsculas**, **Numeração Sequencial**, **Mudar Extensão** e **Regex opcional** recolhido em opções avançadas).
  - Preview em tempo real **virtualizado**, leitura assíncrona *lazy* de metadados **EXIF** e **ID3**, detecção de colisões (inclusive case-insensitive), planejador topológico em 2 fases para ciclos de troca e ação **Desfazer última renomeação**.
- **Extração de arquivos (`.zip`, `.tar`, `.tar.gz`/`.tgz`, `.7z`, `.rar`) e Compactação ("Compactar como...")**:
  - Opções no menu de contexto para **Extrair aqui**, **Extrair em subpasta (`<nome>/`)** — incluindo descompactação de `.rar` (RAR4/RAR5 e volumes `.partN.rar`) em 100% Rust puro com proteção anti-*Zip-Slip* — e atalho **Adicionar ao arquivo `.7z`**.
  - Janela flutuante **Compactar como...** com dropdown de formato (`.7z`, `.zip`, `.tar`, `.tar.gz`) e dropdown de nível de compressão (`Armazenar`, `Mínimo`, `Normal`, `Máximo`), **botão Minimizar** para liberar toda a janela principal e continuar navegando ou abrindo novas abas enquanto o trabalho executa em segundo plano, com pílula persistente de progresso no rodapé (`Restaurar` / `Cancelar`) e atualização automática das abas afetadas.
- **Permissão rápida de execução (POSIX)**:
  - Opção **Autorizar execução como programa** diretamente no menu de contexto e na aba Permissões de Propriedades.

### Melhorado e Corrigido
- **Modais de Propriedades / Renomeação / Compactação**: overlays com `opaque(...)` bloqueando cliques na janela principal atrás, barra de título arrastável dentro da viewport, redimensionamento pelo canto em Propriedades e Renomeação em Lote, e otimização de consumo de memória com cache unificado de ícones em lote.
- **Lixeira virtual unificada e Recycle Bin (Windows)**: unificado comportamento da `trash://` no Linux/Flatpak e corrigidas ações de restauração e exclusão na Lixeira do Windows.

## [0.1.0-alpha.6] — 2026-11-17

### Adicionado
- **Seleção em caixa (*marquee / rubber-band*) no canvas**:
  - Clicar em área vazia e arrastar nos modos **Grade**, **Lista** e **Colunas** seleciona todos os itens interceptados pelo retângulo translúcido, com suporte a seleção aditiva (`Ctrl`/`Shift`) e limpeza ao clicar em espaço vazio.
- **Alinhamento do Grid (Esquerda → Direita ou Centralizado)**:
  - Pastas e arquivos na visualização em Grade agora iniciam por padrão da esquerda para direita (`grid_center_items = false`), com botão rápido na barra do canvas e seletor em **Preferências → Geral** para alternar com o modo centralizado.
- **Ícones híbridos do sistema hospedeiro (Linux / Flatpak / Windows)**:
  - Suporte opcional e 100% em Rust nativo para exibir ícones do tema do sistema (Linux GTK/KDE/Cinnamon via `index.theme`, `globs2`, `gsettings` e `/run/host/share/icons` no Flatpak; Windows via `SHGetFileInfoW` com `SHGFI_USEFILEATTRIBUTES` e `GetDIBits` sem travar arquivos), mantendo botões da UI em `lucide-icons`.
  - Inicia por padrão com os **Ícones padrão do aplicativo** (`IconThemeMode::Bundled`) e traz botão rápido na barra de ferramentas do canvas para alternar em tempo real.

### Corrigido
- **Scroll resetado ao abrir menu de contexto**: mantida a raiz `Stack` estável em `App::view` para que abrir/fechar menus de contexto e modais não descarte o estado de rolagem no modo Lista, Grade, Colunas ou Barra Lateral.
- **Listagem de sistema em Unidades no Flatpak**: filtrados bind-mounts internos do `bubblewrap` (`/usr`, `/app`, extensões GL/codecs, `/etc/*`, `/run/host/*`, `*.usr-is-merged`) e montagens de arquivo único do `/proc/mounts`, além de medir o espaço livre real de `Root (/)` no host via `/run/host/*` dentro do Flatpak.
- **Divisor e scrollbar da barra lateral**: reservado espaçamento dedicado para a scrollbar vertical da sidebar (sem cobrir botões de montar/desmontar), aplicado `.clip(true)`, ampliada a área de captura do divisor (`10px`) com destaque no hover/arraste e elevado o limite máximo para `560px`.

## [0.1.0-alpha.5] — 2026-11-16

### Adicionado
- **Grid compacto e Propriedades do diretório**:
  - Cards do Grid compactados para deixar espaço livre clicável atrás dos itens em pastas cheias.
  - Opção **Propriedades** no menu de contexto do plano de fundo do diretório, além de exibição de **Conteúdo** (contagem recursiva e cache `redb`) e **Espaço livre** do volume.
- **Atalhos e CLI**:
  - `PageUp` / `PageDown` para rolagem rápida (85% da altura visível) no canvas.
  - `Ctrl+Q` para fechar janelas modais (como Propriedades) ou encerrar o aplicativo.
  - Suporte a caminho inicial ou URI `file://` passado por linha de comando ao abrir o aplicativo.
- **Tooltips estilizados e localizados**:
  - Tooltips integrados ao tema (`ObsidianFlow` / `AetherLight`) em botões de cabeçalho, barra de ferramentas do canvas e barra lateral nas 6 traduções (`pt-BR`, `en-US`, `es-ES`, `fr-FR`, `de-DE`, `it-IT`).
- **Barra lateral estilo Nemo e gerenciamento de unidades**:
  - Seção de Acesso Rápido com subpastas expansíveis de `Início` e pastas `Música`, `Vídeos`, `Público` e `Modelos`.
  - Divisor vertical arrastável entre a barra lateral e o conteúdo, com persistência da largura customizada em `AppConfig` (`sidebar_width`), ajuste automático mínimo ao nome dos discos e recorte de texto (`.clip(true)`) sem sobreposição.
  - Descoberta de partições/unidades (`lsblk` + `/proc/mounts` no Linux) e ações de montar/desmontar discos (`udisksctl`/`gio` no Linux e ejeção de mídia removível no Windows).
- **Abrir como root (Linux)**:
  - Opção **Abrir como root** no menu de contexto de diretórios via `pkexec` (restrita exclusivamente ao Linux via `fm_fs::supports_open_as_root()`) e selo visual quando rodando em sessão elevada.

## [0.1.0-alpha.4] — 2026-11-16

### Correções no Windows & Paridade com Linux

* **Zoom `Ctrl + Scroll` no Grid e rastreamento de modificadores** (`crates/fm-app/src/app.rs`, `crates/fm-app/src/views/canvas.rs`):
  * Atualização de `current_modifiers` também em `KeyPressed` e `KeyReleased` (além de `ModifiersChanged`), e limpeza de modificadores em `WindowUnfocused` restrita à janela principal (`main_window_id`).
  * Adicionado estado interno `CtrlWheelState` no widget `CtrlWheelZoom` para capturar `Ctrl + WheelScrolled` diretamente no Windows, garantindo funcionamento tanto do zoom no Grid quanto da seleção múltipla (`Ctrl+Click` / `Shift+Click`).
* **Alternância ao vivo das decorações da janela** (`crates/fm-app/src/app.rs`):
  * `Message::ToggleWindowDecorations` agora executa `window::toggle_decorations(self.main_window_id)` também no Windows, aplicando a troca entre barra nativa e CSD integrado sem reiniciar o aplicativo.
* **Janela de Propriedades no Windows sem distorção de swapchain** (`crates/fm-app/src/{app.rs,views/modals.rs}`):
  * No Windows, **Propriedades** agora abre como modal centralizado sobre a janela principal (`properties_modal_overlay`), evitando o bug multi-janela do `iced`/`wgpu` DX12 que esticava o viewport principal para `720×520` após abrir/fechar a janela flutuante.
* **Formatação** (`crates/fm-app/src/views/header.rs`, `crates/fm-fs/src/lib.rs`):
  * Ajustada formatação conforme `cargo fmt --all -- --check`.

---

## [0.1.0-alpha.3] — 2026-11-16

### Windows UX & Lixeira (Recycle Bin)

* **Lixeira virtual no Windows** (`crates/fm-core/src/{domain,lib}.rs`, `crates/fm-fs/src/lib.rs`, `crates/fm-app/src/{app,views/header}.rs`):
  * Adicionados `VIRTUAL_TRASH_PATH` (`"trash://"`) e `is_virtual_trash_path` em `fm-core`.
  * `resolve_trash_dir` agora expõe `trash://` no Windows, exibindo o ícone **Lixeira** na barra lateral esquerda.
  * `LocalFs::list_dir` lista os itens da Lixeira do Windows via `trash::os_limited::list()`, `restore_trash_items` restaura por `item.id` exato e `delete_permanent` purga itens da Lixeira via `trash::os_limited::purge_all`.
  * Breadcrumb, título da aba e botão **Subir** adaptados para a raiz virtual `trash://`.
* **Sem janela de console em `release` no Windows** (`crates/fm-app/src/main.rs`):
  * Adicionado `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` e ajustado o filtro padrão do `tracing_subscriber` para reduzir ruído de `wgpu`/`naga`.
* **Controles de janela sem duplicação** (`crates/fm-app/src/views/header.rs`):
  * Removido `cfg!(target_os = "windows") ||` de `show_csd` para que os botões integrados sejam exibidos apenas quando `WindowDecorationsMode::IntegratedCsd` estiver ativo.

### Manutenção

* **Limpeza de mockup inicial** (`code.html`, `ARCHITECTURE.md`, `specs/00-spec-index.md`, `specs/01-mockup-reconciliation.md`):
  * Removido o arquivo `code.html` e convertidas suas referências em markdown para texto formatado.

---

## [0.1.0-alpha.2] — 2026-11-16

### Empacotamento, Distribuição Multiplataforma & CI/CD

* **Identidade, ícones e recursos nativos** (`assets/icons/*`, `crates/fm-app/{Cargo.toml,build.rs,vagnarok.rc}`, `Cargo.toml`):
  * Definido o binário instalado como `vagnarok-file-manager` (`[[bin]]` em `crates/fm-app/Cargo.toml`) e o App ID reverso `dev.vagnarok.FileManager`.
  * Gerado ícone vetorial [`dev.vagnarok.FileManager.svg`](assets/icons/dev.vagnarok.FileManager.svg) seguindo a paleta `ObsidianFlow`/`AetherLight`, além de PNGs (`128`/`256`/`512`) e [`vagnarok.ico`](assets/icons/vagnarok.ico) multi-resolução.
  * Integrado `winresource` em `build.rs` para embutir ícone, manifesto `longPathAware` / `PerMonitorV2` e metadados do executável no Windows (MSVC e MinGW).
* **Pacotes Linux (`.deb` e Flatpak)** (`crates/fm-app/Cargo.toml`, `packaging/linux/*`, `packaging/flatpak/*`, `crates/fm-fs/src/linux.rs`):
  * Configurado `[package.metadata.deb]` para geração de `.deb` via `cargo-deb`.
  * Adicionados [`dev.vagnarok.FileManager.desktop`](packaging/linux/dev.vagnarok.FileManager.desktop) e [`dev.vagnarok.FileManager.metainfo.xml`](packaging/linux/dev.vagnarok.FileManager.metainfo.xml) validados com `desktop-file-validate` e `appstreamcli validate --no-net`.
  * Criado manifesto Flatpak [`dev.vagnarok.FileManager.yml`](packaging/flatpak/dev.vagnarok.FileManager.yml) (`org.freedesktop.Platform//24.08`, build offline vendored) e suporte a `flatpak-spawn --host` em `open_terminal_at` quando `FLATPAK_ID` estiver ativo.
* **Instalador Windows & Release automatizada** (`packaging/windows/vagnarok.iss`, `.github/workflows/release.yml`, `scripts/cut-release.sh`, `.gitignore`):
  * Criado script Inno Setup [`vagnarok.iss`](packaging/windows/vagnarok.iss) com 6 idiomas, licença GPLv3, ícone e integrações opcionais.
  * Criado workflow [`release.yml`](.github/workflows/release.yml) disparado por tags `v*.*.*` ou `workflow_dispatch`, validando `Cargo.toml` vs. tag, rodando testes, gerando `.deb` (`ubuntu-22.04`), `.flatpak` (`ubuntu-24.04`), setup `.exe` + ZIP portátil (`windows-latest`), `SHA256SUMS.txt` e publicando GitHub Release.
  * Adicionado helper local executável [`scripts/cut-release.sh`](scripts/cut-release.sh) para bump SemVer, checagem, commit e criação de tag anotada.

### Interface & UX

* **Dropdown (`pick_list`) de idiomas em Preferências** (`crates/fm-app/src/views/modals.rs`, `crates/fm-core/{src/i18n.rs,locales/*.ftl}`):
  * Substituída a grade de botões de idiomas na aba **Geral** por um dropdown compacto com suporte à opção **Sistema (Auto)** e escalável para novos idiomas.

### Correções no Windows & Documentação

* **Compatibilidade e testes em `fm-fs` no Windows** (`crates/fm-fs/src/{windows.rs,lib.rs}`, `Cargo.toml`, `crates/*/Cargo.toml`):
  * Adicionado `win_err_to_io` para converter códigos Win32 encapsulados em `HRESULT` (`0x8007xxxx`) preservando `ErrorKind::NotFound` / `PermissionDenied`, além do ajuste do teste de item oculto com `FILE_ATTRIBUTE_HIDDEN` no Windows.
  * Eliminados avisos de manifesto Cargo (`rust-version.workspace = true` nos 7 crates e limpeza de dependências não usadas por alvo).
* **Documentação & Captura principal** (`README.md`, `assets/screenshots/screenhome.png`, `AGENTS.md`, `specs/00-spec-index.md`):
  * Criado `README.md` completo com captura real [screenhome.png](assets/screenshots/screenhome.png) em `assets/screenshots/` e adicionada política obrigatória de commit/aviso no Spec Kit.

---

## [0.1.0-alpha.1] — 2026-11-16

### Aba "Sobre", Versionamento `v0.1.0-alpha.1` & Licença GPLv3

* **Aba e atalho contextual "Sobre"** (`crates/fm-app/src/{app.rs,views/sidebar.rs,views/modals.rs}`, `crates/fm-core/{src/i18n.rs,locales/*.ftl}`):
  * Refatorado o modal de **Preferências** com duas abas (`modal_tab_button` compartilhado com Propriedades): **Geral** (`prefs-tab-general`) e **Sobre** (`about-tab`, `about_tab_view`).
  * Adicionado clique direito no botão **Preferências** da barra lateral (`ContextMenuKind::PreferencesMenu` via `Message::OpenPreferencesContextMenu` e `Message::OpenAbout`), permitindo abrir diretamente **Preferências** ou **Sobre**.
  * Aba **Sobre** exibe versão dinâmica (`concat!("v", env!("CARGO_PKG_VERSION"))` → `v0.1.0-alpha.1`), estágio *Alpha — Prévia de Desenvolvimento*, desenvolvedor (`em desenvolvimento` traduzido nos 6 idiomas), licença `GNU GPL v3.0 (GPL-3.0-or-later)`, motor `Rust · Iced 0.14 + wgpu` e botão para copiar resumo do ambiente (`about-copy-info`).
* **Versionamento & migração para GNU GPL v3.0** (`Cargo.toml`, `Cargo.lock`, `LICENSE`, `specs/05-tech-stack-prototype.md`):
  * `[workspace.package]` atualizado para `version = "0.1.0-alpha.1"` e `license = "GPL-3.0-or-later"`.
  * Adicionado o texto oficial integral da **GNU General Public License v3.0** em [LICENSE](LICENSE) na raiz do projeto e atualizada a referência em `specs/05-tech-stack-prototype.md`.

### Internacionalização Multi-Idiomas (`Project Fluent`)

* **Motor Fluent & 6 locales embutidos** (`Cargo.toml`, `crates/fm-core/locales/*.ftl`, `crates/fm-core/src/i18n.rs`):
  * Integrados `fluent-bundle`, `unic-langid`, `sys-locale` e `fluent-syntax` (dev) sem I/O em runtime (`include_str!`) e com cache estático `OnceLock` em `Strings` (`Copy + Send + Sync`, `set_use_isolating(false)` e fallback automático para `en-US`).
  * Adicionados catálogos completos para **Português (BR)**, **English (US)**, **Español**, **Français**, **Deutsch** e **Italiano**, com regras plurais CLDR (`format_items_count`, `format_folders_count`, `format_files_count`, `format_selected_multi`), formatação de bytes exatos e descrição de tipos (`describe_entry_kind`).
  * Testes automatizados em `fm-core` (`all_locales_match_canonical_keys_and_variables` e `resolves_locale_tags_and_plural_forms`) validando paridade de 100% das chaves e variáveis entre todos os `.ftl`.
* **Modelo de idioma, detecção do SO & formatação regional** (`crates/fm-core/src/{state,domain,lib}.rs`, `crates/fm-fs/src/config.rs`):
  * Expandido `Language` (`ALL`, `bcp47()`, `short_code()`, `native_name()`, `from_locale_tag()`, `detect_system()`) e adicionado `language_customized: bool` em `AppConfig`.
  * Removido o acoplamento `pt_br: bool` em favor de `LocaleFormatSpec`, `format_grouped_u64(value, thousands_sep)` e `local_timestamp_kind()`.
* **Seletores de idioma & limpeza de literais na UI** (`crates/fm-app/src/app.rs`, `views/{header,modals,canvas,inspector,footer}.rs`):
  * Clique no indicador de idioma no topo (`PT`/`EN`/`ES`/`FR`/`DE`/`IT`) abre `ContextMenuKind::LanguageMenu` com opção **Sistema (Auto)** (`Message::UseSystemLanguage`) e seleção direta (`Message::SetLanguage`); **Preferências** exibe pílulas para todos os idiomas e modo automático, propagando imediatamente inclusive para janelas flutuantes de Propriedades.
  * Removidos `strings.close == "Fechar"` e textos hardcoded restantes em ações, diálogos, Inspetor, rodapé e canvas.

### Repositório Git, UX da Barra Lateral & Zoom `Ctrl + Scroll`

* **Ativação e mapeamento histórico do Git** (`.gitignore`, ramo `main`):
  * Inicializado o repositório Git em `main` com [.gitignore](.gitignore) ignorando `/target/` e temporários, mapeando cronologicamente todas as ondas de desenvolvimento do baseline inicial (`9cedaef`) até o estado atual.
* **Menu de contexto nos favoritos e barra lateral** (`crates/fm-app/src/views/sidebar.rs`, `modals.rs`, `app.rs`, `crates/fm-core/src/i18n.rs`):
  * Adicionado clique direito em pastas favoritadas (e nos atalhos de *Acesso Rápido*, *Unidades* e *Lixeira*) via `Message::OpenSidebarContextMenu` → `ContextMenuKind::SidebarPlace`.
  * Opções contextuais: **Abrir**, **Abrir em nova aba**, **Copiar caminho**, **Propriedades** e **Remover dos favoritos** (`Message::RemoveFavorite`, chave i18n `remove_favorite`, persistido em `config.toml`), além de sanitização `sanitize_single_line` nos rótulos de favoritos.
  * **Corrigido** (`crates/fm-app/src/app.rs`): `Message::OpenProperties` agora consulta `fm_fs::stat_entry` (`Message::PropertiesEntryStatLoaded` + helper `open_properties_window`) quando o caminho não está listado na aba ativa, garantindo abertura da janela flutuante centralizada **Informações** também pela barra lateral, e `PropertiesApplied` sincroniza `config.favorites` caso uma pasta favoritada seja renomeada.
* **Zoom no modo Grade com `Ctrl + Scroll`** (`crates/fm-app/src/views/canvas.rs`, `app.rs`):
  * Adicionado wrapper `CtrlWheelZoom` sobre o painel central que intercepta `mouse::Event::WheelScrolled` quando `Ctrl` está pressionado em `ViewMode::Grid` (sem menus/modais abertos), evitando rolagem simultânea da grade.
  * `Message::CtrlWheelZoom` converte deltas de mouse (`Lines`) e trackpad (`Pixels`) via `wheel_zoom_delta` (com teste unitário `wheel_zoom_delta_maps_lines_and_pixels`), atualiza miniaturas visíveis (também em `StepGridCardWidth`/`ResetGridCardWidth`) e persiste a largura em `config.toml` após `350 ms` de pausa (`Message::FlushZoomConfig`) ou ao soltar `Ctrl` (`ModifiersChanged` / `window::Event::Unfocused`).

---

## [0.1.0-dev] — 2026-11-15

### Pós-Polimento: UX de Navegação, Sanitização de Texto & Janela Flutuante de Propriedades

#### Centralização da Janela Flutuante de Propriedades
* **Alterado** (`crates/fm-app/src/app.rs`): janelas flutuantes de **Propriedades** abertas pelo menu de contexto agora iniciam centralizadas na tela via `window::Position::Centered` (tamanho `720x520`), em vez de coordenadas em cascata no canto superior esquerdo.

#### Janelas Multi-Janela Flutuantes e Arrastáveis de Propriedades
* **Alterado** (`crates/fm-app/src/main.rs`, `crates/fm-app/src/app.rs`, `crates/fm-app/src/views/modals.rs`):
  * Migrado o loop principal de `iced::application` para `iced::daemon`, permitindo gerenciar a janela principal (`main_window_id`) e múltiplas janelas independentes de propriedades (`properties_windows: HashMap<window::Id, PropertiesWindowState>`).
  * Adicionada `properties_window_view` conectada ao `window::Id` correspondente, com arraste tanto pela barra nativa do sistema quanto pelo cabeçalho interno (`mouse_area` + `Message::DragPropertiesWindow` -> `window::drag`).
  * Permitida comparação lado a lado com múltiplas janelas abertas simultaneamente, cancelamento independente de telemetria por janela e foco automático (`window::gain_focus`) ao reabrir um item já aberto.

#### Redesign da Janela de Propriedades (`Informações`)
* **Adicionado / Alterado**:
  * **Interface com 5 abas** (`crates/fm-app/src/views/modals.rs`): **`Geral`**, **`Segurança`**, **`Histórico`**, **`Metadados`** e **`Permissões`**, mantendo integralmente a paleta `ThemePalette` (`ObsidianFlow` Dark e `AetherLight` Light) sem copiar cores do mockup de referência.
  * **Aba `Geral`**: hero card com ícone (`52x52`) e renomeação direta do item (bloqueada dentro de arquivos compactados), grade em 2 colunas (`Tipo`, `Sistema / MIME` com abertura no app padrão, `Localização` com cópia rápida e `Tamanho` com bytes agrupados e barra proporcional), linha do tempo (`Criado`, `Modificado`, `Último acesso`), switches de `Propriedades Rápidas` (`Somente leitura`, `Oculto`) e botão `Configurações Avançadas` apontando para `Permissões`.
  * **Abas secundárias**: `Segurança` (SHA-256 completo + cópia, atributos e alvo de symlink), `Histórico` (datas + status Git), `Metadados` (nome, caminho, extensão, MIME, tamanho, `EntryId`), e `Permissões` (matriz interativa POSIX R/W/X em Unix e toggles de atributos + flags hex no Windows).
  * **Domínio & Formatação** (`Cargo.toml`, `crates/fm-core/src/domain.rs`, `crates/fm-core/src/i18n.rs`): adicionada dependência `chrono`, `EntryExtraInfo`, `format_grouped_u64`, `format_timestamp_short` (`Hoje, HH:MM` / `Ontem, HH:MM` / `DD Mmm YYYY, HH:MM` no fuso local) e chaves i18n em PT-BR e EN-US.
  * **Backend de Propriedades** (`crates/fm-fs/src/lib.rs`, `linux.rs`, `windows.rs`, `fallback.rs`): implementados `read_entry_extra_info`, `apply_entry_properties` (`PropertyPatch`) e `stat_entry` assíncronos, com `PropertiesTab` e controle `is_dirty` em `crates/fm-app/src/app.rs`.

#### Correção de Quebra de Linha em Nomes com Caracteres de Controle (`\n`)
* **Corrigido** (`crates/fm-app/src/views/mod.rs`, `header.rs`, `canvas.rs`, `footer.rs`, `inspector.rs`, `modals.rs`, `app.rs`):
  * Adicionado `sanitize_single_line(&str)` (com teste unitário `sanitizes_embedded_newlines_into_single_spaces`) exclusivamente na camada de exibição para converter `\n`, `\r`, `\t` e demais caracteres de controle em espaço simples, preservando `PathBuf` / `OsString` reais no disco.
  * Aplicado `text::Wrapping::None` e `.width(Length::Fill)` nos containers de linha (`sticky_header` e `row_content` da `detail_table_view`, `hero_card`, `unified_breadcrumb`, abas e `columns_view`), ajustando a largura do campo de busca no topo de `360px` para `310px`.

#### Navegação 2D por Setas, Overflow em Linha Única e Grade Responsiva
* **Adicionado / Corrigido**:
  * **Navegação 2D e auto-scroll** (`crates/fm-app/src/views/virtualized.rs`, `crates/fm-app/src/app.rs`): adicionados `step_grid_selection`, `grid_band_y_span`, `navigate_selection_arrow(dx, dy)` e `scroll_entry_into_view` para movimentação vertical/horizontal correta em **Grade**, **Lista** e **Colunas** (`→` entra na pasta, `←` sobe nível).
  * **Recorte em linha única e ajuste de colunas** (`crates/fm-app/src/views/canvas.rs`): criado `single_line_label`, ampliado peso da coluna `Nome` em `table_col_weights` (`[48.0, 11.0, 9.0, 17.0, 15.0]`) e adicionado divisor arrastável na vista **Colunas** (`StartColumnsPanelResize` / `columns_panel_width` entre `200px` e `480px`).
  * **Grade responsiva e redimensionável** (`crates/fm-core/src/state.rs`, `virtualized.rs`, `canvas.rs`): `AppConfig::grid_card_width` (padrão `140.0`, faixa `104px`–`260px` com `clamp_grid_card_width`, `grid_card_height`, `grid_columns_for_width`), centralização simétrica dos blocos da grade, alça `StartGridResize` no canto inferior direito dos cards, controles `-` / `140px` / `+` + toggle de agrupamento na barra da aba e atalhos `Ctrl++`, `Ctrl+-`, `Ctrl+0`.

#### Correções de Duplo Clique, Flickering e Propriedades In-App
* **Corrigido**:
  * **Duplo clique** (`crates/fm-app/src/views/canvas.rs`, `app.rs`): removido `.on_double_click(...)` capturado pelo `button` interno; `Message::SelectEntry` detecta dois cliques consecutivos no mesmo item dentro de `450 ms` (sem `Ctrl`/`Shift`) e aciona `Message::ActivateEntry`.
  * **Propriedades in-app** (`crates/fm-app/src/app.rs`, `modals.rs`, `fm-core/src/i18n.rs`): desacoplado do D-Bus (`ShowItemProperties`, que abria o Nemo no Linux), passando a exibir diálogo interno com cálculo ao vivo e cancelável.
  * **Anti-flicker em diretórios ativos** (`crates/fm-index/src/watcher.rs`, `app.rs`, `canvas.rs`): debounce do watcher elevado para `250 ms` com `WatchEvent::has_visible_change` ignorando arquivos ocultos `.` quando `show_hidden` está desativado, throttle mínimo de `600 ms` em `DirectoryChangedOnDisk`, preservação de `child_count` conhecido entre reloads em `DirectoryLoaded`, e altura padronizada (`height(22)`) no rótulo do card da grade.

---

### Polimento Técnico — Grid, Tamanho Recursivo, QuickLook `512 KB` e Lixeira Completa (2026-11-15)

* **Grid agrupado vs. unificado e `dirs_first` configurável** (`crates/fm-core/src/state.rs`, `views/virtualized.rs`, `views/canvas.rs`, `views/modals.rs`, `app.rs`):
  * Adicionados `grid_group_folders` e `dirs_first` (padrão `true`) em `AppConfig`, `build_unified_grid_bands`, toggles em Preferências e unificação de busca + filtro de pastas via `DIR_FILTER_SENTINEL`.
* **Tamanho recursivo assíncrono com cache `redb` no Hero Card** (`crates/fm-core/src/domain.rs`, `crates/fm-telemetry/src/meta_cache.rs`, `stats.rs`, `canvas.rs`, `app.rs`):
  * Adicionados `RecursiveSize` em `DirectoryTelemetry`, tabela `dir_size_v1` no `DiskMetadataCache` e worker cancelável `compute_folder_size_cached` (limite de `25.000` entradas, profundidade `32`, timeout `1,5 s`, sem seguir symlinks ou pseudo-filesystems).
* **QuickLook de código ampliado** (`crates/fm-app/src/app.rs`): limite de leitura textual estendido de `64 * 1024` para `512 * 1024` bytes em arquivos locais e dentro de compactados.
* **Ações nativas de Lixeira** (`crates/fm-core/src/vfs.rs`, `crates/fm-fs/src/lib.rs`, `canvas.rs`, `modals.rs`):
  * Implementados `VfsProvider::restore_from_trash` e `VfsProvider::empty_trash` via `trash::os_limited` (Linux e Windows), botões **Restaurar** e **Esvaziar lixeira**, ação contextual **Restaurar** e diálogo `ActiveDialog::ConfirmEmptyTrash`.
* **Benchmarks aferidos** (`cargo bench -p fm-fs --bench vfs_bench -- --sample-size 10 --measurement-time 1`):
  * `localfs_list_dir_2000_entries`: `4.3052 ms` (`[4.2230 ms, 4.3916 ms]`, dentro da meta `< 5 ms`);
  * `tab_recompute_sort_10000_rayon`: `4.7902 ms`;
  * `nucleo_fuzzy_rank_10000`: `1.9078 ms`.

---

### Fase P6 — Paridade Windows Nativa, `ArchiveVfs`, Cache `redb` & CI (2026-11-15)

* **Paridade nativa Windows & propriedades de sistema** (`crates/fm-fs/src/windows.rs`, `linux.rs`, `copy_engine.rs`, `crates/fm-core/src/vfs.rs`, `crates/fm-app/build.rs`, `vagnarok.manifest`, `vagnarok.rc`, `app.rs`):
  * Cópia nativa com progresso e cancelamento via `CopyFileExW` e suporte a caminhos longos `\\?\` (`to_long_wide`).
  * Abertura e propriedades nativas com `ShellExecuteExW` no Windows e `org.freedesktop.FileManager1.ShowItemProperties` no Linux (`VfsProvider::open_properties`).
  * Manifesto `longPathAware` compilado via `build.rs` (compatível com MSVC e `x86_64-w64-mingw32-windres`).
  * Efeito Mica/Acrylic no Windows via `apply_window_vibrancy` (`window-vibrancy`).
* **Navegação em arquivos compactados (`ArchiveVfs`)** (`crates/fm-fs/src/archive.rs`, `lib.rs`, `crates/fm-preview/src/thumbs.rs`, `app.rs`):
  * Navegação read-only em `.zip`, `.tar`, `.tar.gz` e `.tgz`, extração sob demanda (`VfsProvider::read_bytes`), geração de miniaturas em memória (`decode_thumbnail_from_bytes`) e QuickLook para itens internos.
* **Cache persistente de telemetria em `redb`** (`crates/fm-telemetry/src/meta_cache.rs`, `stats.rs`, `app.rs`):
  * `DiskMetadataCache` (`telemetry_v1.redb`) persistindo MIME, SHA-256 (prefixo e completo) e contagem rasa de pastas (`count_direct_children_cached`).
* **Refinamentos de viewport** (`crates/fm-app/src/views/canvas.rs`, `modals.rs`, `app.rs`):
  * Colunas redimensionáveis na tabela (`table_col_weights`, `StartColumnResize` / `EndColumnResize`);
  * Miller Columns multinível com rolagem horizontal e carregamento de ancestrais (`maybe_load_column_ancestors`);
  * Filtro por tipo/extensão (`TabState.extension_filter` e `ContextMenuKind::FilterMenu`).
* **CI, Tracing & Benchmarks** (`.github/workflows/ci.yml`, `crates/fm-fs/benches/vfs_bench.rs`):
  * Pipeline GitHub Actions em `ubuntu-latest` e `windows-latest` (`fmt`, `clippy -- -D warnings`, `test --workspace`, `bench --no-run`), benchmark Criterion e spans `tracing::debug!` em `LocalFs::list_dir`.

---

### Fase P5 — Menu de Contexto, Ordenação por Coluna, Tema do SO, Atalhos & Drag & Drop (2026-11-15)

* **Data de criação nativa (`created_at`)** (`crates/fm-core/src/domain.rs`, `crates/fm-fs/src/linux.rs`, `windows.rs`, `lib.rs`, `inspector.rs`):
  * Captura nativa de `created_at` via `statx` (`BTIME`, com fallback para `statat`) no Linux e `ftCreationTime` no Windows, exibida no Inspetor.
* **Tema do sistema e barra de janela dinâmica** (`crates/fm-core/src/state.rs`, `crates/fm-app/src/app.rs`):
  * Adicionado `AppConfig::theme_customized`; sincronização com `system::theme()` e `system::theme_changes()` quando não customizado, e alternância dinâmica de decorações com `window::toggle_decorations` fora do Windows.
* **Menu de contexto, `+ Novo` e ordenação na tabela** (`crates/fm-app/src/views/modals.rs`, `canvas.rs`):
  * Popover `context_menu_overlay` com ações para itens e área vazia (incluindo cópia de caminho via `iced::clipboard::write`), dropdown no botão `+ Novo` e cabeçalhos clicáveis na `DetailTable` (`Message::SortByField`).
* **Atalhos, navegação rápida e Drag & Drop externo** (`crates/fm-core/src/state.rs`, `crates/fm-app/src/views/header.rs`, `app.rs`):
  * `Ctrl+A` (`SelectionState::select_all`), `Ctrl+Tab` / `Ctrl+Shift+Tab`, clique do meio para fechar abas, `Alt+Left` / `Alt+Right` / `Alt+Up` / `Backspace`, navegação `↑`/`↓` com QuickLook aberto, autocompletar `Tab` em `Ctrl+L`, e recepção em lote de `window::Event::FileDropped` (`FlushDroppedFiles`) via `CopyEngine`.

---

### Fase P4 — Hot Paths Nativos, SVG, Cache em Disco & Polimento de UX (2026-11-15)

* **Hot paths nativos & terminal** (`crates/fm-fs/src/linux.rs`, `windows.rs`, `copy_engine.rs`, `lib.rs`, `crates/fm-core/src/vfs.rs`):
  * Linux (`rustix`): `scan_directory_native` (`open(O_DIRECTORY)` + `RawDir`/`getdents64` + `statat`), clone COW `try_ficlone` (`ioctl_ficlone`) no `CopyEngine` e `open_terminal_at`.
  * Windows (`windows` crate): `scan_directory_native` em lote (`FindFirstFileExW` com `FindExInfoBasic` + `FIND_FIRST_EX_LARGE_FETCH`, `FindNextFileW` e RAII `FindClose`) e `open_terminal_at` (`wt.exe` / `cmd.exe`), validado com cross-check `x86_64-pc-windows-gnu`.
* **SVG & Cache persistente de miniaturas** (`crates/fm-preview/src/thumbs.rs`, `disk_cache.rs`):
  * Rasterização SVG via `resvg` + `usvg` + `tiny-skia` e cache persistente `DiskThumbnailCache` em `redb` (`generate_or_load_thumbnail`).
* **Status Git por arquivo** (`crates/fm-telemetry/src/git.rs`, `inspector.rs`): `detect_file_git_status` via `gix` exibido no Inspetor.
* **UX** (`header.rs`, `modals.rs`, `canvas.rs`, `sidebar.rs`, `app.rs`): popover de subpastas nos chevrons `>` do breadcrumb (`breadcrumb_menu_overlay`), resolução interativa de conflitos (`ActiveDialog::Conflict`), botão/atalho de Terminal (`Ctrl+Shift+T`) e barra de uso de disco nas unidades da barra lateral.

---

### Fase P3 — Operações de Arquivo (`CopyEngine`), Lixeira, Favoritos & Miller Columns (2026-11-15)

* **Contratos de transferência & configuração** (`crates/fm-core/src/domain.rs`, `state.rs`, `i18n.rs`):
  * Adicionados `CopyMode`, `ConflictPolicy`, `JobState`, `JobProgress`, `ClipboardTransfer`, além de `default_view_mode` e `favorites` em `AppConfig`.
* **Motor de cópia, lixeira & config TOML** (`crates/fm-fs/src/copy_engine.rs`, `config.rs`, `lib.rs`, `linux.rs`):
  * `run_transfer_job` assíncrono e cancelável com pré-medição, move rápido via `rename` (com fallback cross-device), `copy_file_range` no Linux e `share_mode(0x7)` no Windows.
  * `LocalFs::move_to_trash` (`trash`), `LocalFs::delete_permanent` e persistência atômica TOML (`load_config` / `save_config` via `ProjectDirs`).
* **UI operacional & 3ª coluna** (`crates/fm-app/src/views/modals.rs`, `canvas.rs`, `sidebar.rs`, `footer.rs`, `app.rs`):
  * Modais de Nova Pasta / Novo Arquivo (`Ctrl+Shift+N`), Renomear (`F2`), Confirmação de Lixeira (`Delete`) / Exclusão Permanente (`Shift+Delete`) e Preferências;
  * Barra contextual (`Ctrl+C`/`Ctrl+X`/`Ctrl+V`), banner de progresso com cancelamento, favoritos na Sidebar (`★`) e modo **Miller Columns** (`ViewMode::Columns`) com prévia assíncrona na 3ª coluna (`column_preview`).

---

### Fase P2 — Watcher em Tempo Real, Telemetria, Busca Fuzzy/Profunda & Miniaturas SIMD (2026-11-15)

* **Watcher com debounce** (`crates/fm-index/src/watcher.rs`, `app.rs`): `start_directory_watcher` (`notify` + `notify-debouncer-mini`) integrado via `Subscription::run_with` para refresh silencioso.
* **Workers de telemetria canceláveis** (`crates/fm-telemetry/src/{git,stats,mime,hash,open_shared}.rs`):
  * `detect_git_branch` (`gix`), `count_direct_children` (contagem rasa até `10_000` entradas), `detect_mime_type` (`infer`), `compute_sha256` (`sha2` streaming 64 KB) e abertura compartilhada `share_mode(0x7)` no Windows.
* **Busca fuzzy e profunda** (`crates/fm-index/src/search.rs`, `header.rs`, `inspector.rs`):
  * `fuzzy_rank_entries` (`nucleo-matcher`) no filtro `Ctrl+K` e `deep_search` (`ignore::WalkBuilder` respeitando `.gitignore`) via botão Subpastas / `Ctrl+Shift+F` e modal `deep_search_modal`.
* **Miniaturas SIMD & QuickLook** (`crates/fm-preview/src/thumbs.rs`, `lib.rs`, `canvas.rs`, `inspector.rs`):
  * `ThumbnailCache` (`LruCache`) + `generate_thumbnail` (`image` + `fast_image_resize`), carregamento sob demanda para o viewport visível e QuickLook (`Space`) com preview de imagem e realce de sintaxe (`iced::widget::text_editor` + `iced::highlighter`).

---

### Fase P1 — Navegação Real (`LocalFs`), Unidades Nativas & Virtualização (2026-11-15)

* **Driver real de disco** (`crates/fm-fs/src/{lib,linux,windows}.rs`):
  * `LocalFs` implementando `VfsProvider` em `tokio::task::spawn_blocking` com `CancellationToken`, detecção de symlinks (incluindo links quebrados) e `open::that_detached`.
  * Linux: descoberta de volumes em `/proc/mounts` (decodificando escapes octais como `\040`) + `rustix::fs::statvfs`.
  * Windows: descoberta isolada via `GetLogicalDriveStringsW`, `GetDriveTypeW`, `GetVolumeInformationW`, `GetDiskFreeSpaceExW` e `FILE_ATTRIBUTE_HIDDEN`.
  * `user_places()` via `directories` (`Home`, `Desktop`, `Documents`, `Downloads`, `Pictures` e Lixeira XDG no Linux).
* **Domínio & ordenação paralela** (`crates/fm-core/src/{domain,state,i18n}.rs`): `UserPlaces`, `compute_entry_id` (FNV-1a 64-bit), `format_age` e ordenação paralela com `rayon::par_sort_by` para diretórios com `>= 2.000` itens.
* **Prévia textual & virtualização estrita** (`crates/fm-preview/src/lib.rs`, `crates/fm-app/src/views/virtualized.rs`, `canvas.rs`, `app.rs`):
  * `read_text_preview` (`64 KB`, pulando arquivos binários com byte NUL, `share_mode(0x7)` no Windows);
  * Remoção dos dados mockados (`sample.rs`), navegação real com descarte de respostas obsoletas por `request_id` e renderização estrita apenas dos itens visíveis (+ overscan) com `partition_point` O(log N) no Grid e Tabela.

---

### Fase P0 — Bootstrap do Workspace & Shell Visual Base (2026-11-14)

* **Workspace Cargo** (`Cargo.toml`): estrutura modular com 7 crates (`fm-core`, `fm-fs`, `fm-index`, `fm-telemetry`, `fm-preview`, `fm-theme`, `fm-app`), `edition = "2024"`, `[profile.dev.package."*"] opt-level = 2` e perfil release (`lto = "thin"`, `codegen-units = 1`, `panic = "abort"`).
* **Domínio puro** (`crates/fm-core/src/{domain,state,vfs,i18n}.rs`): `FileEntry` (`CompactString`), `PermissionsMeta`, `DirectorySnapshot`, `SortSpec`, `ViewMode`, `format_bytes`, `TabState`, `SelectionState`, `AppConfig`, `ThemePreset`, `Language`, `VfsProvider`, `FsError` e catálogo `Strings` (`PtBr` / `EnUs`).
* **Design System & fontes** (`crates/fm-theme/src/{fonts,palette,styles}.rs`, `assets/fonts/`): fontes `Inter` e `JetBrains Mono` embutidas via `include_bytes!`, presets `ThemePalette::aether_light()` e `ThemePalette::obsidian_flow()` e builders Iced.
* **Shell GUI P0** (`crates/fm-app/src/{main,sample}.rs`, `crates/fm-app/src/views/{mod,header,sidebar,canvas,inspector,footer}.rs`): alocador `MiMalloc`, ícones `lucide-icons`, layout de 3 painéis com abas, sidebar colapsável (`Ctrl+B`), breadcrumb (`Ctrl+L`), Inspetor (`Alt+I`), QuickLook (`Space`) e rodapé único sobre dados mockados iniciais.

---

### Spec Kit & Fundação Arquitetural (2026-11-14)

* **Documentação de engenharia** (`ARCHITECTURE.md`, `AGENTS.md`): definição da arquitetura 100% Rust nativa (`iced` + `wgpu`), grafo rígido de crates, regras invioláveis (thread de UI sem I/O bloqueante, isolamento `#[cfg(target_os)]` em `fm-fs`, `PathBuf`/`OsString` preservados, Windows `FILE_SHARE_*` e virtualização obrigatória).
* **Spec Kit** (`specs/00-spec-index.md` – `specs/05-tech-stack-prototype.md`):
  * Reconciliação oficial de inconsistências entre `DESIGN.md`, `code.html` e `screen.png`;
  * Tokens semânticos unificados de cores, tipografia e espaçamentos;
  * Contratos assíncronos de domínio, mensagens e cancelamento;
  * Especificação dos componentes/atalhos e roadmap incremental (`P0`–`P3`).

---

## Estado de Verificação Atual

Última bateria executada sobre o workspace atual (sem alterações posteriores):
* `cargo fmt --all -- --check`: aprovado (`exit 0`).
* `cargo clippy --workspace --all-targets -- -D warnings`: aprovado (`0` avisos).
* `cargo test --workspace`: **44 testes aprovados, 0 falhas** (`fm-app`: 7, `fm-core`: 9, `fm-fs`: 12, `fm-index`: 2, `fm-preview`: 7, `fm-telemetry`: 6, `fm-theme`: 1).
* `cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings`: aprovado (`0` avisos).
* `cargo build -p fm-app`: compilado com sucesso (`target/debug/fm-app`).

## Itens Previstos para Próximas Versões

* **`SftpVfs`**: provedor remoto SSH/SFTP implementando `VfsProvider` (`crates/fm-core/src/vfs.rs`) sem alterações estruturais na UI.
* **Polimentos adicionais**: blur opcional em compositores Linux (KDE/Wayland), drag-and-drop interno entre abas/pastas e validação interativa direta em ambiente Windows 11 nativo.
