# Plano de Implementação — **Renomeação em Lote (`Bulk Rename`) Nativo**

Documento de análise de viabilidade e plano arquitetural para adicionar ao **Vagnarok File Manager** uma ferramenta modal nativa de **Renomeação em Lote** com **preview em tempo real**, suporte a **Regex**, **numeração sequencial** e extração de **metadados (EXIF / ID3)**, alinhada às regras de engenharia do repositório (`AGENTS.md`).

---

## 1. Veredito de Viabilidade na Arquitetura Atual

**Sim — é 100% viável e encaixa naturalmente na arquitetura atual do projeto**, sem quebrar nenhuma fronteira de crates ou regra de thread única de UI:

1. **100% Rust Nativo**: Toda a lógica de regras, compilação Regex, parser de templates, detecção de conflitos e leitura de headers EXIF/ID3 roda em Rust puro.
2. **Thread de UI Sagrada (`iced`)**:
   - O cálculo de preview de nomes sobre strings em memória é rápido (sub-milissegundo para dezenas/centenas de itens); para lotes muito grandes (ex.: > 500 arquivos) ou regex complexas, pode ser despachado em worker `rayon`/`tokio` com cancelamento por geração (`generation_id`).
   - **Toda leitura de disco para metadados (EXIF/ID3) e execução de `rename`** roda exclusivamente fora da thread do Iced (`fm-telemetry` e `fm-fs` via `tokio`), retornando mensagens incrementais para a UI.
3. **Dependências & Build Offline (Flatpak `--offline --locked`)**:
   - Verificado em `Cargo.lock`: as crates **`regex` (`1.13.1`)** e **`kamadak-exif` (`0.6.2`)** já existem no `Cargo.lock` atual do workspace (como dependências transitivas), o que evita baixar novos tarballs desconhecidos ao promover o uso para `fm-core`/`fm-telemetry`.
   - Já crates de áudio (`id3`, `lofty`, `symphonia`) **não** estão no `Cargo.lock`. Para manter builds offline simples (como em `packaging/flatpak/dev.vagnarok.FileManager.yml`, que executa `cargo build --release --offline --locked -p fm-app`), recomenda-se implementar em `fm-telemetry` um **leitor puro-Rust enxuto apenas de headers ID3v2.3/v2.4 (`TIT2`, `TPE1`, `TALB`, `TRCK`, `TDRC`/`TYER`) + fallback ID3v1** (~150–250 linhas), sem adicionar dependências externas novas. *(Se no futuro quiser suportar FLAC/Vorbis/M4A/Opus, pode-se avaliar `lofty` atualizando o `Cargo.lock` e o cache offline do Flatpak).*
4. **Windows File Locking**: Toda abertura de arquivo para leitura de EXIF/ID3 em `fm-telemetry` reutilizará `open_shared_read` (`crates/fm-telemetry/src/open_shared.rs`), garantindo `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE` no Windows e fechamento imediato via `CancellationToken`.
5. **Virtualização Obrigatória**: A tabela de preview ("Nome Atual → Novo Nome") renderizará somente a janela visível no scroll (+ overscan mínimo), permitindo abrir 5.000 arquivos sem instanciar 5.000 linhas de widgets.

---

## 2. Pontos de Entrada e UX / UI (Design System Vagnarok)

### 2.1 Como o usuário aciona

Atualmente:
- `crates/fm-app/src/app.rs` (`Message::OpenRenameDialog`, atalho `F2`) e `crates/fm-app/src/views/canvas.rs` (`if sel_count == 1`) habilitam renomeação apenas quando há **1 item** selecionado.
- **Novo comportamento**:
  - `sel_count == 1`: mantém o diálogo rápido de renomeação individual atual (ou opção secundária no menu para abrir o Bulk Rename caso queira usar regex/EXIF nele também).
  - `sel_count > 1`: `F2`, botão de **Renomear** na barra superior e item **Renomear em Lote...** no menu de contexto (`ContextMenuKind::Entry`) abrem diretamente o modal de **Bulk Rename** com todos os itens selecionados na ordem da aba ativa.
- **Proteção de VFS somente leitura**: Caso a aba atual esteja na Lixeira virtual (`is_virtual_trash_path`) ou dentro de um arquivo compactado (`ArchiveVfs` somente leitura), a ação deve bloquear a abertura com aviso limpo na barra de status (`status_notice`).

### 2.2 Janela Modal Overlay (Arrastável, Redimensionável e com Colisão Completa)

Seguindo o padrão recém-consolidado em `crates/fm-app/src/views/mod.rs` (`modal_overlay_backdrop_at` com `opaque`) e `crates/fm-app/src/app.rs` (`effective_properties_modal_rect`):
- O modal deve rodar sobre `modal_overlay_backdrop_at` + `opaque(...)` para **bloquear cliques na janela principal atrás**.
- Deve suportar **arrastar pela barra de título** e **redimensionar pelo canto inferior direito** (essencial para visualizar caminhos/nomes longos sem truncamento excessivo), com dimensões mínimas seguras (ex.: `740x520`).

### 2.3 Wireframe Conceitual (Tokens `ObsidianFlow` / `AetherLight`)

```text
+---------------------------------------------------------------------------------+
| [Renomear em Lote]  124 itens selecionados                                  [x] |
+---------------------------------------------------------------------------------+
| PRESETS / MODO:  [Substituir / Regex]  [Numeração]  [Metadados EXIF/ID3]  [Aa]  |
+------------------------------------+--------------------------------------------+
| REGRAS ATIVAS                      | PREVIEW EM TEMPO REAL                      |
|                                    |                                            |
| 1. Buscar / Substituir             |  Status | Atual            | Novo Nome     |
|    Buscar:   [ IMG_(\d+)_(\d+)  ]  |  -------+------------------+-------------- |
|    Subst.:   [ Viagem_$1        ]  |   [OK]  | IMG_2026_01.jpg  | Viagem_01.jpg |
|    [x] Usar Regex   [ ] Ign. Caso  |   [OK]  | IMG_2026_02.jpg  | Viagem_02.jpg |
|    Escopo: (o) Nome  ( ) Tudo      |   [!!]  | IMG_2026_03.jpg  | Viagem_01.jpg |
|                                    |           ^ Conflito: destino duplicado    |
| 2. Numeração & Template            |   [--]  | notas.txt        | (sem mudança) |
|    Padrão: [ {stem}_{n:03}      ]  |                                            |
|    Início: [ 1 ]  Passo: [ 1 ]     |                                            |
|    Ordenar por: [ Nome v ]         |  (Lista virtualizada: renderiza só visível)|
|                                    |                                            |
| 3. Tokens Rápidos (Clique p/ ins.) |                                            |
|    {stem} {ext} {n:03}             |                                            |
|    {exif:date} {exif:camera}       |                                            |
|    {id3:artist} {id3:title} {id3:#}|                                            |
|                                    |                                            |
| 4. Caixa & Limpeza                 |                                            |
|    Caixa: [ Sem alteração     v ]  |  Leitura EXIF/ID3: 124/124 (em cache)      |
+------------------------------------+--------------------------------------------+
| 120 alterados | 3 sem mudança | 1 conflito (bloqueando aplicar)                 |
|                                               [ Cancelar ]  [ Aplicar (120) ]   |
+---------------------------------------------------------------------------------+
```

---

## 3. Arquitetura por Crate

### 3.1 `fm-core` — Motor de Regras Puro & Planejador de Execução

Novo módulo `crates/fm-core/src/bulk_rename.rs` (sem `iced`, sem `tokio`, sem `#[cfg(target_os)]` para I/O):

1. **Estruturas de Entrada e Metadados**:
   - `BulkRenameInput`: `entry_id: EntryId`, `path: PathBuf`, `parent_dir: PathBuf`, `original_name: OsString`, `is_dir: bool`, `size_bytes: u64`, `modified_secs: Option<i64>`.
   - `BulkRenameMediaTags`: campos opcionais leves (`Option<CompactString>`) para:
     - EXIF: `date_taken` (ex.: `2026-08-14_15-30-22`), `date_ymd` (`2026-08-14`), `camera_model`, `width`, `height`.
     - ID3: `title`, `artist`, `album`, `track` (numérico para zero-padding), `year`.
2. **Pipeline Componível de Transformações**:
   - Ordem previsível de avaliação sobre cada item `(index, input, tags)`:
     1. **Separação segura de nome base (`stem`) e extensão (`ext`)**: respeitar arquivos sem extensão e diretórios (em pastas, `is_dir == true`, o ponto em `"v1.2.0"` normalmente pertence ao nome, com opção explícita se o usuário quiser tratar extensão).
     2. **Template / Numeração**: expansão de tokens (`{stem}`, `{ext}`, `{name}`, `{n}`, `{n:02}`..`{n:06}`, `{mtime:ymd}`, `{exif:date}`, `{exif:ymd}`, `{exif:camera}`, `{id3:artist}`, `{id3:album}`, `{id3:title}`, `{id3:track:02}`, `{id3:year}`). Fallbacks limpos quando uma tag estiver ausente no arquivo (ex.: manter vazio ou valor fallback configurável, evitando literais `"None"`).
     3. **Busca & Substituição / Regex**: usando `regex::RegexBuilder` com tratamento gracioso de erro de sintaxe (retorna `Err(RegexError)` para exibição em banner inline na UI sem panics) e suporte a grupos de captura (`$1`, `$2`, `${nome}`). Compilar o `Regex` **uma única vez por atualização**, nunca dentro do loop de arquivos.
     4. **Transformação de Caixa & Sanitização**: `Keep`, `Lowercase`, `Uppercase`, `TitleCase`, `TrimSpaces`, além de sanitização automática opcional de caracteres ilegais em nomes (`/` no POSIX; `\ / : * ? " < > |` e nomes reservados `CON`/`PRN`/`NUL` quando o alvo for Windows ou modo portátil).
3. **Detector de Conflitos e Validador**:
   - Compara todos os destinos propostos dentro do mesmo `parent_dir`:
     - Em Windows (ou modo case-insensitive), compara com `to_lowercase()` para detectar colisão real em NTFS/FAT (`Foto.jpg` e `foto.jpg` no mesmo lote!).
   - Verifica conflitos com arquivos existentes no diretório que **não** fazem parte do lote (ou que pertencem ao conjunto mas não estão saindo do caminho).
   - Marca cada linha com `BulkRenameRowStatus`:
     - `Ready`
     - `Unchanged`
     - `InvalidName(String)` (nome vazio, `.` / `..`, separador de caminho `/` ou `\`)
     - `ConflictInternal` (colisão com outro item do próprio lote)
     - `ConflictExternal` (já existe no disco fora do lote)
   - O botão **Aplicar** fica desabilitado enquanto houver qualquer `InvalidName` ou `Conflict*` (ou se nenhum item tiver mudança).

4. **Solucionador Topológico / Renomeação em 2 Fases (Ciclos & Case-Only)**:
   - **Problema clássico**:
     - Ciclo de troca: `01.txt -> 02.txt` e `02.txt -> 01.txt`, ou deslocamento `a->b, b->c`.
     - Troca apenas de maiúsculas/minúsculas no Windows (`arquivo.txt -> ARQUIVO.txt`): `rename` direto pode falhar ou exigir passo intermediário dependendo do filesystem.
   - **Solução em `fm-core`**:
     - Construir um grafo direcionado `src -> dst` dentro do diretório.
     - Itens cujo destino ainda está ocupado por outro item do lote (ciclos ou cadeias bloqueadas) ou que alteram apenas case no mesmo caminho entram na **Fase 1** via nome temporário exclusivo no mesmo `parent_dir` (`.__vfm_br_<batch_id>_<idx>__.tmp`), liberando todos os nomes originais, e concluem na **Fase 2** para o destino final.
     - Itens fora de ciclo cujo destino está livre podem ser renomeados em **1 única operação**.
     - Caso um arquivo já falhe na Fase 2, o executor consegue saber exatamente quais temporários restaram para tentar restaurar o nome original e relatar o erro por item sem perda de dados.

---

### 3.2 `fm-telemetry` — Extração Assíncrona e Segura de EXIF / ID3

Novo módulo `crates/fm-telemetry/src/media_tags.rs`:

1. **Carregamento Sob Demanda (`Lazy`)**:
   - Abrir o modal **não** deve disparar leitura de disco de 1.000 arquivos se o usuário quer apenas rodar um Regex simples ou numeração sequencial.
   - O worker de metadados só é disparado quando o pipeline ativo referenciar ao menos um token `{exif:...}` ou `{id3:...}` (ou se o usuário clicar em "Carregar tags").
2. **Leitura Parcial & Baixo Consumo de RAM**:
   - **Abertura segura**: usar sempre `crate::open_shared::open_shared_read(&path)` (Windows share mode `0x7`), evitando erros de arquivo em uso por VLC, navegadores ou editores.
   - **EXIF** (JPEG, TIFF, WebP/PNG quando aplicável via `kamadak-exif`): ler apenas um buffer inicial limitado (ex.: cabeçalho até ~128–256 KiB em chunks ou leitor com `Take`), nunca carregar uma imagem RAW/JPEG de 40 MB inteira na memória para ler 4 tags de texto.
   - **ID3** (MP3 / arquivos com header `ID3`):
     - Ler os primeiros 10 bytes (`"ID3"`, versão `v2.3`/`v2.4`, tamanho synchsafe). Limitar leitura máxima de tag (ex.: pular frames binários gigantes `APIC` de capa de álbum usando `Seek`, **sem alocar imagens de capa na RAM**!).
     - Se não houver ID3v2, fazer `SeekFrom::End(-128)` e verificar assinatura `"TAG"` (ID3v1) nos últimos 128 bytes.
3. **Concorrência Controlada & Cancelamento**:
   - Pool limitado (ex.: `Semaphore` com 4–8 leituras simultâneas em `spawn_blocking` ou `tokio::task` dedicada).
   - Checar `CancellationToken::is_cancelled()` antes de cada arquivo; ao fechar o modal ou trocar de diretório/seleção, cancelar imediatamente liberando todos os handles.
   - Enviar resultados em pequenos lotes (ex.: a cada 25 arquivos ou 50 ms) para evitar tempestade de redraws no Iced.

---

### 3.3 `fm-fs` — Execução de Renomeação & Rollback/Undo

1. O plano gerado em `fm-core` (`Vec<RenameStep>`) é executado em background via `tokio::task::spawn_blocking` usando `VfsProvider::rename(&from, &to)` (ou `std::fs::rename` direto dentro do `LocalFs`).
2. **Antes de executar a Fase 1**, verificar rapidamente se todos os destinos finais ainda não colidem no disco (`try_exists` para evitar corrida se outro processo criou um arquivo enquanto o usuário editava o preview).
3. **Progresso & Feedback**:
   - Emitir progresso (`concluídos / total`) para barra de progresso no rodapé do modal.
   - Ao concluir, disparar refresh da aba afetada, preservar/re-selecionar os arquivos renomeados pelos novos `PathBuf` e registrar uma entrada opcional de **Desfazer Último Lote** (guardando o mapeamento reverso se todos os passos tiveram sucesso).

---

### 3.4 `fm-app` — Estado, Mensagens e Performance da UI

- **Estado dedicado** (ex.: `BulkRenameModalState` em `crates/fm-app/src/app.rs` ou submódulo `bulk_rename_modal.rs`):
  - `items: Vec<BulkRenameItemState>` (com `BulkRenameInput`, `Option<BulkRenameMediaTags>`, último `BulkRenamePreviewRow` calculado).
  - `existing_names_in_dir: HashSet<OsString>` (snapshot instantâneo dos nomes já presentes em `visible_entries` da aba ativa, evitando I/O síncrono para checar colisão!).
  - `rule_state`: campos do formulário (busca, substituição, flags regex, template, início/passo de numeração, transformação de caixa, ordenação do lote).
  - `preview_generation: u64`, `tags_cancel: Option<CancellationToken>`, `tags_progress: (usize, usize)`.
  - `modal_offset: Option<(f32, f32)>` e `modal_size: Option<(f32, f32)>` + estados de drag/resize análogos a `properties_modal_offset` / `properties_modal_size`.
- **Virtualização da Tabela de Preview**:
  - Cada linha tem altura fixa (`ROW_HEIGHT = 28.0`), permitindo cálculo direto de `first_visible .. last_visible` a partir de `scroll_offset_y` e `viewport_height`, mantendo `Space::new().height(top_pad)` e `Space::new().height(bottom_pad)` — mesmo padrão já adotado nas visualizações virtualizadas do app.

---

## 4. Riscos Técnicos e Mitigações

| Risco | Impacto | Mitigação |
| :--- | :--- | :--- |
| **Regex catastrófico (ReDoS) ou inválido** | Travamento ao digitar ou panic | `regex` crate em Rust garante tempo linear $O(n)$ (sem backtracking exponencial) e `RegexBuilder::size_limit` limita padrões gigantes; erros retornam `Result` para feedback inline. |
| **Colisão case-insensitive no Windows** | Falha ou sobrescrita silenciosa ao trocar `a.txt -> A.txt` ou `a.txt + A.txt` | Detecção de colisões usando normalização case-insensitive no Windows e passo intermediário temporário em 2 fases para renomeações que mudam só a caixa. |
| **Capas de álbum grandes (`APIC`) em MP3** | Pico de RAM durante leitura de ID3 | Parser ID3 lê apenas o cabeçalho do frame e usa `seek` para pular frames `APIC`/binários sem alocar o payload na memória. |
| **Arquivo aberto em outro programa (VLC, editor)** | Falha ao ler EXIF/ID3 ou renomear no Windows | Leitura usa `open_shared_read`; falhas no `rename` exibem erro por arquivo e abortam com segurança o passo pendente sem corromper os demais. |
| **Lotes com milhares de arquivos travando a UI** | Queda de FPS ao digitar | Lista de preview 100% virtualizada (altura fixa por linha) + disparo opcional de cálculo em worker para lotes grandes (`> 500` itens). |
| **Build Flatpak `--offline --locked`** | Falha de build se novas crates forem adicionadas sem atualizar vendor | Reusar `regex` e `kamadak-exif` já no `Cargo.lock` e usar parser puro-Rust para ID3v1/v2.3/v2.4. |

---

## 5. Plano Faseado de Entrega

### Fase 1 — Motor Puro em `fm-core` (P0)
- Criar `crates/fm-core/src/bulk_rename.rs` com:
  - Parser de templates e numeração sequencial (`{n}`, `{n:03}`, `{stem}`, `{ext}`, `{mtime:...}`, `{exif:...}`, `{id3:...}`).
  - Integração com `regex = "1.13.1"` (`fm-core/Cargo.toml` + `Cargo.toml` raiz).
  - Validador de nomes inválidos (POSIX + regras Windows), detector de conflitos internos/externos e planejador em 2 fases para ciclos e case-only renames.
- Cobertura extensiva de testes unitários em `fm-core` (ciclos A↔B, cadeias A→B→C, colisão case-insensitive, captura regex `$1`, caracteres inválidos, fallback de tags ausentes).

### Fase 2 — Modal UI + Preview Virtualizado + Execução VFS (P1)
- Implementar `BulkRenameModalState` e mensagens em `fm-app`.
- Integrar gatilhos: `F2`, botão da toolbar e menu de contexto quando `sel_count > 1` (e atalho opcional mesmo com 1 arquivo pelo menu de contexto).
- Construir modal em `crates/fm-app/src/views/modals.rs` seguindo **ObsidianFlow / AetherLight**, com arraste pela barra de título, redimensionamento pelo canto inferior direito, bloqueio de clique ao fundo (`opaque`) e tabela de preview virtualizada.
- Executar passos gerados via worker assíncrono no `fm-fs`, atualizar aba mantendo seleção nos itens renomeados e reportar resumo em `status_notice`.

### Fase 3 — Metadados EXIF & ID3 em `fm-telemetry` (P2)
- Criar `crates/fm-telemetry/src/media_tags.rs` usando `open_shared_read`, `kamadak-exif = "0.6.2"` e leitor streaming puro-Rust para ID3v1 / ID3v2.3 / ID3v2.4.
- Disparo lazy ao usar tokens `{exif:*}` / `{id3:*}`, com cancelamento imediato (`CancellationToken`) e atualização progressiva do preview.
- Testes unitários em `fm-telemetry` com fixtures binárias mínimas geradas em teste (JPEG/EXIF mínimo + MP3/ID3v2 com `APIC` pulado + ID3v1).

### Fase 4 — Polimento & Internacionalização (P3)
- Chaves Fluent (`.ftl`) em todos os idiomas suportados pelo `fm-core`.
- Presets recentes / favoritos persistidos em `AppConfig` (opcional).
- Ação rápida **Desfazer última renomeação em lote** na sessão ativa.

---

## 6. Critérios de Verificação (pós-implementação)

Ao executar cada fase, validar obrigatoriamente:
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- Cross-check Windows (se toolchain disponível): `cargo check --target x86_64-pc-windows-gnu`
- Teste de build reprodutível com lockfile: `cargo check --locked`
