# Plano de Implementação — Recurso **"Abrir com..."** (`Open With`)

Documento de estudo e referência arquitetural para adicionar ao **Vagnarok File Manager** a seleção de aplicativos externos, execução personalizada e persistência do aplicativo padrão por tipo de arquivo, seguindo o modelo visual enviado (estilo Nemo/Cinnamon adaptado ao Design System **ObsidianFlow / AetherLight**).

---

## 1. Objetivos Funcionais

1. Permitir ao usuário **escolher com qual programa abrir um arquivo** a partir de:
   - **Aba "Abrir com"** dentro da janela de **Propriedades** (`PropertiesWindowState`);
   - Atalho **"Abrir com..."** no menu de contexto de arquivos (`ContextMenuKind::Entry`).
2. Categorizar automaticamente os aplicativos instalados para o arquivo selecionado em:
   - **Aplicativo padrão** (atualmente configurado);
   - **Aplicativos recomendados** (associados ao mesmo MIME type / extensão);
   - **Outros aplicativos** (demais programas gráficos descobertos no sistema).
3. Permitir digitar ou selecionar um **comando/executável personalizado** e:
   - **Abrir apenas uma vez** com o aplicativo ou comando selecionado;
   - **Adicionar à lista** de aplicativos conhecidos para aquele tipo;
   - **Definir como padrão** e persistir a escolha;
   - **Restaurar para os padrões do sistema** removendo o override salvo.
4. Fazer o duplo clique / ação **Abrir** (`Message::ActivateEntry`) respeitar o aplicativo padrão salvo antes de recorrer ao comportamento atual (`VfsProvider::open_default`).

---

## 2. Regras Arquiteturais Aplicáveis (`AGENTS.md`)

- **100% Rust nativo**: nenhuma dependência WebView/Node/Python; parsers de `.desktop`/`mimeapps.list` e consultas ao Registro Win32 escritos em Rust puro.
- **Thread de UI sagrada**: descoberta de apps, leitura de MIME, leitura de `.desktop`/Registro, rasterização de ícones de aplicativos e `spawn` de processos rodam exclusivamente em `tokio::task::spawn_blocking` / workers assíncronos.
- **Fronteiras de crates**:
  - `fm-core`: apenas structs de domínio, variantes de `AppConfig`, assinatura em `VfsProvider` e chaves Fluent (`.ftl`), sem `iced` nem APIs de SO.
  - `#[cfg(target_os = "linux")]` e `#[cfg(target_os = "windows")]`: confinados em `fm-fs`.
- **Paths reais vs. exibição**: armazenar executáveis e arquivos alvo como `PathBuf`/`OsString` (ou comando tokenizado seguro), convertendo com `to_string_lossy()` apenas na renderização.
- **Sem `unwrap()` em I/O**: erros ao ler `.desktop`, consultar Registro ou iniciar processos externos devem retornar `Result<..., FsError>` com feedback limpo em `status_notice`.

---

## 3. Modelo Visual (Design System Vagnarok)

Em vez da estética GTK cinza tradicional da imagem de referência, o layout da aba/modal deve reutilizar diretamente os componentes de `crates/fm-app/src/views/modals.rs` e os tokens de `fm-theme`:

```text
+-----------------------------------------------------------------------+
| [i] Propriedades de ANTARES - VENOM...mp4                         [x] |
+-----------------------------------------------------------------------+
| Geral | Segurança | Histórico | Metadados | Permissões | [Abrir com] |
+-----------------------------------------------------------------------+
|  Selecione um aplicativo para abrir "ANTARES - VENOM...mp4"           |
|  e outros arquivos do tipo "Vídeo MPEG-4" (`video/mp4`).              |
|                                                                       |
|  +-----------------------------------------------------------------+  |
|  | APLICATIVO PADRÃO                                               |  |
|  |  [icone] Reprodutor de Mídias VLC                (Selecionado)  |  |
|  |                                                                 |  |
|  | APLICATIVOS RECOMENDADOS                                        |  |
|  |  [icone] Celluloid                                              |  |
|  |  [icone] mpv Media Player                                       |  |
|  |  [icone] MediaInfo                                              |  |
|  |                                                                 |  |
|  | OUTROS APLICATIVOS                                              |  |
|  |  [icone] Audacity                                               |  |
|  |  [icone] OBS Studio                                             |  |
|  +-----------------------------------------------------------------+  |
|                                                                       |
|  Comando ou executável personalizado:                                 |
|  [ /usr/bin/mpv --fs                                    ]  [ Pasta ]  |
|                                                                       |
|  [ Adicionar à lista ]  [ Definir como padrão ]  [ Restaurar padrão ] |
+-----------------------------------------------------------------------+
|                                       [ Abrir agora ]  [ Fechar ]     |
+-----------------------------------------------------------------------+
```

### Tokens e Primitivas Reutilizadas
- **Container da lista**: `styles::panel_surface(palette.surface_low, palette.border_subtle, 8.0)` com `scrollable(...).height(Length::Fixed(230.0))`.
- **Cabeçalhos de seção** (`APLICATIVO PADRÃO`, `APLICATIVOS RECOMENDADOS`, `OUTROS APLICATIVOS`): `text(...).size(10).font(FONT_MONO_BOLD).color(palette.text_muted)`.
- **Item selecionado / ativo**: `styles::panel_surface(palette.accent_surface, palette.accent_border, 6.0)` com texto `palette.text_primary` em `FONT_UI_SEMIBOLD`.
- **Ícones de aplicativos**: `widget::image` a partir de `image::Handle::from_rgba(w, h, rgba)` (pipeline `SystemIconPayload` já existente em `fm-app` / `fm-preview`), com fallback para ícone Lucide (`icon_app_window` / `icon_terminal`) quando o app não possuir ícone gráfico ou estiver no modo `Bundled` (opcionalmente carregando ícones reais de apps sempre nessa tela para reconhecimento imediato).
- **Campo de comando personalizado**: `text_input(..., ...).style(styles::search_input_style(palette))` dentro de `panel_surface`, acompanhado de botão `pill_secondary_button(palette)` com `icon_folder()`.
- **Botões de ação**: `pill_primary_button(palette)` para a ação principal (**Definir como padrão** / **Abrir**) e `pill_secondary_button(palette)` para **Adicionar à lista** e **Restaurar para os padrões do sistema**.

---

## 4. Arquitetura por Crate

### Etapa 1 — Contratos e Persistência (`crates/fm-core`)

1. **`crates/fm-core/src/domain.rs`**:
   Criar tipos puros e serializáveis para aplicativos e associações:
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   pub struct OpenWithApp {
       /// Identificador estável (`org.videolan.VLC.desktop` no Linux; ProgId/exe no Windows; `custom:<cmd>`).
       pub id: CompactString,
       pub display_name: CompactString,
       /// Linha de comando bruta (`Exec=` no Linux ou caminho do `.exe` no Windows).
       pub exec: String,
       /// Nome de ícone FreeDesktop ou caminho de executável/ícone para resolução via `fm-fs`/`fm-preview`.
       pub icon_hint: Option<String>,
       pub is_recommended: bool,
       pub is_default: bool,
       pub is_custom: bool,
       pub runs_in_terminal: bool,
   }

   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   pub struct OpenWithCatalog {
       /// Tipo MIME normalizado (ex.: `"video/mp4"`) ou chave por extensão (ex.: `".mp4"`).
       pub type_key: CompactString,
       /// Rótulo amigável do tipo de arquivo (ex.: `"Vídeo MPEG-4"` ou `"Arquivo MP4"`).
       pub type_label: CompactString,
       pub system_default: Option<OpenWithApp>,
       pub recommended: Vec<OpenWithApp>,
       pub others: Vec<OpenWithApp>,
   }

   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   pub struct OpenWithAssociation {
       pub app_id: String,
       pub display_name: String,
       pub exec: String,
       #[serde(default)]
       pub icon_hint: Option<String>,
       #[serde(default)]
       pub runs_in_terminal: bool,
   }
   ```
   *(Verificar `serde` em `crates/fm-core/Cargo.toml` — `domain.rs` ou `state.rs` já usam `serde::Serialize/Deserialize`.)*

2. **`crates/fm-core/src/state.rs`** (`AppConfig`):
   Adicionar campos com `#[serde(default)]` para compatibilidade total com `config.toml` antigos:
   ```rust
   /// Associação padrão salva por chave de tipo (`video/mp4` ou `.ext`).
   #[serde(default)]
   pub open_with_defaults: BTreeMap<String, OpenWithAssociation>,
   /// Comandos personalizados adicionados pelo usuário por chave de tipo.
   #[serde(default)]
   pub open_with_custom_apps: BTreeMap<String, Vec<OpenWithAssociation>>,
   ```

3. **`crates/fm-core/src/vfs.rs`** (`VfsProvider`):
   Estender o trait com método explícito de execução:
   ```rust
   async fn open_with(&self, path: &Path, target: &OpenWithAssociation) -> Result<(), FsError>;
   ```

4. **Localização (`crates/fm-core/locales/*.ftl` e `crates/fm-core/src/i18n.rs`)**:
   Adicionar todas as novas strings nos 6 idiomas suportados (`en-US`, `pt-BR`, `es-ES`, `fr-FR`, `de-DE`, `it-IT`), mantendo 100% de paridade validada pelo teste `all_locales_match_canonical_keys_and_variables`:
   - `open-with` (`"Abrir com..."`)
   - `prop-tab-open-with` (`"Abrir com"`)
   - `open-with-prompt` (`"Selecione um aplicativo para abrir este arquivo e outros do mesmo tipo:"`)
   - `open-with-default-app` (`"Aplicativo padrão"`)
   - `open-with-recommended-apps` (`"Aplicativos recomendados"`)
   - `open-with-other-apps` (`"Outros aplicativos"`)
   - `open-with-custom-hint` (`"Ou digite/selecione um comando ou executável personalizado:"`)
   - `open-with-custom-placeholder` (`"Ex.: mpv --no-terminal ou /caminho/programa"`)
   - `open-with-add-to-list` (`"Adicionar à lista"`)
   - `open-with-set-default` (`"Definir como padrão"`)
   - `open-with-reset-default` (`"Restaurar para os padrões do sistema"`)
   - `open-with-open-once` (`"Abrir agora"`)
   - `open-with-loading` (`"Carregando aplicativos instalados..."`)
   - `open-with-empty` (`"Nenhum aplicativo compatível encontrado."`)
   - `open-with-saved` (`"Aplicativo padrão atualizado."`)
   - `open-with-reset-done` (`"Padrão do sistema restaurado."`)

---

### Etapa 2 — Backend Nativo Linux/Flatpak e Windows (`crates/fm-fs`)

Criar um módulo dedicado `crates/fm-fs/src/open_with/` seguindo o mesmo isolamento já usado em `crates/fm-fs/src/icons/`:
- `crates/fm-fs/src/open_with/mod.rs` (API pública `discover_open_with_apps`, `set_system_default_app`, `reset_system_default_app`, `launch_open_with`)
- `crates/fm-fs/src/open_with/linux.rs` (`#[cfg(target_os = "linux")]`)
- `crates/fm-fs/src/open_with/windows.rs` (`#[cfg(target_os = "windows")]`)

#### 2.1 Implementação Linux + Flatpak (`linux.rs`)
1. **Resolução de MIME do arquivo alvo**:
   - Reutilizar a tabela `globs2` (`ext -> mime`) já parseada em `crates/fm-fs/src/icons/linux.rs` + fallback por extensão/conteúdo (via `infer` / `fm_telemetry::detect_mime`) para obter `mime_type` (ex.: `video/mp4`) e chave fallback `.ext` quando o MIME for `application/octet-stream`.
   - Ler descrição amigável do MIME em `/usr/share/mime/<tipo>/<subtipo>.xml` (tag `<comment xml:lang="...">`, fallback `<comment>`), ou no Flatpak em `/run/host/usr/share/mime/...`.
2. **Descoberta de `.desktop` (em worker `spawn_blocking`)**:
   - Diretórios XDG de aplicativos:
     - Nativo: `$XDG_DATA_HOME/applications`, `/usr/local/share/applications`, `/usr/share/applications`, `/var/lib/flatpak/exports/share/applications`, `$HOME/.local/share/flatpak/exports/share/applications`.
     - Flatpak (`FLATPAK_ID` presente): incluir `/run/host/usr/share/applications`, `/run/host/usr/local/share/applications` e `$HOME/.local/share/applications` (usando `BaseDirs::home_dir()`, pois `$XDG_DATA_HOME` dentro do Flatpak aponta para o sandbox).
   - Parsear `[Desktop Entry]`:
     - Ignorar entradas com `NoDisplay=true`, `Hidden=true`, `Type != Application` ou sem `Exec=`.
     - Extrair `Name[locale]` -> `Name`, `Icon`, `Exec`, `MimeType` (lista separada por `;`), `Terminal`.
   - Classificação:
     - Consultar `~/.config/mimeapps.list` (no Flatpak: `$HOME/.config/mimeapps.list`) e `/usr/share/applications/mimeapps.list` na seção `[Default Applications]` e `[Added Associations]` para determinar o padrão atual do sistema.
     - Apps cujo `MimeType` contenha o MIME alvo (ou subtipo herdado/genérico compatível) vão para **Recomendados**; os demais vão para **Outros aplicativos**, ordenados alfabeticamente sem duplicatas por `desktop_id`.
3. **Execução (`launch_open_with`)**:
   - **Tokenizador seguro de `Exec=`** (sem `sh -c`):
     - Respeitar aspas simples/duplas e escapes conforme a *Desktop Entry Specification*.
     - Expandir field codes: `%f` / `%F` -> caminho local real do arquivo, `%u` / `%U` -> URI `file://...`, `%c` -> nome do app, `%i` / `%k` removidos/expandidos, `%%` -> `%`. Se nenhum `%f`/`%F`/`%u`/`%U` existir no comando (comum em comandos personalizados digitados pelo usuário), anexar o caminho do arquivo ao final dos argumentos.
   - **Execução no Flatpak (`FLATPAK_ID` definido)**:
     - Como o container Flatpak não enxerga `/usr/bin/vlc` ou bibliotecas do host diretamente, executar o comando via `flatpak-spawn --host <bin> <args...>` (viável porque `packaging/flatpak/dev.vagnarok.FileManager.yml` já concede `--talk-name=org.freedesktop.Flatpak` e `--filesystem=host`).
     - Se `Terminal=true`, tentar encapsular com terminal padrão do host (`x-terminal-emulator`, `gnome-terminal`, `konsole`, `xfce4-terminal`, etc.) ou reportar erro limpo se indisponível.
   - Desacoplar `stdin/stdout/stderr` (`Stdio::null()`) para não travar a janela após encerrar o gerenciador.
4. **Persistência sincronizada opcional (`~/.config/mimeapps.list`)**:
   - Ao salvar padrão no Linux, atualizar `AppConfig::open_with_defaults` **e** (quando for um `.desktop` válido) atualizar `[Default Applications]` em `$HOME/.config/mimeapps.list` de forma atômica (`.tmp` + `rename`), mantendo consistência com Nautilus/Nemo/xdg-open.
   - Em **Restaurar padrões do sistema**, remover a entrada de `AppConfig::open_with_defaults` e remover o override manual do MIME em `$HOME/.config/mimeapps.list`.

#### 2.2 Implementação Windows (`windows.rs`)
1. **Descoberta no Registro Win32 (`windows` crate)**:
   - Usar a crate `windows` (adicionando `Win32_System_Registry` já listada em `Cargo.toml:58`!):
     - Ler extensão `.ext` em `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.ext\UserChoice` (`ProgId`) para identificar o padrão atual do Windows.
     - Agregar `ProgId`s em `FileExts\.ext\OpenWithProgids`, `FileExts\.ext\OpenWithList`, `HKEY_CLASSES_ROOT\.ext\OpenWithProgids` e programas em `HKEY_CLASSES_ROOT\Applications\*` / `App Paths`.
     - Resolver o comando em `<ProgId>\shell\open\command` (limpando `%1` / `"%1"` / `%*` para obter o executável base e argumentos) e o `FriendlyAppName` / `DefaultIcon`.
   - Classificação:
     - `UserChoice` -> **Aplicativo padrão**;
     - `OpenWithProgids` / `OpenWithList` da extensão -> **Aplicativos recomendados**;
     - Demais apps descobertos em `Applications` / `App Paths` -> **Outros aplicativos**.
2. **Ícones dos aplicativos no Windows**:
   - Passar o caminho do `.exe` para uma variação do extrator `SHGetFileInfoW` já implementado em `crates/fm-fs/src/icons/windows.rs` (com guardas RAII `OwnedHicon` / `OwnedBitmap`).
3. **Execução (`launch_open_with`)**:
   - Tokenizar argumentos de `exec`, substituindo `%1` / `"%1"` pelo caminho real do arquivo (ou passando `path` como argumento ao executável) via `std::process::Command::new(&exe).args(...)` com `CREATE_NO_WINDOW` apenas se necessário — preservando janelas GUI normais — e tratando erros via `FsError`.
4. **Persistência no Windows (`AppConfig` vs. `UserChoice`)**:
   - **Por que NÃO gravar `HKCU\...\FileExts\.ext\UserChoice` diretamente**: desde o Windows 8/10/11, a chave `UserChoice` exige um hash proprietário calculado pelo sistema; gravações diretas por terceiros são invalidadas pelo Windows.
   - **Solução robusta**:
     - Salvar a escolha do usuário em `AppConfig::open_with_defaults` (chave normalizada da extensão, ex.: `".mp4"`), fazendo o duplo clique e o menu **Abrir** do Vagnarok obedecerem imediatamente o programa escolhido.
     - O botão **"Restaurar para os padrões do sistema"** remove o override em `AppConfig::open_with_defaults`, voltando a chamar `open_default` (`ShellExecuteExW(..., "open")` em `crates/fm-fs/src/windows.rs:199`).
     - *(Opcional futuro)*: um link discreto *"Alterar associação global no Windows..."* chamando `SHOpenWithDialog` para quando o usuário quiser alterar fora do app também.

---

### Etapa 3 — Pipeline de Ícones dos Aplicativos (`fm-preview` / `fm-app`)

- Hoje o app já possui `SystemIconKey`, `SystemIconBucket` e cache `HashMap<(SystemIconKey, SystemIconBucket), iced::widget::image::Handle>` em `fm-app`.
- Para o recurso "Abrir com...", adicionar uma chave simples ou carregamento dedicado assíncrono por lote para os apps retornados em `OpenWithCatalog`:
  - **Linux**: resolver `icon_hint` usando `lookup_icon_file` (do motor de ícones Linux já existente) para SVG/PNG -> rasterizar em RGBA8 24×24 via `resvg` / `image`.
  - **Windows**: resolver ícone de `.exe`/`.ico` via `SHGetFileInfoW` para RGBA8 24×24.
  - Guardar os handles em cache em memória no `App` (`open_with_icon_cache: HashMap<String, iced::widget::image::Handle>`) evitando recarregar ao trocar de arquivo ou reabrir Propriedades.

---

### Etapa 4 — Estado e Interface em `crates/fm-app`

1. **`crates/fm-app/src/app.rs`**:
   - Adicionar `PropertiesTab::OpenWith` em `PropertiesTab`.
   - Adicionar estado `OpenWithDraftState` em `PropertiesWindowState`:
     - `loading: bool`
     - `catalog: Option<OpenWithCatalog>`
     - `selected_app_id: Option<String>`
     - `custom_command: String`
     - `pending_default: Option<OpenWithAssociation>` (para integrar ao botão **Aplicar / Confirmar** das Propriedades **ou** aplicar imediatamente via **Definir como padrão**, mantendo comportamento claro e sem surpresa).
   - *(Opcional se houver modal dedicado para "Abrir com..." pelo menu de contexto)*: criar `ActiveDialog::OpenWith { path, state }` reutilizando a mesma função de renderização `open_with_panel(...)` da aba de Propriedades para zero duplicação de UI.
   - Atualizar `Message::ActivateEntry(index)` (`app.rs:1235`) e `Message::PropertiesOpenDefault(path)` (`app.rs:2779`):
     - Verificar `self.resolve_open_with_override(&entry.path)` (consulta `config.open_with_defaults` por MIME ou extensão em lowercase).
     - Se existir override, despachar `vfs.open_with(&path, &assoc)`; caso contrário, manter `vfs.open_default(&path)`.
   - Adicionar `Message::OpenPropertiesAtTab(PathBuf, PropertiesTab)` ou `Message::OpenWithDialog(PathBuf)` para o menu de contexto.

2. **`crates/fm-app/src/views/modals.rs`**:
   - Em `ContextMenuKind::Entry` (apenas quando `!is_dir`, `!fm_core::is_virtual_trash_path(path)` e `fm_fs::archive::split_archive_path(path).is_none()`):
     - Inserir item **"Abrir com..."** (`icon_external_link` ou `icon_sliders_horizontal` / `icon_app_window`) logo abaixo de **Abrir**.
   - Em `properties_window_view` (`modals.rs:470-490`):
     - Exibir a aba `PropertiesTab::OpenWith` quando `!is_dir && !is_archive && !is_virtual_trash_path`.
   - Implementar componente compartilhado `open_with_section(...)` renderizando:
     1. Banner informativo com o nome do arquivo e tipo MIME/descrição;
     2. Caixa rolável categorizada (`Aplicativo padrão`, `Aplicativos recomendados`, `Outros aplicativos`, mais eventuais itens de `open_with_custom_apps` adicionados pelo usuário);
     3. Input de comando personalizado + botão de pasta (pode acionar um seletor de arquivo modal interno simples usando `VfsProvider::list_dir`, sem precisar de biblioteca C externa);
     4. Linha de ações: **Adicionar à lista**, **Definir como padrão**, **Restaurar padrões do sistema** e **Abrir agora**.
   - Atualizar `est_height` do `ContextMenuKind::Entry` em `modals.rs:2701` (+32 px para entradas de arquivo regular) para que o menu nunca corte perto da borda inferior da janela.

---

## 5. Riscos, Edge Cases e Mitigações

| Risco / Caso de Borda | Impacto | Mitigação |
| :--- | :--- | :--- |
| **Execução dentro do Flatpak** | Programas do host (`/usr/bin/...`) falham se chamados diretamente dentro do runtime Flatpak | Usar `flatpak-spawn --host` quando `FLATPAK_ID` estiver presente (`--talk-name=org.freedesktop.Flatpak` já está habilitado no manifesto). Tratar ausência/erro com fallback claro ao portal `open::that_detached` quando apropriado. |
| **Injeção ou falha por espaços/aspas em comandos** | Arquivos com espaços, parênteses ou unicode podem quebrar se passados via shell | Nunca invocar `sh -c`; usar parser próprio de `Exec=` passando argumentos diretamente ao `std::process::Command::args(...)`. |
| **Arquivos sem extensão ou MIME genérico (`application/octet-stream`)** | Muitos arquivos têm MIME ambíguo | Usar chave composta: MIME específico quando conhecido (diferente de `application/octet-stream`), caso contrário chave `.ext` em minúsculas (e fallback por nome exato se não tiver extensão). |
| **Windows `UserChoice` protegido por hash** | Escrita direta no Registro falha no Windows 10/11 | Persistir o padrão no `config.toml` do Vagnarok (`open_with_defaults`) como fonte de verdade dentro do gerenciador, mantendo restauração instantânea para o comportamento nativo do Windows. |
| **Muitos `.desktop` no sistema (ex.: Wine, Steam, Proton)** | Lista poluída ou lentidão ao abrir Propriedades | Filtrar `NoDisplay=true`, `Hidden=true`, deduplicar por nome+exec, agrupar os compatíveis no topo (`Recomendados`) e opcionalmente adicionar campo rápido de filtro dentro de `Outros aplicativos`. |
| **App personalizado inexistente** | Erro silencioso | Validar existência do binário (`which` / `Path::exists`) ao clicar em **Definir como padrão** / **Abrir agora**, exibindo erro amigável no rodapé da modal ou `status_notice`. |

---

## 6. Ordem Sugerida de Implementação e Testes

1. **Fase 1 (`fm-core` + `locales`)**: tipos `OpenWith*`, campos em `AppConfig`, assinatura em `VfsProvider`, traduções `.ftl` e testes de serialização/paridade de locales.
2. **Fase 2 (`fm-fs` Linux + Windows)**: parser `.desktop` / `mimeapps.list`, tokenizador `Exec=` (com testes unitários de `%f`, `%U`, aspas e `%%`), leitor de Registro Windows e cross-check `x86_64-pc-windows-gnu`.
3. **Fase 3 (`fm-app` + ícones)**: carregamento assíncrono do catálogo + ícones de apps, UI da aba `Abrir com` em Propriedades + atalho no menu de contexto + integração com `ActivateEntry`.
4. **Verificação final obrigatória (`AGENTS.md`)**:
   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings`
   - Validação prática no build **nativo Linux** e no pacote **Flatpak** (`flatpak-spawn --host`).
