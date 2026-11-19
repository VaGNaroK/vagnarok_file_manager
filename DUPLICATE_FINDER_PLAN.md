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
     └─> Arquivos > 24 KiB com cabeçalho/meio/fim diferentes caem aqui
         (evita ler vídeos/ISOs de vários GBs desnecessariamente)
               |
               v
  FASE 3: MD5 Completo (RFC 1321 em chunks streaming de 64 KiB)
     └─> Roda apenas nos candidatos empatados na Fase 2
         (se nível escolhido = Padrão, confirma o grupo aqui)
               |
               v
  FASE 4 (Opcional): Confirmação Estrita Zero-Colisão
     ├─> Modo SHA-256: recalcula/verifica SHA-256 nos grupos empatados no MD5
     └─> Modo Byte-a-Byte: leitura pareada chunk-a-chunk comparando bytes reais
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
  - `entry_id: EntryId`, `path: PathBuf`, `relative_display: CompactString`, `size_bytes: u64`, `modified_secs: Option<i64>`, `is_hardlink_sibling: bool`, `selected_for_delete: bool`
- **`DuplicateGroup`**:
  - `group_id: u64`, `size_bytes: u64`, `md5_hex: CompactString`, `sha256_prefix: Option<CompactString>`, `verified_level: DuplicateConfirmLevel`, `items: Vec<DuplicateFileItem>`
  - Métodos puros: `reclaimable_bytes()`, `all_items_selected()`, `selected_count()`.
- **Estratégias puras de seleção inteligente**:
  - `KeepStrategy`: `KeepOldest`, `KeepNewest`, `KeepShortestPath`, `KeepFirstInScan`, `ClearAll`.
  - Função determinística `apply_keep_strategy(groups: &mut [DuplicateGroup], strategy: KeepStrategy)` (sempre deixa ao menos 1 cópia desmarcada por grupo, usando `path` como desempate em timestamps idênticos).
- **Chaves Fluent (`.ftl` em 6 idiomas + `Strings`)**: labels dos níveis de confirmação, estratégias de seleção, estágios de progresso, alertas de segurança e lixeira.

### 5.2 `fm-telemetry` — Motor MD5, Funil Paralelo e Detecção de Hardlinks
Novos módulos em `crates/fm-telemetry/src/`:
1. **`md5.rs`**:
   - Implementação RFC 1321 pura em Rust (streaming `Md5Hasher::update` + `finalize` com buffer `64 KiB`) e funções:
     - `compute_partial_md5_blocking(path, file_size, cancel) -> Option<[u8; 16]>` (lê início + meio + fim via `Seek`, usando `open_shared_read`).
     - `compute_full_md5_blocking(path, cancel) -> Option<[u8; 16]>`.
2. **`hardlink.rs`** (ou helper com `#[cfg(target_os)]` confinado):
   - Retorna `Option<FileIdentity>` (`(u64, u64)` = `(dev, ino)` no Linux; volume serial + file index no Windows via `open_shared_read` + `GetFileInformationByHandle`).
   - Quando `ignore_hardlinks == true`, apenas 1 caminho por `FileIdentity` entra no pipeline; caso `false`, mantém todos marcando `is_hardlink_sibling = true` na UI.
3. **`duplicates.rs`**:
   - Orquestrador `run_duplicate_scan(options, cache_opt, cancel, tx)`:
     - **Estágio 1 (Walk + Metadata)**: percorre diretório(s) com `WalkBuilder` (`follow_links(false)`, `git_ignore(false)`, `hidden(!options.include_hidden)`), aplica `min_size_bytes` / `ignore_empty_files`, agrupa em `HashMap<u64, Vec<Candidate>>`, descarta buckets com `len < 2`.
     - **Estágio 2 (MD5 Parcial)**: processa candidatos restantes em pool `rayon` respeitando `cancel`, descartando hashes parciais únicos (`len < 2`).
     - **Estágio 3 (MD5 Completo + Cache `redb`)**:
       - Se o nível for `QuickPartialMd5`, já emite grupos aqui e encerra!
       - Caso contrário, consulta tabela opcional `md5_meta_v1` em `DiskMetadataCache` (validando `size` e `mtime_nanos`) para pular releitura em re-escaneamentos; calcula MD5 streaming em pool limitado (`4` threads em paralelo para equilibrar SSD e HDD) checando `cancel` a cada chunk de `64 KiB`.
     - **Estágio 4 (Confirmação Extra)**:
       - `StrictMd5AndSha256`: calcula `compute_sha256` (reutilizando `crate::hash`) apenas nos grupos empatados no MD5 e subdivide caso algum par divirja.
       - `ParanoidMd5AndByteCompare`: lê em paralelo grupos empatados comparando buffers de `64 KiB` simultaneamente até EOF.
     - Ordena grupos por **maior espaço recuperável (`size_bytes * (count - 1)`) decrescente** e envia `DuplicateScanEvent` (`Progress`, `Completed(Vec<DuplicateGroup>)`, `Cancelled`, `Failed`).

### 5.3 `fm-fs` — Remoção Segura em Lote
- Reutiliza diretamente `VfsProvider::move_to_trash(&paths)` e `VfsProvider::delete_permanent(&paths)`.
- Após a exclusão bem-sucedida, remove os caminhos apagados de `DuplicateFinderModalState.groups` (removendo grupos que ficarem com `< 2` itens) e atualiza as abas abertas afetadas.

### 5.4 `fm-app` — Estado, Minimização e Virtualização da Lista
1. **Estado no `App` (`crates/fm-app/src/duplicate_finder.rs`)**:
   - `duplicate_finder: Option<DuplicateFinderState>` (desacoplado de `active_tab`, com `minimized: bool`, `modal_offset`, `modal_size`, `dragging_modal`, `resizing_modal`, `cancel: Option<CancellationToken>`).
   - `flat_rows: Vec<DuplicateListRow>` reconstruído apenas quando resultados chegam ou um grupo é expandido/recolhido (`GroupHeader(group_idx)` | `FileItem(group_idx, item_idx)`).
2. **Lista Virtualizada (`crates/fm-app/src/views/duplicate_finder.rs`)**:
   - Como `flat_rows` tem altura fixa (`ROW_HEIGHT = 32.0`), o `scrollable` calcula `first_visible .. end_visible` a partir de `scroll_y` e `viewport_height` + overscan mínimo (`Space::new().height(top_pad)` + apenas linhas visíveis + `bottom_pad`), mantendo 60 fps mesmo com 20.000 arquivos duplicados.
3. **Minimizar e Rodapé**:
   - Quando `minimized == true` durante o scan, o overlay modal `opaque` sai da árvore de UI permitindo navegar e abrir abas livremente, exibindo pílula de progresso no `footer.rs` com **[Restaurar]** e **[Cancelar]**.

---

## 6. Riscos Técnicos e Mitigações

| Risco | Impacto | Mitigação |
| :--- | :--- | :--- |
| **Colisão teórica / forjada de MD5** | Perda de dados caso arquivos diferentes tenham mesmo MD5 | Disponibilizar **Nível 3 (`MD5 + SHA-256`)** e **Nível 4 (`MD5 + Byte a Byte`)**; sinalizar claramente no modo Rápido (`Parcial`) que a confirmação é aproximada. |
| **Hardlinks apagados achando que liberam espaço** | Espaço em disco não reduzido ou confusão em backups | Detecção nativa `(dev, ino)` no Linux e File ID no Windows; filtro **"Ignorar hardlinks"** ativo por padrão. |
| **Saturação de I/O em HDs mecânicos (HDD)** | Lentidão do sistema durante hash completo | Limitar paralelismo de leitura pesada (ex.: `4` leitores simultâneos em estágio completo vs tamanho livre na Fase 1) e fazer throttling de progresso para a UI (~60 ms). |
| **Arquivo alterado/removido durante o scan** | Erro ao abrir para confirmação ou exclusão | Usar `open_shared_read` e tratar `NotFound`/`PermissionDenied` graciosamente removendo o candidato sem abortar o scan inteiro (`AGENTS.md` #6 — zero `unwrap()`). |
| **Exclusão acidental de todas as cópias** | Usuário apagar último exemplar do arquivo | Ações automáticas preservam sempre ao menos 1 exemplar; aviso visual explícito se algum grupo ficar com 100% dos itens marcados; ação principal usa a **Lixeira**. |
| **Milhares de grupos na UI** | Travamento ao abrir resultados | Lista achatada `flat_rows` 100% virtualizada com altura fixa de linha. |

---

## 7. Plano Faseado de Implementação

### Fase 1 — Domínio Puro e Localização (`fm-core`)
- Criar `crates/fm-core/src/duplicates.rs` (`DuplicateConfirmLevel`, `DuplicateScanOptions`, `DuplicateGroup`, `KeepStrategy`, `apply_keep_strategy`).
- Adicionar chaves Fluent nos 6 arquivos `.ftl` e em `Strings`.
- Testes unitários de `apply_keep_strategy` (desempate determinístico, preservação de pelo menos 1 cópia, cálculo de bytes recuperáveis).

### Fase 2 — Engine MD5, Hardlinks e Funil Multi-Estágios (`fm-telemetry`)
- Implementar `crates/fm-telemetry/src/md5.rs` (RFC 1321 + testes contra vetores oficiais) e detecção de hardlinks Linux/Windows (`#[cfg]`).
- Implementar `crates/fm-telemetry/src/duplicates.rs` cobrindo os 4 níveis (`QuickPartialMd5`, `StandardFullMd5`, `StrictMd5AndSha256`, `ParanoidMd5AndByteCompare`) + cache opcional `redb`.
- Testes unitários completos: arquivos idênticos vs mesmo tamanho mas byte diferente no meio (pego no MD5 Completo), byte diferente no cabeçalho/fim (descartado no MD5 Parcial), hardlinks ignorados por padrão, cancelamento mid-scan e nível Paranóico byte a byte.

### Fase 3 — UI Virtualizada, Minimização e Integração no Menu (`fm-app`)
- Adicionar **"Buscar arquivos duplicados..."** no menu de contexto de fundo e pastas (`crates/fm-app/src/views/modals.rs`).
- Criar `crates/fm-app/src/duplicate_finder.rs` e `crates/fm-app/src/views/duplicate_finder.rs` com janela arrastável, redimensionável, minimizável e lista virtualizada de grupos/cópias.
- Conectar remoção segura (`move_to_trash` / `delete_permanent`) com atualização instantânea dos grupos e das abas abertas no `fm-app`, além de pílula de progresso no `footer.rs`.

---

## 8. Critérios de Verificação (após implementar)

- `cargo check --locked --offline --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- Cross-check Windows: `cargo check --target x86_64-pc-windows-gnu --workspace --all-targets`
- Teste manual no binário: escanear pasta com duplicatas reais + hardlinks + arquivos únicos grandes usando os diferentes níveis de confirmação, minimizar durante a busca, aplicar *"Manter mais antigo"* e mover duplicatas marcadas para a Lixeira.
