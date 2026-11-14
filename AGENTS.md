# AGENTS.md — Guia de Engenharia para Agentes

Este repositório implementa o **Vagnarok File Manager**, um gerenciador de arquivos desktop nativo para **Linux e Windows**, escrito **100% em Rust** com **Iced + wgpu**.

Antes de propor alterações ou escrever código, leia os documentos abaixo na ordem indicada.

---

## 1. Mapa do Spec Kit (`specs/`)

| Arquivo | Quando Ler | Conteúdo |
| :--- | :--- | :--- |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Sempre (início) | Visão geral, grafo de crates, fluxo assíncrono e decisões Linux/Windows |
| [specs/00-spec-index.md](specs/00-spec-index.md) | Sempre (início) | Índice do Spec Kit e critérios globais de aceitação |
| [specs/01-mockup-reconciliation.md](specs/01-mockup-reconciliation.md) | Ao tocar em UI/UX | Resolução oficial de conflitos entre `DESIGN.md`, `code.html` e `screen.png` |
| [specs/02-design-system-tokens.md](specs/02-design-system-tokens.md) | Ao estilizar/criar temas | Tokens unificados (`ObsidianFlow` Dark e `AetherLight` Light) para `fm-theme` |
| [specs/03-architecture-contracts.md](specs/03-architecture-contracts.md) | Ao codar backend/estado | Traits (`VfsProvider`), structs de domínio, mensagens e cancelamento |
| [specs/04-ui-components-and-views.md](specs/04-ui-components-and-views.md) | Ao criar telas/widgets | Especificação de layout 3 painéis, viewports virtualizados, QuickLook e atalhos |
| [specs/05-tech-stack-prototype.md](specs/05-tech-stack-prototype.md) | Ao iniciar/expandir projeto | Stack exata de crates Rust, configuração Cargo e roadmap P0→P3 |

---

## 2. Regras Invioláveis de Arquitetura

1. **100% Rust Nativo**: Nunca introduzir WebViews, Tauri, Electron, bindings C++ pesados ou scripts Node/Python em runtime.
2. **Thread de UI Sagrada**: A thread do Iced (`update` e `view`) **nunca** deve executar I/O de disco, cálculo de hash, parsing Git, leitura de `/proc/mounts` ou decodificação de imagens de forma síncrona. Tudo vai para workers `tokio`/`rayon` e retorna via `Subscription` / `Task`.
3. **Fronteiras de Crates**:
   * `fm-core` não depende de `iced`, `tokio` (além de traits/tipos mínimos se necessário — preferir abstrações puras) nem de APIs específicas de SO.
   * `#[cfg(target_os = "linux")]` e `#[cfg(target_os = "windows")]` devem ficar confinados em `fm-fs` (e na adaptação de janela/CSD em `fm-app`).
4. **Paths Reais vs. Exibição**: Armazene e manipule caminhos reais como `PathBuf` / `OsString`. Use `.to_string_lossy()` **apenas** na camada de renderização.
5. **Windows File Locking**: Toda leitura em background (`fm-telemetry`, `fm-preview`) deve abrir arquivos com compartilhamento permissivo (`FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE` no Windows) e liberar handles imediatamente quando o `CancellationToken` for acionado.
6. **Sem `unwrap()` em I/O**: Erros de permissão, symlinks quebrados, arquivos em uso ou dispositivos desconectados são esperados em um gerenciador de arquivos e devem ser tratados com `Result` e feedback limpo na UI.
7. **Virtualização Obrigatória**: Listas e grids de diretório nunca devem instanciar widgets para todos os itens de uma pasta grande; renderizar somente a janela visível no scroll (+ overscan mínimo).

---

## 3. Verificação Mínima Após Mudanças

* `cargo check --workspace --all-targets`
* `cargo clippy --workspace --all-targets -- -D warnings`
* `cargo test --workspace`
* Ao modificar código condicional de SO, validar cross-check se disponível (`cargo check --target x86_64-pc-windows-gnu` ou `x86_64-unknown-linux-gnu`) e manter testes unitários para os dois alvos.
