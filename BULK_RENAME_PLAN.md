# Plano de Implementação — **Renomeação em Lote (`Bulk Rename`) Nativo**

Documento de análise de viabilidade e plano arquitetural para adicionar ao **Vagnarok File Manager** uma ferramenta modal nativa e intuitiva de **Renomeação em Lote** com **preview em tempo real**, controles visuais diretos (substituir, remover, inserir, capitalização, numeração sequencial e alteração de extensão), extração de **metadados (EXIF / ID3)** e **Regex opcional (modo avançado)**, alinhada às regras de engenharia do repositório (`AGENTS.md`).

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
  - `sel_count == 1`: mantém o diálogo rápido de renomeação individual atual (com opção secundária no menu para abrir a **Renomeação em Lote** caso queira aplicar substituição guiada, caixa ou metadados EXIF/ID3 nele também).
  - `sel_count > 1`: `F2`, botão de **Renomear** na barra superior e item **Renomear em Lote...** no menu de contexto (`ContextMenuKind::Entry`) abrem diretamente o modal de **Bulk Rename** com todos os itens selecionados na ordem da aba ativa.
- **Proteção de VFS somente leitura**: Caso a aba atual esteja na Lixeira virtual (`is_virtual_trash_path`) ou dentro de um arquivo compactado (`ArchiveVfs` somente leitura), a ação deve bloquear a abertura com aviso limpo na barra de status (`status_notice`).

### 2.2 Janela Modal Overlay (Arrastável, Redimensionável e com Colisão Completa)

Seguindo o padrão recém-consolidado em `crates/fm-app/src/views/mod.rs` (`modal_overlay_backdrop_at` com `opaque`) e `crates/fm-app/src/app.rs` (`effective_properties_modal_rect`):
- O modal deve rodar sobre `modal_overlay_backdrop_at` + `opaque(...)` para **bloquear cliques na janela principal atrás**.
- Deve suportar **arrastar pela barra de título** e **redimensionar pelo canto inferior direito** (essencial para visualizar caminhos/nomes longos sem truncamento excessivo), com dimensões mínimas seguras (ex.: `740x520`).

### 2.3 Design User-Friendly e Wireframe Conceitual (Tokens `ObsidianFlow` / `AetherLight`)

A interface deve ser **direta para qualquer usuário**, evitando exigir sintaxe técnica como `IMG_(\d+)_(\d+)` ou templates complexos `{n:03}` na tela principal. Em vez disso, o painel de regras oferece **cartões com caixas de seleção (`[x]`)** e controles explícitos e fáceis de entender:

- **Procurar e Substituir**: campos simples de texto + escolha clara de onde atuar (*Em qualquer lugar*, *Só no início do nome*, *Só no fim do nome*, *Só início de palavra*, *Só fim de palavra*, *Palavra inteira*). A opção **Usar Regex** fica recolhida/discreta em **"Opções avançadas"** e desligada por padrão.
- **Remover**: caixas para marcar remoção dos *N* primeiros caracteres, *N* últimos caracteres, texto específico, números, espaços extras ou pontuação.
- **Inserir**: adicionar texto no *início do nome*, *fim do nome* (antes da extensão) ou em uma *posição exata*.
- **Maiúsculas / Minúsculas**: opções diretas (*Sem alteração*, *TUDO MAIÚSCULA*, *tudo minúscula*, *Primeira Letra De Cada Palavra*, *Só a 1ª letra do nome*).
- **Numeração Sequencial**: escolher posição (*Início* / *Fim* / *Substituir nome*), separador, número inicial, passo e quantidade de zeros (`1`, `01`, `001`...).
- **Metadados (EXIF / ID3)**: botões/caixas prontas para adicionar *Data da foto*, *Modelo da câmera*, *Artista*, *Álbum*, *Título* ou *Nº da faixa* sem precisar decorar códigos.
- **Extensão**: controles dedicados para *Manter original*, *Trocar extensão para...*, *Colocar extensão em minúscula/MAIÚSCULA* ou *Remover extensão*.

```text
+--------------------------------------------------------------------------------------+
| [Renomear em Lote]  124 itens selecionados                        [Limpar regras] [x]|
+--------------------------------------+-----------------------------------------------+
| O QUE VOCÊ QUER FAZER?               | PREVIEW EM TEMPO REAL                         |
|                                      |                                               |
| [x] Procurar e Substituir            |  Status | Nome Atual       | Novo Nome        |
|     Procurar:   [ IMG_2026_       ]  |  -------+------------------+---------------- |
|     Substituir: [ Ferias_         ]  |   [OK]  | IMG_2026_01.jpg  | Ferias_01.jpg    |
|     Onde: [ Qualquer lugar        v ]|   [OK]  | IMG_2026_02.jpg  | Ferias_02.jpg    |
|       (Qualquer / Só início / Só fim |   [!!]  | IMG_2026_03.jpg  | Ferias_03.jpg    |
|        Início da palavra / Fim pal.) |         ^ Conflito: nome já existe no disco   |
|     [ ] Diferenciar MAIÚSC./minúsc.  |   [--]  | notas.txt        | (sem mudança)    |
|     [ ] Só a 1ª ocorrência           |                                               |
|     > Opções avançadas (Regex)       |  (Lista virtualizada: renderiza só visível)   |
|                                      |                                               |
| [ ] Remover                          |                                               |
|     [ ] Primeiros [ 0 ] caracteres   |                                               |
|     [ ] Últimos   [ 0 ] caracteres   |                                               |
|     [ ] Trecho:   [               ]  |                                               |
|     [ ] Espaços extras  [ ] Números  |                                               |
|                                      |                                               |
| [ ] Inserir Texto                    |                                               |
|     Texto:   [                    ]  |                                               |
|     Posição: [ No início do nome  v ]|                                               |
|                                      |                                               |
| [x] Maiúsculas / Minúsculas          |                                               |
|     Estilo: [ Primeira Letra Pal. v ]|                                               |
|       (Sem alteração / TUDO MAIÚSC.  |                                               |
|        minúscula / Só 1ª letra nome) |                                               |
|                                      |                                               |
| [ ] Numeração Sequencial             |                                               |
|     Posição: [ No fim ]  Sep.: [ _ ] |                                               |
|     Início: [ 1 ]  Passo: [ 1 ]      |                                               |
|     Dígitos: [ 001 (3 dígitos)    v ]|                                               |
|     Ordem:   [ Nome (A-Z)         v ]|                                               |
|                                      |                                               |
| [ ] Usar Dados da Foto / Música      |                                               |
|     [+] Data da foto  [+] Câmera     |                                               |
|     [+] Artista  [+] Álbum  [+] Faixa|                                               |
|                                      |                                               |
| [ ] Mudar Extensão                   |                                               |
|     Ação: [ Manter original       v ]|                                               |
|     Nova extensão: [ jpg ]           |  Leitura EXIF/ID3: ociosa (só ao ativar)     |
+--------------------------------------+-----------------------------------------------+
| Escopo: [x] Nome  [ ] Pastas     | 120 alterados | 3 sem mudança | 1 conflito        |
|                                                    [ Cancelar ]  [ Aplicar (120) ]   |
+--------------------------------------------------------------------------------------+
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
2. **Pipeline Componível e Visual de Transformações**:
   - Cada bloco da UI mapeia para uma estrutura tipada em `BulkRenamePipeline` (somente as etapas marcadas como ativas são aplicadas), executada em ordem determinística sobre `(index, input, tags)`:
     1. **Separação segura de nome base (`stem`) e extensão (`ext`)**: respeitar arquivos sem extensão e diretórios (em pastas, `is_dir == true`, o ponto em `"v1.2.0"` pertence ao nome).
     2. **Composição por Metadados (opcional)**: caso o bloco de EXIF/ID3 esteja ativo, monta ou anexa campos selecionados pelo usuário (*Data da foto*, *Câmera*, *Artista*, *Álbum*, *Título*, *Faixa* com zero-padding) com separador configurável e fallback limpo quando a tag não existir no arquivo (nunca gerar literais `"None"`).
     3. **Remover**:
        - Primeiros `N` caracteres / últimos `N` caracteres / intervalo `[de..até]`.
        - Texto literal exato (com filtro de posição: *qualquer lugar*, *só início*, *só fim*).
        - Limpeza rápida por checkbox: espaços duplicados / início e fim (`trim`), apenas dígitos, apenas símbolos/pontuação.
     4. **Procurar e Substituir**:
        - Modo padrão **texto simples** com escopo amigável: `Anywhere`, `StartOfName`, `EndOfName`, `WordStart` (altera só o início da palavra), `WordEnd` (altera só o fim da palavra), `WholeWord`, mais flags `case_sensitive` e `first_occurrence_only`.
        - Modo **Regex opcional** (`use_regex: bool`, desligado por padrão): usa `regex::RegexBuilder` com tratamento gracioso de erro de sintaxe (retorna `Err` para banner inline sem panics) e compilação única por atualização antes do loop nos arquivos.
     5. **Inserir Texto**: texto literal no início do nome, no final do nome base, ou em índice `pos` (contando a partir do início ou do fim).
     6. **Numeração Sequencial**: insere contador no início, no fim ou substituindo o nome base, com `start`, `step`, `zero_pad_digits` (1..6), `separator`, e ordenação do lote (`SelectionOrder`, `NameAsc`, `NameDesc`, `ModifiedAsc`, `ModifiedDesc`, `SizeAsc`).
     7. **Maiúsculas / Minúsculas (Capitalização)**: `Keep`, `Uppercase` (`TODOS MAIÚSCULA`), `Lowercase` (`todos minúscula`), `TitleCaseWords` (`Primeira Letra De Cada Palavra`), `SentenceCaseFirst` (`Só a primeira letra do nome maiúscula`).
     8. **Mudar Extensão**: `Keep`, `Lowercase`, `Uppercase`, `Replace(String)`, `Remove`, mais sanitização automática opcional de caracteres ilegais no SO (`/` no POSIX; `\ / : * ? " < > |` e nomes reservados `CON`/`PRN`/`NUL` quando aplicável).
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
   - Abrir o modal **não** deve disparar leitura de disco de 1.000 arquivos se o usuário quer apenas fazer uma substituição simples, remover caracteres, ajustar maiúsculas ou numerar.
   - O worker de metadados só é disparado quando o usuário marcar opções do bloco **"Usar Dados da Foto / Música"** (ou clicar em "Carregar tags").
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
  - `rule_state`: estado dos cartões visuais marcáveis (procurar/substituir com escopo de posição/palavra + flag avançada Regex, remover, inserir, maiúsculas/minúsculas, numeração sequencial, metadados EXIF/ID3 selecionados, regra de extensão e ordenação do lote).
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
  - Pipeline componível para regras visuais: **Procurar/Substituir** (qualquer lugar, início/fim do nome, início/fim de palavra), **Remover** (N primeiros/últimos, trecho, espaços/números), **Inserir** (início/fim/posição), **Capitalização** (maiúsculas, minúsculas, primeira letra de cada palavra, só 1ª letra do nome), **Numeração Sequencial**, **Extensão** e campos de metadados.
  - Suporte opcional a Regex via `regex = "1.13.1"` (`fm-core/Cargo.toml` + `Cargo.toml` raiz) apenas quando o usuário marcar o modo avançado.
  - Validador de nomes inválidos (POSIX + regras Windows), detector de conflitos internos/externos e planejador em 2 fases para ciclos e case-only renames.
- Cobertura extensiva de testes unitários em `fm-core` (ciclos A↔B, cadeias A→B→C, colisão case-insensitive, início/fim de palavra, mudança de extensão, remoção/inserção por índice, modo Regex avançado, fallback de tags ausentes).

### Fase 2 — Modal UI Amigável + Preview Virtualizado + Execução VFS (P1)
- Implementar `BulkRenameModalState` e mensagens em `fm-app`.
- Integrar gatilhos: `F2`, botão da toolbar e menu de contexto quando `sel_count > 1` (e atalho opcional mesmo com 1 arquivo pelo menu de contexto).
- Construir modal em `crates/fm-app/src/views/modals.rs` seguindo **ObsidianFlow / AetherLight**, com cartões recolhíveis/marcáveis de fácil leitura (sem expor sintaxe Regex por padrão), arraste pela barra de título, redimensionamento pelo canto inferior direito, bloqueio de clique ao fundo (`opaque`) e tabela de preview virtualizada.
- Executar passos gerados via worker assíncrono no `fm-fs`, atualizar aba mantendo seleção nos itens renomeados e reportar resumo em `status_notice`.

### Fase 3 — Metadados EXIF & ID3 em `fm-telemetry` (P2)
- Criar `crates/fm-telemetry/src/media_tags.rs` usando `open_shared_read`, `kamadak-exif = "0.6.2"` e leitor streaming puro-Rust para ID3v1 / ID3v2.3 / ID3v2.4.
- Disparo lazy ao marcar chips/opções de **Dados da Foto / Música (EXIF / ID3)**, com cancelamento imediato (`CancellationToken`) e atualização progressiva do preview.
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
