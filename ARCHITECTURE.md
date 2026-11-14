# Arquitetura do Projeto — Vagnarok File Manager

> Gerenciador de arquivos desktop de alta performance para **Linux** e **Windows**, escrito **100% em Rust** sobre **Iced + wgpu**, com foco em latência sub-milissegundo, baixo consumo de memória e estética *Fluent Glass Minimalist* (*Obsidian Flow* / *Aether Light*).

---

## 1. Visão Geral e Objetivos de Engenharia

O projeto tem como objetivo entregar um gerenciador de arquivos moderno para power users e desenvolvedores, unindo a clareza visual dos mockups ([DESIGN.md](DESIGN.md), [code.html](code.html), [screen.png](screen.png)) com execução nativa sem WebView, sem Electron/Tauri e sem bindings pesados de C/C++.

### Metas Técnicas
* **Renderização GPU a 60–120+ FPS**: Interface desenhada via `iced` + `wgpu` (Vulkan no Linux, DirectX 12/Vulkan no Windows, com fallback `tiny-skia`).
* **Listagem Instantânea**: Diretórios comuns devem aparecer na tela em `< 5 ms`; diretórios massivos (`50.000+` entradas) devem permanecer responsivos graças à **virtualização estrita de viewport** e ao carregamento progressivo.
* **UI Nunca Bloqueia**: Nenhuma chamada síncrona de disco, hash, Git ou decodificação de imagem na thread principal de eventos/renderização.
* **Memória Enxuta**: Uso reduzido de heap com alocador otimizado (`mimalloc` / `jemalloc`), strings compactas (`compact_str`) e cache LRU limitado para thumbnails.
* **Portabilidade Pragmática (Linux + Windows)**: 80%–85% do código agnóstico de sistema operacional; diferenças de SO estritamente confinadas às camadas de adaptação (`fm-fs` e integração de janela em `fm-app`).

---

## 2. Análise e Reconciliação do Mockup Inicial

A inspeção conjunta de [DESIGN.md](DESIGN.md), [code.html](code.html) e [screen.png](screen.png) revelou decisões que devem ser normalizadas na arquitetura:

| Aspecto | Estado Atual no Mockup | Decisão Arquitetural |
| :--- | :--- | :--- |
| **Tema Dark vs. Light** | `DESIGN.md` define *Obsidian Flow* (Dark, `#111319`/`#0F1117`); `code.html` e `screen.png` exibem tema Light (`#f8fafc`, `#ffffff`, `#2563eb`). Além disso, o YAML de `DESIGN.md` difere do texto corrido do próprio arquivo. | Suportar **dois temas oficiais** no crate `fm-theme` sob uma única estrutura `ThemePalette`: **`ObsidianFlow` (Dark)** e **`AetherLight` (Light)**, selecionáveis em runtime e respeitando preferência do sistema por padrão. |
| **Breadcrumbs Duplicados** | O header exibe `home > aether > Documents` e o `<main>` exibe `~ / Documents / Workspace`. | Unificar em **um único Breadcrumb interativo** (com modo edição direta via clique ou `Ctrl+L` e dropdown rápido nos separadores `>`). |
| **Seleção Contraditória** | `main.rs` aparece selecionado e a barra interna mostra `1 item selecionado (8.9 KB)`, mas o footer fixo exibe `Selected: None`. | Única fonte de verdade (`SelectionState` por aba) alimentando Canvas, Inspector e Footer simultaneamente. |
| **Painel Inspetor Direito** | Previsto no `DESIGN.md` (`280px` colapsável/toggleável), mas omitido em `code.html`/`screen.png` (apenas botão `info`). | Implementar o **Right Inspector Drawer (`280px`)** colapsável via botão `info` ou atalho (`Alt+I`), exibindo preview, metadados, permissões, Git e SHA-256 sob demanda. |
| **Controles de Janela (CSD)** | `code.html` usa *traffic lights* estilo macOS à esquerda. | Adaptar por plataforma e configuração: controles à direita no Windows; no Linux, modo configurável (CSD customizado opcional ou decorações nativas do WM/DE para compatibilidade com tiling WMs como `bspwm`/`Hyprland`). |
| **Mistura de Idiomas (i18n)** | Mistura PT-BR e EN na mesma tela (`Quick Access` / `Documentos`, `Preferences`, `6 folders`, `Novo`). | Estruturar strings de interface com catálogo i18n tipado desde o início (`pt-BR` e `en-US`). |
| **Telemetria Cara no Hero/Status** | Mostra tamanho total da pasta (`438.2 MB`), contagem em subpastas (`12 itens`), branch Git e `sha256` do arquivo selecionado. | Executar **sempre em background via workers canceláveis** após a primeira pintura do diretório, com cache e limites de profundidade/timeout. |

---

## 3. Stack Tecnológica 100% Rust

### 3.1 Interface Gráfica e Janela
* **`iced`** (camada de UI reativa estilo Elm, renderização declarativa e tipada).
* **`iced_wgpu` / `wgpu`** (backend gráfico acelerado por GPU via Vulkan/DX12, com fallback `tiny-skia` para ambientes headless ou GPUs antigas).
* **`window-vibrancy` / `raw-window-handle`**:
  * **Windows 11**: Mica / Acrylic nativos via DWM (`DwmSetWindowAttribute`).
  * **Linux**: Efeito blur quando suportado pelo compositor (KDE/Wayland), com degradação elegante para superfícies sólidas/translúcidas definidas nos tokens sem quebrar em WMs sem compositor.
* **Assets Embutidos**: Fontes **Inter** e **JetBrains Mono** + ícones **SVG** empacotados no binário via `include_bytes!`.

### 3.2 Runtime, Concorrência e Memória
* **`tokio`**: Runtime assíncrono multi-thread para I/O, watchers e workers canceláveis (`tokio_util::sync::CancellationToken`).
* **`rayon`**: Paralelismo de dados CPU-bound (ordenação de 100k+ itens, filtragem, travessia paralela).
* **`flume`** (ou `tokio::sync::mpsc`): Canais MPSC de baixa contenção entre workers e o `Subscription` stream do Iced.
* **`compact_str`**: Small-string optimization (evita alocação no heap para nomes de arquivos e extensões curtas).
* **`mimalloc`** (Windows/Linux) ou **`tikv-jemallocator`** (Linux): Alocador global para reduzir fragmentação com grande volume de metadados.

### 3.3 Sistema de Arquivos, OS e Indexação
* **`std::fs` (Fase Protótipo)** evoluindo para **drivers nativos em `fm-fs`**:
  * **Linux (`rustix`)**: `getdents64`, `statx`, `statvfs`, `copy_file_range` / `FICLONE` (reflinks instantâneos em Btrfs/XFS), leitura de `/proc/mounts`.
  * **Windows (`windows` crate)**: `FindFirstFileExW` (`FIND_FIRST_EX_LARGE_FETCH` + `FindExInfoBasic`), `CopyFileExW` (progresso nativo), `GetLogicalDriveStringsW`, `GetVolumeInformationW`, `GetDiskFreeSpaceExW`, `ShellExecuteExW`.
* **`notify` + `notify-debouncer-mini`**: Monitoramento de diretórios em tempo real (`inotify` no Linux / `ReadDirectoryChangesW` no Windows) com janela de debounce (~60 ms).
* **`trash`**: Lixeira nativa (Freedesktop Trash spec no Linux; Recycle Bin / `IFileOperation` no Windows).
* **`directories`**: Resolução padrão de diretórios do usuário (`Home`, `Desktop`, `Downloads`, `Documents`, `Pictures`, `Config`, `Cache`).
* **`ignore` / `jwalk`**: Travessia paralela de diretórios para busca profunda e cálculo assíncrono de estatísticas.
* **`nucleo`**: Motor de busca fuzzy (*Helix engine*, zero-allocation no hot path) para a paleta/busca `Ctrl+K`.

### 3.4 Telemetria, Previews e Persistência
* **`gix` (*gitoxide*)**: Inspeção Git 100% Rust (branch atual no breadcrumb/hero card e status de arquivos).
* **`sha2` + `memmap2`**: Cálculo de `SHA-256` assíncrono e cancelável sob demanda para o item selecionado.
* **`infer`**: Identificação de tipo real via *magic bytes* + tabela estática de extensões/badges (`rs`, `json`, `sh`, `md`, `yml`, `pdf`, `tar`).
* **`image` + `fast_image_resize` + `resvg`**: Decodificação de imagens, redimensionamento SIMD de thumbnails e renderização de SVGs.
* **`syntect`**: Syntax highlighting nativo para o QuickLook (`Space`) de código-fonte e arquivos de configuração.
* **`redb`**: Banco key-value embedded 100% Rust para cache persistente de thumbnails e metadados computados.
* **`serde` + `toml`**: Persistência de configurações do usuário, favoritos, layout e preferências.

---

## 4. Arquitetura Modular (Cargo Workspace)

O projeto é dividido em um **Cargo Workspace** com fronteiras rígidas de dependência. Nenhuma crate de domínio ou sistema operacional pode depender de `iced` ou da UI.

```text
vagnarok_file_manager/
├── Cargo.toml                  # Workspace root + profiles (dev rápido / release LTO)
├── ARCHITECTURE.md             # Este documento
├── AGENTS.md                   # Regras operacionais para agentes de desenvolvimento
├── specs/                      # Spec Kit modular para agentes e protótipo
├── assets/
│   ├── fonts/                  # Inter, JetBrains Mono
│   └── icons/                  # Ícones SVG vetoriais
└── crates/
    ├── fm-core/                # Domínio puro, traits VFS, estados, mensagens, config
    ├── fm-fs/                  # Implementação Linux (rustix) & Windows (windows-rs), CopyEngine, Trash, Mounts
    ├── fm-index/               # Watcher debounced, scan profundo paralelo, busca fuzzy (nucleo)
    ├── fm-telemetry/           # Workers canceláveis: Git (gix), SHA-256, MIME (infer), contagem rasa
    ├── fm-preview/             # QuickLook (Space), thumbnails SIMD, cache LRU + redb, highlight
    ├── fm-theme/               # Tokens Obsidian Flow & Aether Light + estilos Iced reutilizáveis
    └── fm-app/                 # Binário principal: shell Iced, abas, viewports virtualizados, atalhos
```

### Grafo de Dependências Permitido

```text
                ┌──────────────┐
                │    fm-app    │  (Iced Application, Viewports, Keybindings, Subscriptions)
                └──────┬───────┘
       ┌───────────────┼───────────────┬───────────────┬──────────────┐
       ▼               ▼               ▼               ▼              ▼
 ┌───────────┐  ┌────────────┐  ┌─────────────┐  ┌────────────┐  ┌──────────┐
 │ fm-theme  │  │ fm-preview │  │fm-telemetry │  │  fm-index  │  │  fm-fs   │
 └─────┬─────┘  └──────┬─────┘  └──────┬──────┘  └─────┬──────┘  └────┬─────┘
       │               │               │               │              │
       └───────────────┴───────────────┴───────────────┴──────────────┘
                                       │
                                       ▼
                               ┌──────────────┐
                               │   fm-core    │  (Zero I/O direto, zero UI)
                               └──────────────┘
```

### Responsabilidade de Cada Crate

1. **`fm-core`**
   * Tipos de domínio: `VfsPath`, `FileEntry`, `EntryKind`, `PermissionsMeta`, `DriveInfo`, `SortField`, `SortDirection`, `FilterSpec`, `ViewMode` (`Grid`, `DetailTable`, `Columns`).
   * Estado de navegação: `TabId`, `TabState` (histórico voltar/avançar, caminho atual, entradas carregadas, `SelectionState`), `AppConfig`.
   * Trait assíncrona **`VfsProvider`**: abstrai listagem, metadados, criação, renomeação, exclusão e streams de leitura. Permite que o protótipo use `LocalFs` e o futuro suporte `ArchiveVfs` (`.tar.gz`/`.zip`) ou `SftpVfs` sem alterar a UI.
   * Eventos e comandos tipados entre backend e UI.

2. **`fm-fs`**
   * Implementa `VfsProvider` para o sistema de arquivos local (`LocalFs`).
   * Módulos isolados `linux.rs` (`#[cfg(target_os = "linux")]`) e `windows.rs` (`#[cfg(target_os = "windows")]`).
   * Descoberta de volumes/mounts (`SSD Principal`, espaço livre, tipo `ext4`/`btrfs`/`ntfs`).
   * `CopyEngine`: fila de operações de arquivo (copiar, mover, deletar, lixeira) com progresso em bytes, detecção de conflitos e cancelamento.
   * Regras de segurança no Windows: abertura de arquivos para leitura com `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE` para nunca causar `ERROR_SHARING_VIOLATION` em operações concorrentes.

3. **`fm-index`**
   * Gerencia instâncias de watcher (`notify`) vinculadas apenas aos diretórios abertos nas abas ativas.
   * Mantém o índice em memória do diretório atual (e opcionalmente subárvore) alimentando o filtro instantâneo e a busca global `Ctrl+K` via `nucleo`.

4. **`fm-telemetry`**
   * Pipeline de **enriquecimento progressivo** em background:
     * **Prioridade Alta**: Branch Git do diretório atual (`gix`).
     * **Prioridade Média**: Contagem rasa (`1 nível`) de itens das pastas visíveis e soma de tamanhos dos arquivos diretos para o Hero Card.
     * **Prioridade Baixa (Sob Demanda)**: Cálculo `SHA-256` e inspeção detalhada do arquivo atualmente selecionado (com `CancellationToken` amarrado à seleção).

5. **`fm-preview`**
   * Cache LRU em memória para thumbnails prontas + persistência opcional em `redb` (chaveada por `path + size + mtime`).
   * Geração de thumbnails assíncrona priorizando **apenas os índices visíveis no viewport**.
   * Suporte ao **QuickLook (`Space`)**: leitura limitada/paginada de texto com highlight (`syntect`), imagens e resumo de metadados para binários/arquivos grandes.

6. **`fm-theme`**
   * Implementa `ThemePreset::ObsidianFlow` (Dark) e `ThemePreset::AetherLight` (Light).
   * Centraliza paleta de cores, espaçamentos (`4px/8px` grid), raios de borda, espessura hairline (`1px`), fontes (`Inter`, `JetBrains Mono`) e funções construtoras de estilo do Iced (`button`, `container`, `text_input`, `scrollable`).

7. **`fm-app`**
   * Ponto de entrada binário, inicialização do `tokio` runtime + `iced::application`.
   * Layout de 3 painéis (Sidebar colapsável + Canvas principal + Inspector direito toggleável) + Header integrado (Tabs + Toolbar/Breadcrumb + Search + View Switcher) + Status Footer.
   * **Viewports virtualizados** para Grid, Detail Table e Column View.

---

## 5. Fluxo de Dados e Pipeline de Baixa Latência

```text
[Usuário clica em Pasta / Muda Aba]
          │
          ▼
[fm-app] Emite Message::Navigate(tab_id, path)
          │  • Cancela tokens de telemetria/preview da navegação anterior
          │  • Atualiza breadcrumb e histórico imediatamente (feedback visual instantâneo)
          ▼
[fm-fs] Fast Scan do diretório (< 5ms)
          │  • Retorna Vec<FileEntry> básico (nome, kind, size, mtime, perms/attrs)
          ▼
[fm-app] Aplica Sort/Filter (rayon se > 2.000 itens) e renderiza SOMENTE o viewport visível
          │
          ├──▶ [fm-index] Atualiza watcher inotify / ReadDirectoryChangesW para `path`
          │
          └──▶ [fm-telemetry & fm-preview] Workers em background (não bloqueantes)
                 ├── Batch 1: Git branch (`gix`) + info do volume (`ext4`/`ntfs`, free space)
                 ├── Batch 2: Thumbnails apenas para itens visíveis no scroll atual
                 └── Batch 3 (ao selecionar): MIME (`infer`) + SHA-256 cancelável
```

### Estratégia de Virtualização de Viewport
Para cumprir a meta de 120 FPS mesmo com `50.000+` arquivos em uma pasta:
* **Grid View**: Calcula número de colunas dinamicamente pela largura disponível (`viewport_width / cell_width`), deriva `first_visible_row..last_visible_row` a partir do `scroll_offset_y` e instancia widgets **apenas para as linhas visíveis** (+ 1 linha de overscan acima/abaixo), usando espaçadores superior e inferior para manter a altura total da barra de rolagem exata.
* **Detail Table**: Mesmo princípio com altura fixa de linha (`32px`), cabeçalho *sticky* e colunas redimensionáveis.
* **Column View (Miller Columns)**: Cada coluna é uma lista virtualizada independente sincronizada horizontalmente.

---

## 6. Integração Específica: Linux vs. Windows

| Funcionalidade | Linux | Windows |
| :--- | :--- | :--- |
| **Raízes na Sidebar** | `/`, `Home`, pontos de montagem reais (`/proc/mounts`) filtrando pseudo-FS (`proc`, `sysfs`, `tmpfs`) | Unidades lógicas (`C:\`, `D:\`), rótulos de volume e compartilhamentos UNC |
| **Caminhos Longos / Encoding** | Bytes arbitrários (`OsStr`), exibição UTF-8 lossy na UI mantendo `PathBuf` real para syscalls | UTF-16 nativo, suporte a `\\?\` (>260 chars) e manifesto `longPathAware` |
| **Permissões / Ocultos** | `mode` POSIX (`drwxr-xr-x`), ocultos por prefixo `.` | Atributos (`HIDDEN`, `SYSTEM`, `READONLY`), case-insensitivity no rename |
| **Abrir Arquivo / Terminal** | `xdg-open` / fallback para terminal configurado | `ShellExecuteExW` (`open`, `properties`, terminal padrão) |
| **Decoração de Janela** | Configurável: decorações nativas do WM ou CSD leve (tiling-friendly) | CSD integrado com DWM Mica/Acrylic e controles à direita |

---

## 7. Otimizações de Compilação e Qualidade

* **Profile `release` no `Cargo.toml`**:
  * `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`, `panic = "abort"`, `strip = "symbols"`.
* **Profile `dev` otimizado para iteração rápida em UI**:
  * Manter `opt-level = 0` para crates do workspace, mas aplicar `[profile.dev.package."*"] opt-level = 2` (especialmente para `wgpu`, `image`, `iced`, `gix`, `syntect`) para que o protótipo em modo debug rode fluido a 60 FPS.
* **Proibição de `unwrap()`/`expect()` em caminhos de I/O**: Todo acesso a disco pode falhar por permissão, symlink quebrado ou bloqueio no Windows; erros devem degradar para estado exibível por item ou toast não-bloqueante.
* **Testes e Benchmarks**:
  * Testes unitários puros em `fm-core`, `fm-fs`, `fm-index` e `fm-telemetry`.
  * CI obrigatória em **`ubuntu-latest` e `windows-latest`** desde o primeiro PR do protótipo.
