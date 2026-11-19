# Plano de Implementação — **Extração de Arquivos (`zip`, `tar`, `tar.gz`, `7z`, `rar`) e Compactação (`7z`, `zip`, `tar`, `tar.gz`)**

Documento de análise técnica e plano arquitetural para adicionar ao **Vagnarok File Manager** suporte completo no **menu de contexto** para **Extração de arquivos** (`.zip`, `.tar`, `.tar.gz`/`.tgz`, `.7z` e **`.rar` somente leitura/extração**) e **Compactação ("Compactar como...")** com seletor de formato (sem `.rar`, formato proprietário), seletor de nível de compressão (`Armazenar`, `Mínimo`, `Normal`, `Máximo`), execução assíncrona em segundo plano e **janela minimizável que libera totalmente a navegação e abas durante o processo**.

---

## 1. Análise de Viabilidade & Diagnóstico do Estado Atual

**Viabilidade: 100% compatível com a arquitetura do repositório (`AGENTS.md`)**, exigindo apenas a adição de uma crate pura em Rust para 7-Zip no `Cargo.lock`:

1. **O que já existe no workspace (`Cargo.lock` e `crates/fm-fs`)**:
   - `Cargo.lock` já inclui **`zip` (`9.6.0`)**, **`tar` (`0.4.46`)** e **`flate2` (`1.1.11`)**.
   - `crates/fm-fs/src/archive.rs` já implementa leitura/listagem virtual somente leitura (`ArchiveKind::Zip | Tar | TarGz`), detecção de extensão (`detect_archive_kind`, `is_archive_file`, `split_archive_path`) e abertura compartilhada no Windows (`open_shared_archive` com `share_mode(0x7)`).
   - `crates/fm-fs/src/copy_engine.rs` e `crates/fm-core/src/domain.rs` já contam com `JobProgress`, `JobState`, `ConflictPolicy`, `CancellationToken` e streaming via `tokio::sync::mpsc`.
2. **O que falta para 7-Zip (`.7z`) e RAR (`.rar`) — 100% Rust Nativo**:
   - Não há crate `.7z`, `lzma` ou `.rar` no `Cargo.lock` atual.
   - Pelas regras do `AGENTS.md` (100% Rust nativo, sem bindings C/C++ pesados como `libarchive`/`lzma-sys`/UnRAR C++ nem dependência de CLI externo `7z`/`unrar`), as duas famílias usam crates distintas em Rust puro:
     - **7-Zip (`.7z`)**: **`sevenz-rust2`** (com `lzma-rust2` para LZMA/LZMA2, licença Apache-2.0) — implementa leitura/extração e criação de `.7z`, **mas não decodifica `.rar`** (apenas o binário C++ do 7-Zip embute decoder RAR separado).
     - **RAR (`.rar`, RAR4 / RAR5 — SOMENTE descompactação)**: **`unrar-rs`** (port em **100% Rust puro**, configurado explicitamente com backend criptográfico puro-Rust `crypto-rust` para evitar dependências C/CMake como `aws-lc`). Sua licença (`GPL-3.0-or-later` derivada das regras do UnRAR) **permite descompactar/ler gratuitamente e proíbe implementar compressor RAR** — exatamente alinhado à decisão do projeto de **suportar `.rar` apenas para extração/leitura, nunca para compactação**. *(Opcionalmente encapsulado atrás de uma feature Cargo `rar-extract`, ligada por padrão, caso empacotamentos FOSS estritos como Flathub exijam desativar codecs derivados de UnRAR).*
   - **Atenção ao Build Offline (Flatpak `--offline --locked`)**: como `packaging/flatpak/dev.vagnarok.FileManager.yml` usa `cargo build --release --offline --locked -p fm-app`, ao adicionar `sevenz-rust2` e `unrar-rs` será necessário atualizar `Cargo.lock` e sincronizar o cache offline/vendor usado pelo build Flatpak.
3. **Desempenho e Thread de UI Sagrada**:
   - Compressão LZMA2 em Rust puro é mais custosa em CPU do que C++/ASM (especialmente no nível **Máximo**), podendo levar segundos ou minutos em pastas grandes.
   - Portanto, toda compactação/extração roda em `tokio::task::spawn_blocking` dentro do `fm-fs`, usando canal `mpsc` para emitir progresso incremental sem bloquear um único frame da thread do `iced`.

---

## 2. Experiência do Usuário (UX / UI — Design System Vagnarok)

### 2.1 Entradas no Menu de Contexto (`ContextMenuKind::Entry` em `crates/fm-app/src/views/modals.rs`)

Quando `!is_in_trash` e `split_archive_path(path).is_none()` (não estamos dentro de um arquivo virtual ou da Lixeira):

1. **Quando `sel_count == 1` e o item for arquivo compactado suportado** (`.zip`, `.tar`, `.tar.gz`, `.tgz`, `.7z`, **`.rar`**):
   - **Extrair aqui** → extrai diretamente para o diretório atual da aba (`parent_dir`).
   - **Extrair em "`<nome_base>/`"** → extrai dentro de subpasta com o nome base do arquivo (evitando espalhar dezenas de arquivos soltos na pasta atual; útil também para `.rar` multi-volume quando selecionado `.part1.rar`).
   - *(Opcional futuro: "Extrair para...")*
2. **Para qualquer seleção válida (`sel_count >= 1`, arquivos e/ou pastas)**:
   - **Adicionar a "`<nome_sugerido>.7z`"** *(atalho rápido opcional 1 clique)*
   - **Compactar como...** → abre a janela configurável descrita abaixo (**apenas** formatos abertos `.7z`, `.zip`, `.tar`, `.tar.gz`; `.rar` nunca é oferecido para criação).

### 2.2 Janela **"Compactar como..."** (Configuração + Progresso Minimizável)

Seguindo os tokens de `fm-theme` (`ObsidianFlow` / `AetherLight`) e o padrão de janela overlay flutuante (`modal_overlay_backdrop_at` com `opaque(...)` e barra de título arrastável):

```text
+-----------------------------------------------------------------------+
| [Compactar como...]                                        [ _ ]  [x] |
+-----------------------------------------------------------------------+
|  Itens selecionados: 18 itens (342,5 MB)                              |
|  Destino:            /home/user/Documentos                            |
|                                                                       |
|  Nome do arquivo:    [ Backup_Projeto                             ]   |
|  Extensão / Formato: [ .7z (7-Zip — Alta compressão)            v ]   |
|    Opções: .7z | .zip | .tar | .tar.gz                                |
|                                                                       |
|  Nível de compressão:[ Normal                                   v ]   |
|    Opções: Armazenar -> Mínimo -> Normal -> Máximo                    |
|                                                                       |
|  Se já existir:      [ Renomear automaticamente ((2))           v ]   |
+-----------------------------------------------------------------------+
|                                          [ Cancelar ]  [ Compactar ]  |
+-----------------------------------------------------------------------+
```

#### Opções exatas dos Dropdowns (`pick_list`):
1. **Dropdown de Formato / Extensão**:
   - `.7z` — 7-Zip (LZMA2)
   - `.zip` — ZIP (Deflate / Store)
   - `.tar` — TAR (Empacotamento sem compressão)
   - `.tar.gz` — TAR.GZ (Gzip comprimido)
2. **Dropdown de Nível de Compressão (ordem exata com *Máximo* por último)**:
   1. **Armazenar** — Sem compressão (nível 0 / *Copy* / *Store*; mais rápido possível)
   2. **Mínimo** — Compressão rápida (baixo uso de CPU)
   3. **Normal** — Equilíbrio padrão entre tamanho e velocidade
   4. **Máximo** — Maior taxa de compressão possível (última escolha da lista)
   - *Regra inteligente para `.tar`*: como `.tar` puro não possui algoritmo de compressão próprio, quando `.tar` estiver selecionado no formato, o dropdown de nível fixa/exibe **Armazenar** (ou fica desabilitado com dica visual) para não enganar o usuário.

### 2.3 Minimizar e Continuar Navegando / Abrindo Abas em Segundo Plano

Ao clicar em **Compactar** (e também durante uma **Extração** longa):

1. **Transição para Visão de Progresso dentro da mesma janela**:
   - Mostra barra de progresso percentual (`bytes_done / bytes_total` e `itens`), arquivo atual sendo processado, taxa/tempo estimado e botão **Cancelar**.
   - A barra de título exibe o botão **Minimizar (`_`)** (e fecha/cancela com confirmação se clicar no `[x]`, ou `[x]` minimiza quando há job ativo se preferir segurança — melhor manter **Minimizar (`_`)** explícito e botão **Cancelar** claro para evitar cancelamento acidental!).
2. **Comportamento ao Minimizar (`minimized = true`)**:
   - **Remove o `overlay` bloqueador (`opaque`) da camada principal**!
   - Isso devolve **100% de interatividade** à janela principal do Vagnarok File Manager:
     - O usuário pode navegar para qualquer pasta, ordenar, filtrar, abrir arquivos, usar a Lixeira/Pesquisa e **abrir ou trocar livremente entre abas (`Ctrl+T`, `Ctrl+W`, cliques)** sem interromper a compactação.
     - O estado do job de compactação/extração fica armazenado no nível do `App` (`active_archive_jobs` / `archive_modal`), **desacoplado da aba atual (`active_tab`)** — fechar ou navegar na aba onde a compactação começou **não** cancela a tarefa em background!
   - **Indicador Persistente no Rodapé (`crates/fm-app/src/views/footer.rs`) / Banner Global**:
     - Enquanto minimizado e rodando em segundo plano, aparece uma pílula compacta interativa no rodapé (visível em **todas as abas**):
       ```text
       [7z] Backup_Projeto.7z • 64% (218 / 342 MB)   [Restaurar]  [Cancelar]
       ```
     - Clicar em **Restaurar** (ou na própria pílula) traz a janela de progresso de volta (`minimized = false`) sobre a aba em que o usuário estiver no momento.
     - Clicar em **Cancelar** aciona o `CancellationToken` do worker, limpa o arquivo parcial inacabado e atualiza a interface.
3. **Atualização Inteligente das Abas ao Concluir**:
   - Quando a tarefa termina em segundo plano, o `App` verifica **todas as abas abertas**: qualquer aba cujo `current_path` coincida com o diretório de destino do `.7z` / pasta extraída recebe refresh automático, mantendo intacta a aba onde o usuário estiver navegando no momento!
   - Exibe aviso discreto na barra de status (`status_notice`) informando a conclusão (ex.: `"Compactação concluída: Backup_Projeto.7z"`) e limpa os locks/handles imediatamente.

---

## 3. Arquitetura Técnica por Crate

### 3.1 `fm-core` — Domínio, Enums e Internacionalização

1. **Novos tipos puros** (ex.: `crates/fm-core/src/domain.rs` ou `crates/fm-core/src/archive_job.rs`):
   - `ArchiveFormat`: `SevenZ`, `Zip`, `Tar`, `TarGz` (com `extension() -> &'static str` e `supports_compression_level() -> bool`).
   - `CompressionLevel`: `Store`, `Minimum`, `Normal`, `Maximum`.
   - `ExtractTargetMode`: `Here`, `Subfolder`.
   - `ArchiveJobKind`: `Compress { format: ArchiveFormat, level: CompressionLevel, dest_file: PathBuf }`, `Extract { kind_label: CompactString, dest_dir: PathBuf }`.
   - `ArchiveJobProgress`: `job_id: u64`, `title: CompactString`, `bytes_done: u64`, `bytes_total: u64`, `items_done: usize`, `items_total: usize`, `current_item: CompactString`, `state: JobState`, `dest_dir: PathBuf`, `output_path: PathBuf`.
2. **Localização Fluent (`.ftl` + `fm-core/src/i18n.rs`)**:
   - Adicionar chaves em todos os 6 idiomas (`en-US`, `pt-BR`, `es-ES`, `fr-FR`, `de-DE`, `it-IT`) para: menu (*Extrair aqui*, *Extrair em...*, *Compactar como...*, *Adicionar a .7z*), dropdowns (*Armazenar*, *Mínimo*, *Normal*, *Máximo*), labels da janela (*Nome*, *Formato*, *Nível de compressão*, *Minimizar*, *Restaurar*, *Compactando*, *Extraindo*) e mensagens de erro/conclusão.

---

### 3.2 `fm-fs` — Engine de Extração e Compactação Segura

1. **Dependências em `Cargo.toml` e `crates/fm-fs/Cargo.toml`**:
   - Adicionar **`sevenz-rust2`** (100% Rust puro, leitura e compressão LZMA2 de `.7z`).
   - Adicionar **`unrar-rs`** (feature `crypto-rust` em Rust puro, sem C/CMake; habilitado atrás da feature Cargo `rar-extract = ["dep:unrar-rs"]` ligada por padrão no `fm-fs`) exclusivamente para leitura/extração de `.rar` (RAR4/RAR5).
   - Garantir resolução em `Cargo.lock` e compatibilidade com Windows/Linux.
2. **Estender `crates/fm-fs/src/archive.rs`**:
   - Incluir `ArchiveKind::SevenZ` (`.7z`) e `ArchiveKind::Rar` (`.rar`, somente leitura) em `detect_archive_kind` para reconhecer ambos no menu de contexto e opcionalmente na listagem virtual `list_archive_dir`.
   - Para nomes divididos estilo `.part1.rar` / `.part01.rar`, remover o sufixo `.partN` ao sugerir o nome base da subpasta de destino no modo *"Extrair em..."*.
3. **Novo módulo `crates/fm-fs/src/archive_ops.rs`**:
   - **`run_compress_job(...)`**:
     - Fase 1 (Inventário): percorre os `sources` selecionados sem seguir symlinks recursivos (evita loops infinitos), ignora sockets/FIFOs especiais, soma `bytes_total` e `items_total`, respeitando `CancellationToken`.
     - Fase 2 (Escrita atômica/segura): resolve conflito de destino (`RenameAuto` → `nome (2).7z`, `Overwrite` ou erro limpo). Cria o arquivo de saída em caminho temporário no mesmo diretório (`.__vfm_pack_<id>__.tmp`) ou final e garante remoção imediata se cancelado ou se ocorrer erro antes de finalizar os headers centrais!
     - **Abertura compartilhada Windows**: todo arquivo de entrada é lido via `open_shared_archive` (`FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`).
     - Envolve `Read` com um `ProgressCancelReader` que atualiza `bytes_done` em chunks e aborta se `cancel.is_cancelled()`.
     - **Mapeamento de formatos & níveis**:
       - `.7z` (`sevenz-rust2`): `Store` (método Copy), `Minimum` / `Normal` / `Maximum` (LZMA2 com parâmetros crescentes de nível/dicionário limitados para não estourar RAM).
       - `.zip` (`zip` `9.6.0`): `Store` → `CompressionMethod::Stored`; `Minimum`/`Normal`/`Maximum` → `CompressionMethod::Deflated` com `.compression_level(Some(1 | 6 | 9))`. *(Em `ArchiveKind::Zip`, atenção ao ZIP64 automático com `.large_file(size >= 4 GiB)`!)*.
       - `.tar` (`tar` `0.4.46`): empacotamento direto sem compressão (`Store`).
       - `.tar.gz` (`tar` + `flate2` `1.1.11`): `GzEncoder::new(out, flate2::Compression::none() / fast() / default() / best())`.
   - **`run_extract_job(...)`**:
     - Suporta **`.zip`, `.tar`, `.tar.gz`/`.tgz`, `.7z` e `.rar`** (RAR4 e RAR5 somente leitura/extração).
     - Em `.rar` com senha ou volumes divididos faltantes (ex.: `.part2.rar` ausente na pasta), retornar `FsError::Io` com mensagem clara na UI sem panics/corrupção parcial (e permitir futura caixa de senha na evolução P2).
     - **Hardening Anti-Zip-Slip obrigatório**:
       - Função pura `sanitize_archive_rel_path(raw: &str) -> Option<PathBuf>` rejeita caminhos absolutos, prefixos `C:`, componentes `..`, bytes `\0`, e garante que o caminho final normalizado pertence estritamente a `dest_root`.
       - Symlinks internos são ignorados por padrão (ou validados estritamente dentro de `dest_root`) para impedir escrita fora da pasta alvo.
     - Se o destino for `ExtractTargetMode::Subfolder`, cria pasta segura baseada no nome do arquivo (removendo `.tar.gz`/`.tgz`/`.zip`/`.7z`/`.rar` e eventual `.partN`) e aplica `ConflictPolicy` por entrada ou diretório.
     - Emite progresso contínuo via `UnboundedSender<ArchiveJobProgress>`.

---

### 3.3 `fm-app` — Estado Desacoplado da Aba, Minimização e UI

1. **Estado no `App` (`crates/fm-app/src/app.rs` + módulo `crates/fm-app/src/archive_jobs.rs`)**:
   - Separar **configuração da janela** da **execução em segundo plano** (ou manter uma lista/mapa de jobs ativos `archive_jobs: HashMap<u64, ArchiveJobRuntime>` + `compress_modal: Option<CompressModalState>`):
     - `CompressModalState`:
       - `sources: Vec<PathBuf>`, `dest_dir: PathBuf`, `archive_name: String`, `format: ArchiveFormat`, `level: CompressionLevel`, `conflict_policy: ConflictPolicy`
       - `running_job_id: Option<u64>` (quando já iniciou a compactação a partir desta janela)
       - `minimized: bool` (`false` = mostra janela flutuante; `true` = janela recolhida para o rodapé, liberando o restante da UI!)
       - `modal_offset: Option<(f32, f32)>`, `dragging_modal: Option<(Point, f32, f32)>` (reutilizando `clamp_modal_pos` e `modal_overlay_backdrop_at`).
     - Além disso, extrações disparadas via menu de contexto (*Extrair aqui* / *Extrair em `<nome>/`*) podem também entrar nesse gerenciador `archive_jobs` (aparecendo já no rodapé / banner minimizado com opção de expandir detalhes ou cancelar!).
2. **Fluxo de UI (`overlay` no `App::view`)**:
   - O overlay modal de compactação só é montado em `App::view()` quando `compress_modal.as_ref().is_some_and(|m| !m.minimized)`.
   - Quando `m.minimized == true`, **nenhum `opaque` overlay bloqueia o `main_shell`**, permitindo cliques, atalhos de teclado (`Ctrl+T`, setas, `Enter`, drag-and-drop), navegação de diretórios e troca de abas normalmente.
   - No `footer.rs` (e/ou `canvas.rs`), qualquer job ativo em `app.archive_jobs` (compactação minimizada ou extração em andamento) renderiza uma pílula compacta com:
     - Ícone + nome do arquivo + porcentagem + botões **[Restaurar]** e **[Cancelar]**.
3. **Refresh de todas as abas afetadas**:
   - Ao receber `ArchiveJobProgress` com `JobState::Completed`, verificar todas as abas em `self.tabs`: para cada aba cuja `current_path == job.dest_dir`, disparar recarregamento assíncrono dessa aba; se for a aba ativa e `output_path` ainda existir, opcionalmente selecionar o `.7z`/pasta recém-criado.
4. **Atalhos & Encerramento Seguro**:
   - Se o usuário pressionar `Esc` com a janela de compactação aberta:
     - Se estiver só configurando → fecha sem iniciar.
     - Se estiver compactando ativamente → **minimiza** a janela em vez de cancelar acidentalmente o trabalho!
   - Se fechar o app principal (`WindowClosed`), cancelar os `CancellationToken` ativos para limpar arquivos `.tmp` antes de sair.

---

## 4. Riscos Técnicos e Mitigações

| Risco | Impacto | Mitigação |
| :--- | :--- | :--- |
| **Build Flatpak `--offline --locked` e Licença RAR** | Falha offline se crates faltarem no vendor ou restrição FOSS em repositórios | Atualizar `Cargo.lock` com `sevenz-rust2` e `unrar-rs` (feature `crypto-rust`, sem deps C), isolar RAR atrás da feature Cargo `rar-extract` (somente leitura, nunca criação) e sincronizar o cache offline Flatpak. |
| **RAR multi-volume ou com senha** | Erro ao faltar `.part2.rar` ou senha desconhecida | Validar presença de volumes sequenciais quando aplicável, falhar graciosamente com mensagem descritiva em `status_notice` e nunca deixar diretório pela metade sem feedback. |
| **Explosão de memória em LZMA2 `Máximo`** | Pico alto de RAM com dicionário gigante em múltiplos threads | Limitar tamanho máximo de dicionário/threads do nível `Máximo` para valores desktop seguros (ex.: dicionário 32–64 MiB em vez de 1 GiB) e testar consumo de RAM antes/depois. |
| **Zip Slip / Traversal ao extrair** | Escrita arbitrária fora da pasta destino | Sanitização estrita componente a componente + bloqueio de symlinks saindo de `dest_dir` antes de criar qualquer arquivo. |
| **Cancelamento deixando `.7z` corrompido** | Usuário cancela e fica arquivo quebrado na pasta | Escrever em arquivo temporário oculto `.__vfm_pack_*.tmp` (ou deletar `output_path` no `Drop`/cancelamento) e só renomear para o `.7z` final após finalizar com sucesso. |
| **Fechar aba onde a compactação começou** | Perda de referência ou crash ao atualizar UI | Jobs pertencem ao `App`, nunca ao `TabState`; conclusão atualiza abas ainda abertas no `dest_dir` pelo `PathBuf`, ignorando abas fechadas. |

---

## 5. Plano Faseado de Implementação

### Fase 1 — Motor de Domínio e I18n (`fm-core`)
- Adicionar enums e structs (`ArchiveFormat`, `CompressionLevel`, `ExtractTargetMode`, `ArchiveJobProgress`) em `fm-core`.
- Adicionar chaves Fluent nos 6 arquivos `.ftl` e em `Strings` (`i18n.rs`).
- Testes unitários de formatação de nomes padrão, detecção de extensão e validação das chaves `.ftl`.

### Fase 2 — Engine de Extração (`.zip`, `.tar`, `.tar.gz`, `.7z`, `.rar`) e Compactação em `fm-fs`
- Adicionar `sevenz-rust2` e `unrar-rs` (100% Rust puro, apenas decodificação RAR via feature `rar-extract`) no workspace + `fm-fs`.
- Implementar `crates/fm-fs/src/archive_ops.rs` com `run_extract_job` (`.zip`, `.tar`, `.tar.gz`, `.7z`, `.rar`, anti-Zip-Slip) e `run_compress_job` (`.7z`, `.zip`, `.tar`, `.tar.gz` com os 4 níveis `Armazenar`, `Mínimo`, `Normal`, `Máximo` — sem compactação `.rar`).
- Integrar `ArchiveKind::SevenZ` e `ArchiveKind::Rar` na detecção/listagem somente leitura de `archive.rs`.
- Testes unitários em `fm-fs`: roundtrip completo (`.7z`, `.zip`, `.tar`, `.tar.gz`), extração `.rar` (fixture/teste de cabeçalho e proteção de caminho), bloqueio de Zip Slip (`../../evil.txt`), cancelamento mid-stream apagando temporário e política de conflito.

### Fase 3 — Menu de Contexto, Janela "Compactar como..." e Minimização (`fm-app`)
- Adicionar itens no `ContextMenuKind::Entry` em `crates/fm-app/src/views/modals.rs` e ajustar `est_height`.
- Implementar `CompressModalState` e gerenciador de `archive_jobs` desacoplado de abas em `fm-app`.
- Construir janela flutuante arrastável com dropdown de extensão (`.7z`, `.zip`, `.tar`, `.tar.gz`), dropdown de compressão (`Armazenar`, `Mínimo`, `Normal`, `Máximo`), barra de progresso ao vivo e botão **Minimizar** que recolhe o overlay sem pausar a tarefa.
- Renderizar pílula interativa persistente no rodapé (`views/footer.rs`) com **Restaurar** e **Cancelar**, permitindo navegar e abrir outras abas livremente.

---

## 6. Critérios de Verificação (após implementar)

- `cargo check --locked --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- Cross-check Windows: `cargo check --target x86_64-pc-windows-gnu --workspace --all-targets`
- Teste manual no binário: compactar pasta grande para `.7z` em nível *Normal*/*Máximo*, clicar em **Minimizar**, navegar em outras pastas, abrir novas abas com `Ctrl+T`, restaurar janela pelo rodapé e confirmar conclusão e refresh automático da pasta destino.
