# Plano de Implementação: Quick Jump (`Ctrl+P`)

Este documento registra o plano técnico para implementar o **Quick Jump (`Ctrl+P`)** no **Vagnarok File Manager**, permitindo digitar o nome (ou parte do caminho) de uma pasta e abri-la instantaneamente sem navegar manualmente, seguindo as regras de [AGENTS.md](AGENTS.md).

---

## 1. Fontes de Pastas em Duas Camadas (Instantâneo + Assíncrono)

* **Camada 1 — Candidatos instantâneos em memória (0 ms de disco)**:
  * Subpastas visíveis/carregadas na aba atual (`tab.raw_entries` onde `kind.is_dir_like()`).
  * Histórico de navegação (`back_stack` / `forward_stack` das abas), abas abertas e `favorites` / `user_places` / `drives` (`crates/fm-app/src/app.rs`).
  * Diretórios-pai dos arquivos recentes (`config.recent_files`).
* **Camada 2 — Varredura assíncrona de diretórios (`crates/fm-index/src/search.rs`)**:
  * Nova função `quick_jump_dirs(roots, query, max_results, include_hidden, cancel)` em `tokio::task::spawn_blocking` usando BFS apenas em diretórios (profundidade controlada, ex.: até 5 níveis a partir da pasta atual e da Home), cancelada instantaneamente a cada tecla via `CancellationToken`.
  * Os resultados do disco são mesclados e deduplicados por `PathBuf` sobre os candidatos em memória sem bloquear a UI.

---

## 2. Ranking Fuzzy Inteligente (`crates/fm-app/src/quick_jump.rs`)

* Estrutura `QuickJumpItem { path: PathBuf, name: CompactString, display_path: CompactString, source: QuickJumpSource, score: u32 }`.
* **Prioridade de pontuação**:
  1. Correspondência exata ou por prefixo no nome da pasta.
  2. Bônus por relevância da origem: *Subpasta atual* > *Favorito / Local do sistema* > *Visitada recentemente* > *Subpasta descoberta via BFS*.
  3. Score fuzzy (`nucleo-matcher`) no nome da pasta e no caminho completo (quando o usuário digitar `/`, `\` ou múltiplos termos como `proj/src`).

---

## 3. Estado, Atalhos e Navegação (`crates/fm-app/src/app.rs`)

* **Campos no `App`**: `quick_jump_open: bool`, `quick_jump_query: String`, `quick_jump_selected: usize`, `quick_jump_items: Vec<QuickJumpItem>`, `quick_jump_request_id: u64`, `quick_jump_cancel: Option<CancellationToken>`, `quick_jump_recent: Vec<PathBuf>` e `quick_jump_input_id()`.
* **Mensagens**: `OpenQuickJump`, `CloseQuickJump`, `QuickJumpQueryChanged(String)`, `QuickJumpMoveSelection(isize)`, `QuickJumpResults { request_id, items }`, `QuickJumpOpenSelected { new_tab: bool }`, `QuickJumpOpenPath { path, new_tab: bool }`.
* **Atalhos em `GlobalKeyPressed`**:
  * `Ctrl+P` (sem `Shift`, preservando `Ctrl+Shift+P` da Command Palette) abre/alterna o Quick Jump e foca o campo.
  * Com o Quick Jump aberto: `↑`/`↓` movem a seleção, `Enter` navega imediatamente para a pasta na aba ativa, `Ctrl+Enter` abre em nova aba e `Esc` fecha.
* Registrar também o comando **Quick Jump (`Ctrl+P`)** no catálogo da Command Palette (`crates/fm-app/src/command_palette.rs`).

---

## 4. Interface Visual & i18n

* Modal `quick_jump_modal` em `crates/fm-app/src/views/inspector.rs` seguindo os tokens `ObsidianFlow` / `AetherLight`:
  * Campo de entrada com ícone de pasta, placeholder localizado e badge `Ctrl+P`.
  * Lista de pastas exibindo **nome da pasta**, **caminho completo** em `FONT_MONO`, badge de origem (*Atual*, *Favorito*, *Recente*, *Subpasta*) e botão rápido para abrir em nova aba.
  * Rodapé com dicas (`↑↓ navegar • Enter abrir • Ctrl+Enter nova aba • Esc fechar`).
* Traduções nos 6 idiomas em `crates/fm-core/src/i18n.rs` e `crates/fm-core/locales/*.ftl`.

---

## 5. Verificação Planejada

* Testes unitários em `fm-index` (`quick_jump_dirs` e cancelamento) e `fm-app` (deduplicação, ranking por origem/fuzzy e clamping de seleção).
* Validação completa:
  * `cargo check --workspace --all-targets`
  * `cargo clippy --workspace --all-targets -- -D warnings`
  * `cargo test --workspace`
