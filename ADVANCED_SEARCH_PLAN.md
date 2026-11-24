# Plano de Implementação: Busca Avançada e "Pesquisar contendo..."

Este documento registra o plano técnico para implementar a busca avançada no **Vagnarok File Manager**, sem dependência de indexação prévia e seguindo as regras arquiteturais de [AGENTS.md](AGENTS.md).

---

## 1. "Pesquisar contendo..." (Busca de texto dentro dos arquivos)

* **Acesso na UI (`crates/fm-app/src/views/inspector.rs` e menu de contexto)**:
  * Campo dedicado **"Pesquisar contendo..."** no modal de busca (`Ctrl+Shift+F`), separado do filtro por nome/caminho, com toggles rápidos para **Diferenciar maiúsculas (`Aa`)** e **Regex (`.*`)**.
  * Atalho opcional no menu de contexto de pastas (*"Pesquisar contendo..."*) para abrir o modal já focado na busca de conteúdo dentro da pasta selecionada.
  * Suporte também à sintaxe inline no campo de busca (ex.: `contendo:"TODO"` ou `content:"panic!"`).
* **Motor de leitura em streaming sem indexação (`crates/fm-index/src/search.rs`)**:
  * Execução 100% assíncrona em worker (`tokio` + `CancellationToken`), sem bloquear a thread de UI do Iced.
  * Leitura via buffer em streaming, **pulando arquivos binários** automaticamente (detecção de byte `NUL` nos primeiros 8 KB) e respeitando um limite configurável de tamanho por arquivo (ex.: 16 MB) para manter a busca ágil.
  * **Compatibilidade Windows**: abertura de arquivos com compartilhamento permissivo (`FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`) e liberação imediata de handles ao acionar o `CancellationToken`.
  * **Trechos nos resultados**: extensão de `SearchHit` com metadados opcionais (`size_bytes`, `modified_at`) e `content_matches` (`Vec<ContentMatchSnippet>` com `line_number` e `line_text`) para mostrar a linha e o trecho encontrado diretamente na lista de resultados.

---

## 2. Busca por Regex, Metadados, Tamanho e Data

* **Dependência no `fm-index` (`crates/fm-index/Cargo.toml`)**: habilitar `regex.workspace = true` (já presente no workspace, sem novas dependências externas).
* **Filtragem em cascata** em `crates/fm-index/src/search.rs` (sempre do filtro mais barato para o mais caro, antes de abrir o conteúdo dos arquivos):
  1. **Nome / caminho**: fuzzy (`nucleo-matcher`) ou Regex.
  2. **Tamanho e Data**: faixas de tamanho (`>10MB`, `<500KB`, intervalo min/máx) e data de modificação (presets como *Hoje*, *Últimos 7 dias*, *Últimos 30 dias*, ou intervalo).
  3. **Metadados**: filtro por extensão, categoria/MIME por magic bytes (imagens, áudio, vídeo, documentos, código) e metadados básicos (EXIF/ID3 quando aplicável).
  4. **Conteúdo ("Pesquisar contendo...")**: executado apenas nos arquivos que passaram pelos filtros anteriores.

---

## 3. Filtros Rápidos Sem Indexação Prévia (`Ctrl+K`)

* Evoluir `apply_tab_filter` em `crates/fm-app/src/app.rs` para interpretar tokens instantâneos em memória na aba ativa, sem tocar no disco:
  * Exemplos: `ext:rs`, `>5MB`, `<100KB`, `hoje`, `7d`, `type:image`, `regex:...`.
* Mantém o comportamento imediato ao digitar na barra de filtro da aba.

---

## 4. UI, Virtualização e Traduções (i18n)

* Exibição virtualizada dos resultados em `deep_search_modal` (`crates/fm-app/src/views/inspector.rs`) mostrando caminho relativo, tamanho, data e, quando houver busca de conteúdo, as linhas correspondentes com destaque do número da linha.
* Atualização de `spawn_deep_search_task` (`crates/fm-app/src/app.rs`) para enviar a nova estrutura de critérios de busca e cancelar buscas anteriores instantaneamente ao alterar filtros.
* Adição das novas strings nos 6 idiomas suportados pelo app.

---

## 5. Verificação Planejada

* Testes unitários em `fm-index` cobrindo **"Pesquisar contendo..."** (texto simples, case-sensitive, regex, skip de binários e cancelamento), além de filtros por tamanho, data, metadados e parser do filtro rápido (`Ctrl+K`).
* Validação completa:
  * `cargo check --workspace --all-targets`
  * `cargo clippy --workspace --all-targets -- -D warnings`
  * `cargo test --workspace`
  * Cross-check Linux/Windows (`x86_64-pc-windows-gnu` / `x86_64-unknown-linux-gnu`) se aplicável.
