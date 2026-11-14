# Spec 03 — Contratos de Arquitetura (`fm-core` & Mensageria)

Esta spec define os tipos e interfaces que desacoplam `fm-app` (Iced UI) das implementações de sistema (`fm-fs`, `fm-index`, `fm-telemetry`, `fm-preview`).

---

## 1. Modelo de Domínio (`fm-core`)

### `FileEntry` (Compacto para 100k+ itens)
```rust
pub struct FileEntry {
    pub id: EntryId,                  // Hash estável ou índice único no snapshot
    pub name: CompactString,          // Nome exibido (compact_str, <= 24B inline)
    pub path: PathBuf,                // Caminho real do SO (nunca lossy)
    pub extension: Option<CompactString>,
    pub kind: EntryKind,              // Directory | File | Symlink { target_is_dir: bool } | Special
    pub size_bytes: u64,
    pub modified_at: Option<SystemTime>,
    pub is_hidden: bool,
    pub permissions: PermissionsMeta, // PosixMode(u32) no Linux | WinAttributes(u32) no Windows
    // Preenchido assincronamente (enriquecimento progressivo):
    pub child_count: Option<u32>,     // Apenas diretórios, 1 nível
    pub mime_badge: Option<CompactString>,
}
```

### `DirectorySnapshot` & `DirectoryTelemetry`
* `DirectorySnapshot` é retornado rápido (`< 5ms`) pelo `VfsProvider::list_dir`:
  * `request_id: u64` (descarta respostas obsoletas se o usuário já navegou para outro lugar)
  * `path: PathBuf`
  * `entries: Vec<FileEntry>`
  * `fs_type: Option<CompactString>` (`ext4`, `btrfs`, `ntfs`, etc.)
* `DirectoryTelemetry` chega depois em batches assíncronos:
  * `git_branch: Option<CompactString>` (ex: `"main"`)
  * `folder_stats: FolderSummary { dir_count: usize, file_count: usize, total_direct_bytes: u64 }`
  * `volume_space: Option<VolumeSpace { label: CompactString, free_bytes: u64, total_bytes: u64 }>`

### `TabState` & `SelectionState` (Fonte Única de Verdade)
* Cada aba mantém:
  * `id: TabId`
  * `current_path: PathBuf`
  * `history_back: Vec<PathBuf>` / `history_forward: Vec<PathBuf>`
  * `view_mode: ViewMode` (`Grid`, `DetailTable`, `Columns`)
  * `sort: SortSpec { field: SortField, direction: SortDirection, dirs_first: bool }`
  * `filter_query: String`
  * `entries: Vec<FileEntry>` (lista filtrada/ordenada atual)
  * `selection: SelectionState` (`selected_ids: HashSet<EntryId>`, `anchor: Option<usize>`, `focused: Option<usize>`)
  * `selected_telemetry: Option<SelectedFileTelemetry>` (`sha256_prefix`, `mime_type`, `preview_state`)

---

## 2. Trait `VfsProvider` (`fm-core`)
Todas as operações de arquivo passam por esta abstração assíncrona (sem dependência de tipos `iced`):

```rust
#[async_trait::async_trait]
pub trait VfsProvider: Send + Sync {
    async fn list_dir(&self, path: &Path, cancel: CancellationToken) -> Result<DirectorySnapshot, FsError>;
    async fn list_drives(&self) -> Result<Vec<DriveInfo>, FsError>;
    async fn volume_space(&self, path: &Path) -> Result<Option<VolumeSpace>, FsError>;
    async fn create_dir(&self, path: &Path) -> Result<(), FsError>;
    async fn create_file(&self, path: &Path) -> Result<(), FsError>;
    async fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError>;
    async fn move_to_trash(&self, paths: &[PathBuf]) -> Result<(), FsError>;
    async fn delete_permanent(&self, paths: &[PathBuf]) -> Result<(), FsError>;
    async fn open_default(&self, path: &Path) -> Result<(), FsError>;
}
```

> **Nota sobre `CopyEngine`**: Operações longas (cópia/movimentação de múltiplos arquivos) não devem usar um `await` simples bloqueante, mas retornar um `JobId` com stream de progresso (`JobProgress { bytes_done, bytes_total, current_file, state }`) e aceitar `CancellationToken`.

---

## 3. Protocolo de Mensagens (Iced `update`)
* **Navegação e Abas**:
  * `TabNew(Option<PathBuf>)`, `TabSelect(TabId)`, `TabClose(TabId)`
  * `NavigateTo(TabId, PathBuf)`, `NavigateBack(TabId)`, `NavigateForward(TabId)`, `NavigateUp(TabId)`
  * `DirectoryLoaded { tab_id, request_id, result: Result<DirectorySnapshot, FsError> }`
* **Seleção e Ações**:
  * `SelectEntry { tab_id, index, modifiers }` (suporta clique simples, `Ctrl+Click`, `Shift+Click`, setas do teclado)
  * `ActivateEntry { tab_id, index }` (duplo clique ou `Enter`: entra na pasta ou abre arquivo no SO)
  * `ToggleQuickLook` (`Space`)
  * `ToggleInspector` (`Alt+I`)
  * `OpenSearchPalette` (`Ctrl+K`) / `FilterChanged(String)`
* **Enriquecimento Progressivo**:
  * `TelemetryReady { tab_id, request_id, payload: TelemetryUpdate }`
  * `ThumbnailReady { key: ThumbnailKey, handle: iced::widget::image::Handle }`
  * `FsChanged { watched_path: PathBuf, kind: WatchEventKind }`

---

## 4. Regras de Cancelamento e Descarte
1. Cada `NavigateTo` incrementa um `request_id: u64` monotônico na aba e chama `.cancel()` no `CancellationToken` da navegação anterior.
2. Qualquer mensagem recebida com `request_id != tab.active_request_id` é **descartada imediatamente**.
3. Cada troca de seleção cancela o cálculo anterior de `SHA-256` / leitura de preview antes de iniciar o próximo (com debounce leve de ~80 ms ao navegar rápido pelas setas).
