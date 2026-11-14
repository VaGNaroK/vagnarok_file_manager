# Spec 05 — Stack Tecnológica & Plano do Protótipo

Guia prático das tecnologias Rust escolhidas e etapas executáveis para levantar o protótipo funcional no menor tempo sem comprometer a arquitetura final.

---

## 1. Stack de Crates para Iniciar o Protótipo

### Workspace & Otimização Global (`fm-app` / root)
* `iced` (features: `wgpu`, `tokio`, `image`, `svg`, `advanced`) — GUI declarativa acelerada por GPU.
* `tokio` (features: `rt-multi-thread`, `fs`, `sync`, `time`) — Runtime assíncrono.
* `tokio-util` — `CancellationToken` para abortar tarefas obsoletas.
* `rayon` — Ordenação e scan paralelo.
* `compact_str` — Strings curtas sem alocação no heap para nomes/extensões.
* `mimalloc` (ou `tikv-jemallocator` no Linux) — Alocador global de baixa fragmentação.
* `thiserror` / `anyhow` — Erros tipados nas crates de biblioteca (`fm-core`, `fm-fs`) e contexto na aplicação.
* `tracing` + `tracing-subscriber` — Diagnóstico e medição de latência (`< 5ms` por listagem).

### Sistema de Arquivos & SO (`fm-fs`)
* **Multiplataforma base**:
  * `async-trait`, `directories`, `trash`, `open` (fallback inicial para abrir arquivo padrão enquanto `ShellExecuteExW` / `xdg-open` customizado é refinado).
* **Linux (`[target.'cfg(target_os = "linux")'.dependencies]`)**:
  * `rustix` (features: `fs`, `process`) — `statvfs`, `statx`, `getdents64`, `copy_file_range`.
* **Windows (`[target.'cfg(target_os = "windows")'.dependencies]`)**:
  * `windows` (features: `Win32_Storage_FileSystem`, `Win32_System_IO`, `Win32_UI_Shell`, `Win32_Foundation`) — `FindFirstFileExW`, `GetLogicalDriveStringsW`, `GetVolumeInformationW`, `GetDiskFreeSpaceExW`, `CopyFileExW`, `ShellExecuteExW`.
  * `window-vibrancy` (opcional no protótipo, para aplicar Mica/Acrylic na janela Win11).

### Indexação, Busca & Watcher (`fm-index`)
* `notify` + `notify-debouncer-mini` — Watcher nativo com debounce.
* `ignore` (ou `jwalk`) — Travessia paralela respeitando regras de diretório.
* `nucleo` — Motor fuzzy de altíssima performance para `Ctrl+K`.

### Telemetria & Preview (`fm-telemetry`, `fm-preview`)
* `gix` — Leitura de repositório Git em Rust puro (sem `libgit2`).
* `sha2` + `memmap2` — Hashing rápido e cancelável.
* `infer` — Detecção de tipo por magic bytes.
* `image` + `fast_image_resize` + `resvg` — Thumbnails SIMD e ícones/previews SVG.
* `syntect` — Syntax highlighting para QuickLook de código.
* `redb` — Cache persistente em disco 100% Rust (fase P2+).

### Configuração & Tema (`fm-core`, `fm-theme`)
* `serde` + `toml` — Configurações e favoritos do usuário.

---

## 2. Estrutura Inicial do `Cargo.toml` Raiz

```toml
[workspace]
resolver = "2"
members = [
    "crates/fm-core",
    "crates/fm-fs",
    "crates/fm-index",
    "crates/fm-telemetry",
    "crates/fm-preview",
    "crates/fm-theme",
    "crates/fm-app",
]

[workspace.package]
edition = "2024" # ou "2021" conforme toolchain instalado
license = "MIT OR Apache-2.0"

[profile.dev.package."*"]
opt-level = 2 # Mantém Iced/wgpu/image fluidos mesmo em build de desenvolvimento

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

---

## 3. Roadmap Incremental do Protótipo

### Fase P0 — Esqueleto do Workspace & Design System (Dia 1)
* Criar o workspace Cargo com as 7 crates compilando (`cargo check --workspace`).
* Embutir `Inter` e `JetBrains Mono` + conjunto mínimo de SVGs em `assets/`.
* Implementar `fm-theme` com `AetherLight` e `ObsidianFlow` completos conforme [02-design-system-tokens.md](02-design-system-tokens.md).
* **Critério de aceite**: Janela Iced abre renderizada via `wgpu` alternando entre os dois temas e exibindo o layout base de 3 painéis com dados mockados.

### Fase P1 — Navegação Real & Shell Cross-Platform (Linux + Windows)
* Implementar `LocalFs` em `fm-fs` com:
  * `list_dir` assíncrono (`tokio::task::spawn_blocking` usando `std::fs` inicialmente, já retornando `PermissionsMeta`, tipo e tamanho).
  * `list_drives` e `volume_space` nativos (Linux `/proc/mounts` + `statvfs`; Windows `GetLogicalDriveStringsW` + `GetDiskFreeSpaceExW`).
  * `directories` populando a Sidebar (*Home*, *Desktop*, *Downloads*, *Documents*, *Pictures*).
* Implementar **Abas**, **Histórico Voltar/Avançar**, **Breadcrumb Unificado + `Ctrl+L`**, **Seleção simples/múltipla** e **Grid View + Detail Table virtualizados**.
* **Critério de aceite**: Navegar pastas reais no Linux e Windows com abas múltiplas, abrir arquivos com app padrão do SO e alternar entre Grid e Detail Table a 60+ FPS em pasta com 10.000 arquivos.

### Fase P2 — Telemetria Assíncrona, Busca & QuickLook
* Integrar `notify-debouncer-mini` para atualizar o diretório automaticamente quando arquivos mudarem no disco.
* Integrar `fm-telemetry`: branch Git (`gix`) no breadcrumb/action bar, contagem rasa de itens das pastas visíveis, `SHA-256` cancelável no Inspector (`Alt+I`) e status bar.
* Integrar `nucleo` na barra `Ctrl+K` (filtro instantâneo + busca profunda em subpastas).
* Implementar **QuickLook (`Space`)** com `image` + `syntect` e cache de thumbnails (`fast_image_resize` + LRU).
* **Critério de aceite**: Abrir repositório Rust real, ver badge `git:(main)`, pressionar `Space` em `main.rs` com syntax highlight instantâneo e cancelar cálculo de hash ao trocar de arquivo rapidamente.

### Fase P3 — Operações de Arquivo & Otimização Nativa de Hot Paths
* Implementar `CopyEngine` (Copiar, Recortar/Mover, Renomear com F2, Nova Pasta, Enviar para Lixeira via `trash`) com barra/popover de progresso e detecção de conflitos.
* Implementar **Column View (Miller Columns)** e persistência de preferências (`toml`).
* Substituir hot paths de `fm-fs` por `rustix` (`getdents64`/`statx`/`copy_file_range`) no Linux e `windows-rs` (`FindFirstFileExW`/`CopyFileExW`) no Windows.
* **Critério de aceite**: Operações completas de gerenciamento de arquivos sem travar a UI, passando nos testes de `ubuntu-latest` e `windows-latest`.
