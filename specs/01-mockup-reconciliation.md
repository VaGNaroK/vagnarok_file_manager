# Spec 01 — Reconciliação do Mockup Inicial

Esta spec define a decisão canônica para cada inconsistência encontrada nos arquivos originais de design ([DESIGN.md](../DESIGN.md), [code.html](../code.html) e [screen.png](../screen.png)).

---

## 1. Tema: Dark (*Obsidian Flow*) vs. Light (*Aether Light*)
* **Conflito observado**: `DESIGN.md` descreve um tema dark (`#111319` / `#0F1117`), enquanto `code.html` e `screen.png` exibem um tema light (`#f8fafc`, `#ffffff`, `#2563eb`). Além disso, o frontmatter YAML de `DESIGN.md` usa tokens estilo Material 3 (`#adc6ff`), diferente da seção prose (`#3B82F6`).
* **Decisão**:
  * Implementar **ambos os temas** no `fm-theme` usando exatamente os mesmos nomes de tokens semânticos (ver [02-design-system-tokens.md](02-design-system-tokens.md)).
  * `ObsidianFlow` (Dark) adota os tokens consolidados de `DESIGN.md` (superfícies `#0F1117` → `#191B22` → `#1E1F26` → `#282A30` e acentos `#3B82F6` / `#ADC6FF` / `#8B5CF6`).
  * `AetherLight` (Light) adota fielmente a paleta de `code.html` / `screen.png` (`#f8fafc`, `#ffffff`, `#f4f6fb`, `#2563eb`).
  * Padrão inicial: seguir o tema do SO (`dark` / `light`), com toggle rápido em *Preferences* e atalho.

## 2. Breadcrumbs Duplicados
* **Conflito observado**: `code.html` renderiza `home > aether > Documents` no header e `~ / Documents / Workspace` no topo do `<main>`.
* **Decisão**:
  * Manter **um único Breadcrumb** na barra de navegação do header (linha 2 do top shell).
  * A barra superior interna do `<main>` passa a abrigar apenas: badge contextual (ex: `git:(main)` quando dentro de repositório Git), resumo do caminho/filtro ativo e os botões de ação rápida (`+ Novo`, `Ordenar`, `Filtrar`, `Toggle Inspector`).
  * Clicar na área vazia do breadcrumb ou pressionar `Ctrl+L` transforma o breadcrumb em um input de caminho editável (`PathBar`).

## 3. Contradição no Estado de Seleção
* **Conflito observado**: Em `screen.png`, `main.rs` está selecionado e a micro-barra interna diz `1 item selecionado (8.9 KB)`, mas o footer inferior diz `Selected: None`.
* **Decisão**:
  * Criar uma fonte única de verdade (`SelectionState` na aba ativa).
  * Remover a duplicação entre "micro-status bar" dentro do `<main>` e "footer fixo":
    * O **Footer Fixo (`32px`)** concentra: contagem total de itens (`14 items`), seleção atual (`1 item selecionado (8.9 KB)` ou `None`), dica contextual (`Space: QuickLook`), hash `sha256` resumido quando computado, e espaço livre do volume ativo (`248.4 GB free of 1.0 TB`).

## 4. Painel Inspetor Direito (3º Painel)
* **Conflito observado**: `DESIGN.md` especifica um drawer direito de `280px`; `code.html` tem apenas o botão `info` sem o painel.
* **Decisão**:
  * O **Right Inspector Drawer (`280px`)** existe como painel colapsável/toggleável controlado pelo botão `info` e pelo atalho `Alt+I`.
  * Quando recolhido (padrão em janelas estreitas `< 1100px`), o canvas ganha largura total; quando aberto, exibe preview ampliado, permissões/atributos, datas, metadados de mídia/Git e verificação `SHA-256` sob demanda.

## 5. Controles de Janela e Cross-Platform (Linux + Windows)
* **Conflito observado**: `code.html` usa *traffic lights* macOS à esquerda e menção a `Arch Linux • bspwm` no Hero Card.
* **Decisão**:
  * **Windows**: Controles de janela integrados à direita (Minimizar, Maximizar/Restaurar, Fechar) na titlebar customizada de `40px`.
  * **Linux**: Suporte a `window_decorations` configurável:
    * Modo *Native WM* (recomendado para tiling WMs como `bspwm`/`Hyprland`/`i3`, sem botões redundantes na barra de abas).
    * Modo *Integrated CSD* (botões estilo desktop Linux à direita).
  * O texto do Hero Card não deve ser hardcoded: exibe nome da pasta atual, tipo do sistema de arquivos real (`ext4`, `btrfs`, `ntfs`), contagem de diretórios/arquivos e tamanho calculado de forma assíncrona/não-bloqueante (com toggle nas preferências para ocultar o Hero Card em modo compacto).

## 6. Idioma e Assets Locais
* **Conflito observado**: Mistura PT-BR / EN no mesmo layout e fontes/ícones via Google Fonts CDN.
* **Decisão**:
  * Todo texto visível passa por um módulo `i18n` (`pt-BR` e `en-US`), sem mistura de idiomas na mesma tela.
  * Fontes `Inter` e `JetBrains Mono` + ícones SVG serão embutidos no binário via `include_bytes!`.
