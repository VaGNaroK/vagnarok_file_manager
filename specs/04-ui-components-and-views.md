# Spec 04 — Componentes da UI e Views (`fm-app`)

Layout desktop de **3 painéis** dentro de uma janela fixa com barra superior integrada e rodapé de telemetria.

---

## 1. Anatomia da Tela

```text
┌───────────────────────────────────────────────────────────────────────────┐
│ [CSD?]  [📁 Documents ×] [</> Projects ×] [+]                      [Theme]│  38px
├───────────────────────────────────────────────────────────────────────────┤
│ [←][→]  [🏠 home > aether > Documents]   [🔍 Search... Ctrl+K]   [⊞][☰][⫿]│  42px
├──────────────┬───────────────────────────────────────────┬────────────────┤
│ QUICK ACCESS │ ~/Documents/Workspace  ● git:(main)       │ INSPECTOR      │
│ • Documentos │ [+ Novo] [Ordenar ▾] [Filtrar] [ⓘ]        │ (Toggle 280px) │
│ • Home       ├───────────────────────────────────────────┤                │
│ • Desktop    │ [Hero Card: aether-workspace | ext4 | ...]│ Preview        │
│ • Downloads  │                                           │ Metadata       │
│ • Imagens    │ PASTAS • 6                    drwxr-xr-x  │ Permissions    │
│ • Código/Dev │ [📁 Projetos] [📁 Design] ...             │ SHA-256        │
│              │                                           │                │
│ DRIVES       │ ARQUIVOS • 8                  -rw-r--r--  │                │
│ • SSD Princ. │ [JSON] [SH] [RS main.rs*] [FIG] ...       │                │
│              │                                           │                │
│ TAGS & SYS   │                                           │                │
│ • Favoritos  │                                           │                │
│ • Lixeira    │                                           │                │
│ ──────────── │                                           │                │
│ ⚙ Preferences│                                           │                │
├──────────────┴───────────────────────────────────────────┴────────────────┤
│ ● 14 items   •   1 item selecionado (8.9 KB)   •   SSD: 248.4 GB free     │  30px
└───────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Especificação por Componente

### A. Top Shell (`80px` total)
* **Linha 1 (`38px`) — Tab Bar**:
  * Abas com ícone contextual, nome truncado (`max ~140px`), botão fechar (`×`) e botão `+` para nova aba.
  * Suporte a `Ctrl+T` (nova aba), `Ctrl+W` (fechar aba), `Ctrl+Tab` (alternar), clique do meio para fechar.
  * Alternador rápido de tema (`AetherLight` ↔ `ObsidianFlow`) e controles de janela adaptativos (à direita no Windows; configurável no Linux).
* **Linha 2 (`42px`) — Navigation & Search Bar**:
  * Botões Voltar (`Alt+Left`) / Avançar (`Alt+Right`) / Subir nível (`Alt+Up`).
  * **Breadcrumb Unificado**: segmentos clicáveis para navegar e chevron `>` com menu de subpastas; clicar no espaço vazio ou `Ctrl+L` alterna para modo edição de caminho com autocompletar.
  * **Search Pill (`Ctrl+K`)**: filtro instantâneo no diretório atual ou abre paleta de busca profunda em subdiretórios.
  * **View Switcher**: 3 botões segmentados (`Grid`, `Detail Table`, `Columns`).

### B. Left Navigation Drawer (`240px`, colapsável para `64px`)
* Seções carregadas dinamicamente:
  * **Quick Access**: Home, Desktop, Documents, Downloads, Pictures + pastas fixadas pelo usuário.
  * **Drives**: volumes montados reais obtidos por `fm-fs` (com barra sutil ou tooltip de espaço livre).
  * **Tags & System**: Favoritos e Lixeira nativa.
  * Rodapé fixo: botão *Preferences*.
* Colapsável via atalho (`Ctrl+B`) para rail de ícones (`64px`).

### C. Central File Canvas (Viewport Virtualizado)
* **Action Bar Superior**:
  * Badge Git (`git:(main)`) exibido apenas quando o diretório estiver dentro de um repositório Git válido.
  * Botão primário `+ Novo` (dropdown: Nova Pasta, Novo Arquivo).
  * Botões `Ordenar ▾` (Nome, Tamanho, Data de Modificação, Tipo; Pastas Primeiro), `Filtrar` (extensão/ocultos `Ctrl+H`) e `ⓘ` (Toggle Right Inspector).
* **Hero Card (Colapsável nas preferências)**:
  * Exibe ícone do diretório/projeto, nome da pasta, badge do filesystem (`ext4`, `btrfs`, `ntfs`), contagem de diretórios/arquivos e soma direta (ou total em cache).
* **3 Modos de Visualização (Todos Virtualizados)**:
  1. **Grid View** (padrão no mockup): Separado em seções *Pastas* e *Arquivos* (ou unificado conforme configuração), ícones/thumbnails centralizados, badge de extensão (`RS`, `JSON`, `SH`, `PDF`), estado selecionado com anel azul suave e label estilo pill.
  2. **Detail Table**: Linhas de `32px` com colunas redimensionáveis e ordenáveis (`Nome`, `Tamanho` em `JetBrains Mono`, `Tipo`, `Permissões/Atributos`, `Modificado`).
  3. **Column View (Miller Columns)**: Colunas encadeadas horizontalmente para navegação rápida em árvores profundas.

### D. Right Inspector Drawer (`280px`, toggle `Alt+I`)
* Quando 1 item está selecionado:
  * Thumbnail ampliada ou ícone de alta resolução.
  * Nome, caminho completo, tamanho exato em bytes.
  * Permissões (`drwxr-xr-x` / atributos Windows).
  * Datas (criado, modificado).
  * Telemetria assíncrona: tipo MIME real (`infer`), status Git do arquivo, botão/auto-cálculo de `SHA-256`.
* Quando múltiplos itens estão selecionados: mostra contagem e soma de tamanho dos selecionados.

### E. QuickLook Modal (`Space`)
* Ao pressionar `Space` sobre um arquivo selecionado, abre overlay leve e centralizado:
  * **Imagens/SVG**: visualização ajustada à janela.
  * **Texto/Código** (`.rs`, `.json`, `.sh`, `.md`, `.toml`, `.yml`): leitura limitada (ex: primeiros 512 KB) com *syntax highlighting* via `syntect` em fonte `JetBrains Mono`.
  * **Outros formatos**: cartão técnico com metadados, MIME e hash.
* Fechar com `Space` ou `Esc`; setas `↑`/`↓` trocam o arquivo sem fechar o QuickLook.

### F. Bottom Status Footer (`30px`)
* Esquerda: contagem de itens visíveis (`14 items`), seleção atual (`1 item selecionado • 8.9 KB` ou `Selected: None`), atalho contextual (`Space: QuickLook`).
* Direita: `SHA-256` resumido do item selecionado (quando pronto) + espaço livre do volume ativo (`248.4 GB free of 1.0 TB`).
