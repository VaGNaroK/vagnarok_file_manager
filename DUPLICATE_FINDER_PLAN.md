# Plano de Implementação — **Buscador de Arquivos Duplicados (MD5 com Múltiplos Níveis de Confirmação)**

Documento de análise técnica e plano arquitetural para adicionar ao **Vagnarok File Manager** uma ferramenta nativa de **detecção e limpeza segura de arquivos duplicados**, escrita **100% em Rust puro**, com motor progressivo em funil baseado em **hash MD5** e **múltiplos níveis configuráveis de confirmação** (desde triagem rápida até confirmação absoluta zero-colisão), alinhada às regras do [AGENTS.md](AGENTS.md).

---

## 1. Objetivos Funcionais

1. Permitir localizar arquivos duplicados dentro de um diretório (com opção recursiva em subpastas) a partir de:
   - Menu de contexto do diretório / área vazia (**"Buscar arquivos duplicados..."**);
   - Atalho de teclado ou botão na barra de ferramentas do canvas.
2. Oferecer **níveis progressivos de confirmação da duplicidade** selecionáveis pelo usuário antes de iniciar a busca:
   - **Nível 1 — Rápido (Triagem)**: Tamanho exato + **MD5 Parcial** (cabeçalho + meio + cauda do arquivo).
   - **Nível 2 — Padrão (Recomendado)**: Tamanho exato + MD5 Parcial + **MD5 Completo** (`128-bit` em streaming).
   - **Nível 3 — Alta Segurança (Zero Colisão via Hash)**: Tamanho + MD5 Parcial + MD5 Completo + **SHA-256 Completo** (reaproveitando `sha2` já existente no `fm-telemetry`).
   - **Nível 4 — Confirmação Absoluta (Física)**: Tamanho + MD5 Parcial + MD5 Completo + **Comparação Byte a Byte** direta entre arquivos empatados.
3. Evitar falsos positivos clássicos em sistemas de arquivos:
   - Detectar **hardlinks** (mesmo arquivo físico em disco via `(dev, ino)` no Linux e File ID no Windows) para ignorá-los por padrão ou sinalizá-los claramente;
   - Ignorar **symlinks**, diretórios virtuais (`trash://`, `ArchiveVfs`) e arquivos especiais (FIFOs, sockets).
4. Permitir **minimizar a janela durante buscas demoradas** (em coleções de dezenas/centenas de GB) para continuar navegando e abrindo abas normalmente, com barra de progresso em tempo real no rodapé.
5. Fornecer **seleção inteligente em 1 clique** (manter mais antigo, manter mais recente, manter caminho mais curto) e remoção prioritária via **Lixeira (`move_to_trash`)**, protegendo contra exclusão acidental de todas as cópias de um grupo.

---

## 2. Arquitetura do Motor em Funil (`Nível 1 → Nível 4`)

Em vez de calcular hash completo de todos os arquivos de forma ingênua, o motor funciona como um **funil de eliminação em estágios**. Cada fase descarta a maioria absoluta dos arquivos antes de abrir ou ler conteúdos pesados:

```text
  [Arquivos coletados no escopo]
               |
               v
  FASE 1: Tamanho exato (u64) + Filtro de Hardlink
     └─> Tamanhos únicos (1 arquivo) são descartados com 0 bytes lidos
               |
               v
  FASE 2: MD5 Parcial (8 KiB início + 8 KiB meio + 8 KiB fim)
     ├─> Curto-circuito: se file_size <= 32 KiB, calcula MD5 Completo direto
     │   em 1 leitura sequencial e guarda `full_md5` no candidato (pula Fase 3)
     └─> Arquivos > 32 KiB com cabeçalho/meio/fim diferentes caem aqui
         (evita ler vídeos/ISOs de vários GBs e evita sobreposição de offsets)
               |
               v
  FASE 3: MD5 Completo (RFC 1321 em chunks streaming de 64 KiB)
     └─> Roda apenas nos candidatos empatados na Fase 2 sem `full_md5`
         (se nível escolhido = Padrão, confirma o grupo aqui)
               |
               v
  FASE 4 (Opcional): Confirmação Estrita Zero-Colisão (bypass total do cache)
     ├─> Modo SHA-256: recalcula SHA-256 direto do disco nos grupos empatados no MD5
     └─> Modo Byte-a-Byte: comparação pareada 2-a-2 contra âncora (`<= 2` handles/worker,
         2 buffers reutilizáveis de 64 KiB) particionando subgrupos em caso de divergência
```

### Por que oferecer Nível 4 além do MD5?
Embora colisões acidentais de MD5 em arquivos cotidianos sejam improváveis, o algoritmo MD5 possui vulnerabilidades conhecidas de colisão desde 2004 (dois arquivos diferentes podem ser forjados com o mesmo MD5). Combinar a velocidade do **funil MD5** como filtro principal com a confirmação final **SHA-256** (já presente no workspace) ou **Byte a Byte** garante precisão de 100% antes de apagar dados do usuário.

### Escolha de Implementação do MD5 & Build Offline (`Flatpak --offline --locked`)
- Verificado em `Cargo.lock`: já existem `sha2`, `digest`, `rayon`, `ignore` e `redb`, mas **não** há crate `md-5` no lockfile atual.
- **Duas opções arquiteturais**:
  1. **Implementação pura-Rust RFC 1321 em `fm-telemetry/src/md5.rs`** (~140 linhas, sem dependências externas, validada contra vetores oficiais da RFC 1321): mantém `Cargo.lock` e cache offline Flatpak intactos sem baixar novas crates. *(Recomendado se quiser zero impacto no vendor offline).*
  2. **Crate `md-5` (RustCrypto)**: integra com a mesma trait `digest::Digest` já usada por `sha2`, exigindo atualizar `Cargo.lock` e cache offline do Flatpak.

---

## 3. Regras Arquiteturais Aplicáveis (`AGENTS.md`)

- **100% Rust Nativo**: nenhuma chamada a `md5sum`, `fdupes`, `rdfind` ou bindings C/C++.
- **Thread de UI Sagrada**: varredura, agrupamento, leitura de blocos parciais, MD5, SHA-256 e comparação byte a byte rodam exclusivamente em `tokio::task::spawn_blocking` / pool `rayon` limitado, comunicando progresso e grupos encontrados via canal `mpsc`.
- **Windows File Locking (`open_shared_read`)**: toda abertura de arquivo reutiliza `crates/fm-telemetry/src/open_shared.rs` (`FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`), permitindo ler arquivos abertos em outros programas sem bloquear nem falhar, e liberando handles imediatamente quando o `CancellationToken` for acionado.
- **Fronteiras de Crates**:
  - `fm-core`: structs puras, enums de nível de confirmação, lógica de seleção inteligente em memória e chaves Fluent.
  - Identidade física de hardlink (`#[cfg(target_os = "linux")]` `dev`/`ino` vs `#[cfg(target_os = "windows")]` `file_id` via APIs já habilitadas no workspace) confinada exclusivamente em `fm-fs` ou submódulo `#[cfg]` de `fm-telemetry`.
- **Paths Reais vs Exibição**: manipular caminhos reais como `PathBuf`, usando `.to_string_lossy()` apenas ao renderizar na UI.
- **Virtualização Obrigatória**: listas com centenas/milhares de grupos e cópias nunca devem instanciar widgets para todos os resultados simultaneamente.

---

## 4. UX / UI (Design System `ObsidianFlow` / `AetherLight`)

### 4.1 Pontos de Entrada e Proteções
- Item no menu de contexto de fundo (`ContextMenuKind::Background`) e de pasta (`ContextMenuKind::Entry` quando `is_dir == true`): **"Buscar arquivos duplicados..."**.
- Bloquear com aviso limpo (`status_notice`) se o diretório ativo estiver na Lixeira virtual (`is_virtual_trash_path`) ou dentro de um arquivo compactado (`ArchiveVfs`).

### 4.2 Wireframe Conceitual da Janela (Arrastável, Redimensionável e Minimizável)

```text
+-------------------------------------------------------------------------------------+
| [Buscar Duplicados]  /home/user/Downloads                        [_]  [x]           |
+-------------------------------------------------------------------------------------+
| Pasta: [ /home/user/Downloads ]   Confirmação: [ Padrão (MD5 Completo)          v ] |
| Opções: [x] Subpastas  [ ] Ocultos  [x] Ignorar vazios (0 B)  [x] Ignorar hardlinks |
| Tamanho mín.: [ 1 KB v ]                                [ Iniciar Busca / Parar ]   |
+-------------------------------------------------------------------------------------+
| Seleção rápida: [Manter mais antigo] [Manter mais recente] [Caminho + curto] [Limpar]|
+-------------------------------------------------------------------------------------+
| RESULTADOS (Virtualizado — altura fixa por linha)                                   |
|                                                                                     |
| ▼ Grupo #1 • 3 cópias • 240,0 MB recuperáveis • MD5: 9e107d..b4 (MD5 Completo ✓)    |
|   [ ] /Downloads/ubuntu-24.04.iso          4,80 GB • 12/03/2026 (Mais antigo)       |
|   [x] /Downloads/Backup/ubuntu-24.04.iso   4,80 GB • 18/05/2026                     |
|   [x] /Downloads/Temp/ubuntu (1).iso       4,80 GB • 02/06/2026                     |
|                                                                                     |
| ▼ Grupo #2 • 2 cópias • 14,2 MB recuperáveis • MD5: a1b2c3..09 (Byte-a-Byte ✓)      |
|   [ ] /Downloads/Fotos/IMG_0042.JPG       14,2 MB • 01/04/2026                      |
|   [x] /Downloads/WhatsApp/IMG_0042.JPG    14,2 MB • 04/04/2026                      |
+-------------------------------------------------------------------------------------+
| Estágio 3/3: MD5 Completo (42/58) • 2 grupos • 4,81 GB recuperáveis                 |
|                      [Excluir perm.]  [ Mover 3 marcados para Lixeira (4,81 GB) ]   |
+-------------------------------------------------------------------------------------+
```

### 4.3 Níveis no Dropdown de Confirmação
1. **Rápido (Tamanho + MD5 Parcial)** — *Triagem rápida sem leitura completa; alerta na UI indicando que apenas trechos foram comparados e desabilitando exclusão permanente em 1 clique.*
2. **Padrão (Tamanho + MD5 Parcial + MD5 Completo)** — *Padrão equilibrado com streaming completo.*
3. **Seguro (MD5 + SHA-256 Completo)** — *Dupla validação criptográfica nos grupos encontrados.*
4. **Máximo (MD5 + Comparação Byte a Byte)** — *Verificação física exata bit a bit para grupos empatados.*

### 4.4 Segurança na Seleção e Exclusão
- **Aviso anti-apagar-tudo**: se todas as cópias de um grupo forem marcadas `[x]`, o contador de rodapé destaca um alerta visual em `status_warning` / `status_error` (*"Atenção: todas as cópias de 1 grupo estão marcadas!"*) antes de permitir confirmar.
- **Lixeira como ação principal**: botão primário sempre usa `VfsProvider::move_to_trash` (reversível); exclusão permanente fica em botão secundário discreto exigindo confirmação extra.
- **Minimização em segundo plano**: durante uma busca pesada em TB de arquivos, o botão **`[_]`** recolhe o overlay (`opaque`), liberando a janela e abas do gerenciador enquanto uma pílula no rodapé exibe `"Duplicados: Fase 3/3 (68%) [Restaurar] [Cancelar]"`.

### 4.5 Modelo de Design Aprovado (Referência de Implementação GUI)
- **Protótipo interativo de referência**: [specs/mockups/duplicate_finder_mockup.html](specs/mockups/duplicate_finder_mockup.html) (aderente às paletas `ObsidianFlow` Dark e `AetherLight` Light de `fm-theme`).
- **Diretrizes visuais obrigatórias para a implementação em `fm-app`**:
  1. **Cabeçalho da janela modal**: badge destacado **`MD5`** (`accent_primary`), título **"Buscar Arquivos Duplicados"**, caminho raiz em fonte monoespaçada e botões de janela **`─` (Minimizar)** e **`✕` (Fechar)**.
  2. **Painel de configuração e filtros**:
     - Dropdown **"Nível de confirmação da duplicidade"** com os 4 níveis numerados (`1. Rápido`, `2. Padrão`, `3. Seguro`, `4. Máximo`), dropdown **"Tamanho mínimo"** (`Qualquer (≥ 1 B)`, `≥ 1 KB`, `≥ 1 MB`) e botão primário **`Reescanear`** / **`Iniciar Busca`**;
     - Checkboxes **`Incluir subpastas`**, **`Incluir ocultos`**, **`Ignorar vazios (0 B)`** e **`Ignorar hardlinks`** com badge de estágio do funil à direita (ex.: `Estágio 3/3: MD5 Completo`);
     - **Banner âmbar condicional** no modo **`1. Rápido`** alertando que apenas trechos iniciais/médios/finais são comparados e mantendo o botão **Excluir permanentemente** desabilitado.
  3. **Barra de seleção rápida**: botões **`Manter mais antigo`**, **`Manter mais recente`**, **`Caminho mais curto`** e **`Limpar`** à esquerda; contador dinâmico `N marcados (X,XX GB)` em destaque à direita.
  4. **Cards de grupo na lista virtualizada**:
     - Cabeçalho em `surface_high`: `Grupo #N`, contagem e tamanho (`3 cópias • 4,80 GB cada`), badge semântico de verificação (`MD5 Parcial (aprox.)` em âmbar no nível Rápido; `MD5 Completo ✓`, `MD5 + SHA-256 ✓` ou `Byte-a-Byte 100% ✓` em verde nos níveis 2–4) e hash resumido à direita (`md5: ...`, incluindo `sha256: ...` no nível Seguro);
     - Linhas de arquivo em `surface_low`: checkbox de seleção, caminho truncável em fonte monoespaçada, tags inline **`Mais antigo`** e **`Hardlink`**, além de tamanho + data alinhados à direita.
  5. **Rodapé da modal e proteção contra perda de dados**:
     - Banner vermelho condicional caso todas as cópias visíveis de pelo menos 1 grupo sejam marcadas (`"Atenção: você marcou TODAS as cópias de pelo menos 1 grupo! Desmarque 1 exemplar para não perder o arquivo original."`), bloqueando ambos os botões de remoção;
     - Resumo à esquerda (`N grupos encontrados • X,XX GB recuperáveis`), ação secundária **`Excluir permanentemente`** e ação primária **`Mover para Lixeira (N)`**.
  6. **Estado minimizado no rodapé principal (`footer.rs`)**:
     - O botão **`─`** oculta o overlay liberando abas, sidebar e grid principal, exibindo a pílula persistente **`Duplicados: <Estágio> • <N> grupos`** com botões **`Restaurar`** e **`✕`**.

---

## 5. Arquitetura por Crate

### 5.1 `fm-core` — Domínio Puro, Regras de Seleção e I18n
Novo módulo `crates/fm-core/src/duplicates.rs` (sem I/O, sem `iced`, sem `tokio`):
- **`DuplicateConfirmLevel`**:
  - `QuickPartialMd5`
  - `StandardFullMd5` *(default)*
  - `StrictMd5AndSha256`
  - `ParanoidMd5AndByteCompare`
- **`DuplicateScanOptions`**:
  - `roots: Vec<PathBuf>`
  - `confirm_level: DuplicateConfirmLevel`
  - `recursive: bool`
  - `include_hidden: bool`
  - `ignore_empty_files: bool` (default `true`)
  - `ignore_hardlinks: bool` (default `true`)
  - `min_size_bytes: u64` (default `1`)
- **`DuplicateFileItem`**:
  - `path: PathBuf` *(chave primária de seleção e remoção — não usar `EntryId`)*, `relative_display: CompactString`, `size_bytes: u64`, `modified_nanos: Option<u128>`, `modified_secs: Option<i64>`, `file_identity: Option<(u64, u64)>`, `is_hardlink_sibling: bool`, `selected_for_delete: bool`
- **`DuplicateGroup`**:
  - `group_id: u64`, `size_bytes: u64`, `md5_hex: CompactString`, `sha256_prefix: Option<CompactString>`, `verified_level: DuplicateConfirmLevel`, `items: Vec<DuplicateFileItem>`
  - Métodos puros:
    - `reclaimable_bytes()`: calcula economia real agrupando por `file_identity` quando presente (apagar apenas 1 de 2 hardlinks do mesmo inode libera `0 B`);
    - `all_items_selected()`, `selected_count()`.
- **Estratégias puras de seleção inteligente**:
  - `KeepStrategy`: `KeepOldest`, `KeepNewest`, `KeepShortestPath`, `KeepFirstInScan`, `ClearAll`.
  - Função determinística `apply_keep_strategy(groups: &mut [DuplicateGroup], strategy: KeepStrategy)` (sempre deixa ao menos 1 cópia desmarcada por grupo, usando `path` como desempate em timestamps idênticos).
- **Chaves Fluent (`.ftl` em 6 idiomas + `Strings`)**: labels dos níveis de confirmação, estratégias de seleção, estágios de progresso, alertas de segurança e lixeira.

### 5.2 `fm-telemetry` — Motor MD5, Funil Paralelo e Detecção de Hardlinks
Dependências em `crates/fm-telemetry/Cargo.toml` (todas já presentes em `[workspace.dependencies]` e `Cargo.lock`, mantendo build `--offline --locked` intacto):
- Adicionar `ignore.workspace = true`, `rayon.workspace = true` e, sob `[target.'cfg(windows)'.dependencies]`, `windows.workspace = true` (a feature `Win32_Storage_FileSystem` já está habilitada no `Cargo.toml` raiz).
- Expor `compute_sha256_blocking` em `crates/fm-telemetry/src/hash.rs` como `pub(crate)` para reaproveitamento síncrono direto dentro dos workers do Estágio 4 sem `spawn_blocking` aninhados.

Novos módulos em `crates/fm-telemetry/src/`:
1. **`md5.rs`**:
   - Implementação RFC 1321 pura em Rust (streaming `Md5Hasher::update` + `finalize` com buffer `64 KiB`) e funções:
     - `compute_partial_md5_blocking(path, file_size, cancel) -> Option<[u8; 16]>` (usada apenas quando `file_size > 32_768`: lê `8 KiB` início + `8 KiB` meio + `8 KiB` fim via `Seek`, usando `open_shared_read`, sem sobreposição de offsets).
     - `compute_full_md5_blocking(path, cancel) -> Option<[u8; 16]>` (usada tanto no curto-circuito `file_size <= 32_768` quanto no Estágio 3).
2. **`hardlink.rs`** (com `#[cfg(target_os)]` confinado):
   - Retorna `Option<FileIdentity>` (`(u64, u64)` = `(dev, ino)` no Linux; `(dwVolumeSerialNumber, file_index)` no Windows via `open_shared_read` + `GetFileInformationByHandle`).
   - Quando `ignore_hardlinks == true`, apenas 1 caminho por `FileIdentity` entra no pipeline; caso `false`, mantém todos preservando `file_identity` e marcando `is_hardlink_sibling = true` na UI.
3. **Cache `redb` (`meta_cache.rs` — tabela `md5_meta_v1`)**:
   - Chave codificada com `path.as_os_str().as_encoded_bytes()` (sem `to_string_lossy()`, seguro para paths não-UTF8 no Linux) + `size_bytes` + `mtime_nanos` + validador de integridade por SO:
     - **Linux**: combinar `(meta.ctime(), meta.ctime_nsec())` (`(ctime as i128) * 1_000_000_000 + ctime_nsec as i128` — nunca usar `ctime_nsec()` isolado);
     - **Windows**: `file_attributes` junto de `(size_bytes, mtime_nanos)`.
   - Gravação via método em lote **`put_md5_batch(&self, entries: &[...])`** em uma única transação `write_txn` (ou em chunks de ~500 itens) ao final do Estágio 3, evitando milhares de `fsync`s individuais por arquivo.
   - **Bypass total nos níveis estritos**: em `StrictMd5AndSha256` e `ParanoidMd5AndByteCompare`, ignorar completamente a leitura do cache (nem usar como agrupador preliminar, evitando falsos negativos por cache obsoleto), lendo sempre do disco real.
4. **`duplicates.rs`**:
   - Pool dedicado isolado: instanciar `rayon::ThreadPoolBuilder::new().num_threads(available_parallelism().clamp(1, 4))` — **nunca** usar `par_iter()` no pool global do `rayon`.
   - Orquestrador `run_duplicate_scan(options, cache_opt, cancel, tx)`:
     - **Estágio 1 (Walk + Metadata)**: percorre diretório(s) com `WalkBuilder` (`standard_filters(false)`, `follow_links(false)`, `hidden(!options.include_hidden)`, `threads(worker_count)`), filtrando pseudo-filesystems do Linux (`/proc`, `/sys`, `/dev`, `/run`, reutilizando a guarda de `stats.rs`) e retornando `WalkState::Quit` quando `cancel.is_cancelled()`. Aplica `min_size_bytes` / `ignore_empty_files`, agrupa em `HashMap<u64, Vec<Candidate>>` e descarta buckets com `len < 2`.
     - **Estágio 2 (MD5 Parcial + Curto-Circuito `<= 32 KiB`)**: processa candidatos no pool isolado respeitando `cancel`:
       - Se `file_size <= 32_768`, pula o MD5 Parcial, calcula o **MD5 Completo** em 1 única leitura sequencial e armazena em `candidate.full_md5 = Some(digest)` (sinalizando `verified_level = StandardFullMd5` mesmo quando o scan foi iniciado em modo Rápido);
       - Se `file_size > 32_768`, calcula `compute_partial_md5_blocking`;
       - Descarta hashes únicos (`len < 2`).
     - **Estágio 3 (MD5 Completo + Cache `redb` em Lote)**:
       - Se o nível for `QuickPartialMd5`, já emite grupos aqui e encerra;
       - Caso contrário, reaproveita `candidate.full_md5` já calculado no curto-circuito sem reabrir o arquivo; para os demais (`> 32 KiB`), consulta `md5_meta_v1` (apenas no nível `StandardFullMd5`) ou calcula `compute_full_md5_blocking` no pool isolado checando `cancel` a cada `64 KiB`, gravando novos hashes em lote via `put_md5_batch`.
     - **Estágio 4 (Confirmação Extra Zero-Colisão)**:
       - `StrictMd5AndSha256`: calcula `compute_sha256_blocking` direto do disco nos grupos empatados no MD5 e subdivide caso algum par divirja.
       - `ParanoidMd5AndByteCompare`: comparação **pareada 2-a-2 contra âncora** mantendo no máximo **2 file handles abertos por worker** e **2 buffers pré-alocados reutilizáveis de `64 KiB`** (zero alocações no heap dentro do loop). Algoritmo de particionamento: mantém `subgroups: Vec<Vec<Candidate>>`; cada candidato `c` é comparado com o âncora `sg[0]` de cada subgrupo existente — se igual, entra em `sg`; se divergir de todos, inicia novo subgrupo `vec![c]`; ao final, mantém subgrupos com `len >= 2`. Erros de I/O descartam apenas o arquivo problemático (promovendo o próximo âncora se necessário) sem abortar o grupo inteiro.
     - Ordena grupos por **maior espaço recuperável (`reclaimable_bytes()`) decrescente** e envia `DuplicateScanEvent` (`Progress` com throttling de ~60–80 ms, `Completed(Vec<DuplicateGroup>)`, `Cancelled`, `Failed`).

### 5.3 `fm-fs` — Remoção Segura em Lote e Revalidação Anti-Dessincronização
- Como o usuário pode minimizar o modal e editar/mover/apagar arquivos manualmente em outras abas antes de confirmar a exclusão, o fluxo antes de chamar `VfsProvider::move_to_trash(&paths)` ou `VfsProvider::delete_permanent(&paths)` deve executar uma **revalidação assíncrona por grupo** (`tokio::fs::symlink_metadata`):
  1. Para cada `PathBuf` marcado: confirmar que o caminho ainda existe, é arquivo regular (`meta.file_type().is_file()` e `!is_symlink()`, evitando symlink-swap TOCTOU), mantém `meta.len() == item.size_bytes` e `mtime` compatível com o registrado no scan. Itens ausentes ou alterados externamente são abortados individualmente, removidos da UI e reportados via `status_notice`.
  2. **Guarda da última cópia sobrevivente**: verificar também se **ao menos 1 cópia NÃO marcada do mesmo grupo ainda existe em disco** (`symlink_metadata` válido). Se todas as cópias não marcadas tiverem sido removidas externamente enquanto a janela estava minimizada, abortar a exclusão daquele grupo inteiro e exibir aviso explícito ao usuário.
- Após a exclusão bem-sucedida dos caminhos validados, remover os itens apagados de `DuplicateFinderState.groups` (descartando grupos com `< 2` itens) e atualizar as abas abertas afetadas.

### 5.4 `fm-app` — Estado, Minimização e Virtualização da Lista
1. **Estado no `App` (`crates/fm-app/src/duplicate_finder.rs`)**:
   - `duplicate_finder: Option<DuplicateFinderState>` (desacoplado de `active_tab`, com `minimized: bool`, `modal_offset`, `modal_size`, `dragging_modal`, `resizing_modal`, `cancel: Option<CancellationToken>`).
   - Seleção e remoção indexadas estritamente por `PathBuf` (nunca `EntryId`).
   - `flat_rows: Vec<DuplicateListRow>` reconstruído apenas quando resultados chegam ou um grupo é expandido/recolhido (`GroupHeader(group_idx)` | `FileItem(group_idx, item_idx)`), com `relative_display` pré-computado fora do `view()`.
2. **Lista Virtualizada (`crates/fm-app/src/views/duplicate_finder.rs`)**:
   - Como `flat_rows` tem altura fixa (`ROW_HEIGHT = 32.0`), o `scrollable` calcula `first_visible .. end_visible` a partir de `scroll_y` e `viewport_height` + overscan mínimo (`Space::new().height(top_pad)` + apenas linhas visíveis + `bottom_pad`), mantendo 60 fps mesmo com 20.000 arquivos duplicados.
3. **Minimizar e Rodapé**:
   - Quando `minimized == true` durante o scan, o overlay modal `opaque` sai da árvore de UI permitindo navegar e abrir abas livremente, exibindo pílula de progresso no `footer.rs` com **[Restaurar]** e **[Cancelar]**.

---

## 6. Riscos Técnicos e Mitigações

| Risco | Impacto | Mitigação |
| :--- | :--- | :--- |
| **Colisão teórica / forjada de MD5** | Perda de dados caso arquivos diferentes tenham mesmo MD5 | Disponibilizar **Nível 3 (`MD5 + SHA-256`)** e **Nível 4 (`MD5 + Byte a Byte`)** com bypass total de cache; sinalizar claramente no modo Rápido (`Parcial`) que a confirmação é aproximada e bloquear exclusão permanente. |
| **Cache `redb` obsoleto ou gargalo de `fsync`** | Falso positivo/negativo em arquivos editados com `mtime` preservado (ou FAT32) e lentidão em gravações unitárias | Validar `(ctime, ctime_nsec)` no Linux e `file_attributes` no Windows com chave `as_encoded_bytes()`; gravar em lote (`put_md5_batch`); bypass total nos Níveis 3 e 4. |
| **Exaustão de FDs (`EMFILE`) no Nível 4** | Falha ou *disk thrashing* em pastas com centenas de cópias do mesmo arquivo | Comparação pareada 2-a-2 contra âncora (`<= 2` handles abertos por worker + 2 buffers reutilizáveis de `64 KiB`), particionando subgrupos divergentes. |
| **Saturação de I/O em HDDs / SMB / NFS** | Lentidão do SO durante walk ou hashing paralelo | Isolar workers em `rayon::ThreadPool` local limitado a `clamp(1, 4)` threads e `WalkBuilder::threads(4)`; throttling de eventos de progresso (~60–80 ms). |
| **Hardlinks apagados achando que liberam espaço** | Espaço em disco não reduzido ou cálculo enganoso na UI | Detecção nativa `(dev, ino)` no Linux e `GetFileInformationByHandle` no Windows; filtro **"Ignorar hardlinks"** ativo por padrão; cálculo de `reclaimable_bytes()` consciente de `file_identity`. |
| **Dessincronização durante minimização / TOCTOU** | Exclusão de arquivo modificado externamente, symlink substituído ou da última cópia restante | Revalidar `PathBuf` via `symlink_metadata` (`is_file && !is_symlink`, tamanho e `mtime`) e confirmar existência de ao menos 1 cópia não marcada antes de despachar `move_to_trash`/`delete_permanent`. |
| **Pseudo-filesystems Linux ou `.gitignore` ocultando arquivos** | Travamento ao escanear `/` ou duplicatas ignoradas em projetos | Filtrar `/proc`, `/sys`, `/dev`, `/run` no walker e usar `WalkBuilder::standard_filters(false)` controlando apenas `hidden` e `follow_links(false)`. |
| **Milhares de grupos na UI** | Travamento ao abrir resultados | Lista achatada `flat_rows` 100% virtualizada com altura fixa de linha e `relative_display` pré-computado. |

---

## 7. Plano Faseado de Implementação

### Fase 1 — Domínio Puro e Localização (`fm-core`)
- Criar `crates/fm-core/src/duplicates.rs` (`DuplicateConfirmLevel`, `DuplicateScanOptions`, `DuplicateFileItem` sem `EntryId`, `DuplicateGroup` com `reclaimable_bytes()` ciente de `file_identity`, `KeepStrategy`, `apply_keep_strategy`).
- Adicionar chaves Fluent nos 6 arquivos `.ftl` e em `Strings`.
- Testes unitários de `apply_keep_strategy` (desempate determinístico, preservação de pelo menos 1 cópia, cálculo de bytes recuperáveis com e sem hardlinks).

### Fase 2 — Engine MD5, Hardlinks, Cache em Lote e Funil Multi-Estágios (`fm-telemetry`)
- Atualizar `crates/fm-telemetry/Cargo.toml` (`ignore`, `rayon`, `[target.'cfg(windows)'] windows` via workspace) e expor `compute_sha256_blocking` como `pub(crate)` em `hash.rs`.
- Implementar `crates/fm-telemetry/src/md5.rs` (RFC 1321 + testes contra vetores oficiais), `hardlink.rs` (Linux/Windows `#[cfg]`) e tabela `md5_meta_v1` com `put_md5_batch` + `(ctime, ctime_nsec)` no Linux / `as_encoded_bytes()` em `meta_cache.rs`.
- Implementar `crates/fm-telemetry/src/duplicates.rs` com pool `rayon` isolado (`<= 4` threads), `WalkBuilder::standard_filters(false)` + guarda de pseudo-filesystems, curto-circuito `<= 32 KiB`, bypass de cache nos níveis 3/4 e comparação pareada 2-a-2 com subgrupos no nível 4.
- Testes unitários completos: arquivos `<= 32 KiB` (curto-circuito), arquivos `> 32 KiB` com byte diferente no meio (pego no MD5 Completo) ou cabeçalho/fim (descartado no MD5 Parcial), hardlinks ignorados/exibidos, invalidação de cache `redb` + bypass nos níveis 3/4, cancelamento mid-scan e nível Paranóico 2-a-2 (incluindo particionamento de 2 subgrupos distintos no mesmo bucket).

### Fase 3 — UI Virtualizada, Minimização, Revalidação e Integração no Menu (`fm-app`)
- Adicionar **"Buscar arquivos duplicados..."** no menu de contexto de fundo e pastas (`crates/fm-app/src/views/modals.rs`).
- Criar `crates/fm-app/src/duplicate_finder.rs` e `crates/fm-app/src/views/duplicate_finder.rs` seguindo fielmente o modelo de design da **Seção 4.5** ([specs/mockups/duplicate_finder_mockup.html](specs/mockups/duplicate_finder_mockup.html)), com janela arrastável, redimensionável, minimizável e lista virtualizada de grupos/cópias.
- Conectar remoção segura (`move_to_trash` / `delete_permanent`) com revalidação assíncrona de `PathBuf` (`symlink_metadata`: arquivo regular, tamanho, `mtime` e guarda de pelo menos 1 cópia não marcada viva em disco), atualização instantânea dos grupos/abas e pílula de progresso no `footer.rs`.

---

## 8. Critérios de Verificação (após implementar)

- `cargo check --locked --offline --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- Cross-check Windows: `cargo check --target x86_64-pc-windows-gnu --workspace --all-targets`
- Teste manual no binário: escanear pasta com duplicatas reais + hardlinks + arquivos únicos grandes usando os diferentes níveis de confirmação, minimizar durante a busca, aplicar *"Manter mais antigo"* e mover duplicatas marcadas para a Lixeira.
