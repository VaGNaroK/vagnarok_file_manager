# Spec 02 — Design System & Tokens (`fm-theme`)

Todos os componentes visuais em `fm-app` devem consumir exclusivamente os tokens definidos em `fm-theme`. Valores hexadecimais hardcoded dentro de widgets são proibidos.

---

## 1. Paletas Semânticas

### Preset 1: `AetherLight` (fiel a `code.html` / `screen.png`)
| Token Semântico | Hex / RGBA | Uso |
| :--- | :--- | :--- |
| `canvas` | `#F8FAFC` | Fundo principal do viewport de arquivos |
| `header_bg` | `rgba(255, 255, 255, 0.85)` | Top shell (titlebar + command bar) |
| `sidebar_bg` | `rgba(244, 246, 251, 0.95)` | Barra lateral esquerda |
| `surface_lowest` | `#FFFFFF` | Cards elevados (Hero Card, item ativo na sidebar, popover) |
| `surface_low` | `#F1F5F9` | Inputs (search bar), breadcrumb pill, footer |
| `surface_high` | `#E2E8F0` | Hover sutil, badges `kbd` (`Ctrl+K`) |
| `border` | `#E2E8F0` | Divisores `1px` estruturais |
| `border_strong` | `#CBD5E1` | Bordas de controles interativos / hover |
| `text_primary` | `#0F172A` | Nomes de arquivos, títulos |
| `text_secondary` | `#64748B` | Tamanhos, metadados secundários, abas inativas |
| `text_muted` | `#94A3B8` | Headers de seção (`QUICK ACCESS`), separadores `>` |
| `accent_primary` | `#2563EB` | Botão `+ Novo`, item selecionado, ícones ativos |
| `accent_primary_hover` | `#1D4ED8` | Hover do botão primário |
| `accent_surface` | `rgba(37, 99, 235, 0.10)` | Fundo de item selecionado no Grid/Table |
| `accent_border` | `#93C5FD` | Borda de foco / card selecionado (`main.rs`) |
| `accent_tertiary` | `#6366F1` | Badges técnicos / tags |
| `status_success` | `#10B981` (`bg #ECFDF5`) | Badge `git:(main)` |
| `status_warning` | `#F59E0B` | Favoritos / avisos |
| `status_error` | `#EF4444` | Erros / ações destrutivas |

### Preset 2: `ObsidianFlow` (reconciliado de `DESIGN.md`)
| Token Semântico | Hex / RGBA | Uso |
| :--- | :--- | :--- |
| `canvas` | `#0F1117` | Fundo base mais profundo do viewport |
| `header_bg` | `rgba(25, 27, 34, 0.80)` | Top shell (com Mica/blur nativo quando disponível) |
| `sidebar_bg` | `rgba(17, 19, 25, 0.88)` | Barra lateral esquerda |
| `surface_lowest` | `#191B22` | Cards elevados e aba ativa |
| `surface_low` | `#1E1F26` | Inputs, breadcrumb pill, footer |
| `surface_high` | `#282A30` | Hover em linhas/botões e badges `kbd` |
| `border` | `#2B303C` | Divisores `1px` estruturais |
| `border_strong` | `#424754` | Bordas de foco secundário |
| `text_primary` | `#E2E2EB` | Texto principal |
| `text_secondary` | `#C2C6D6` | Texto secundário / abas inativas |
| `text_muted` | `#8C909F` | Headers de seção, separadores |
| `accent_primary` | `#3B82F6` | Ações principais e seleção ativa |
| `accent_primary_hover` | `#60A5FA` | Hover primário e contorno de foco |
| `accent_surface` | `rgba(59, 130, 246, 0.16)` | Fundo de seleção |
| `accent_border` | `rgba(96, 165, 250, 0.45)` | Anel de destaque de item selecionado |
| `accent_tertiary` | `#8B5CF6` (`#D0BCFF` text) | Tags e telemetria |
| `status_success` | `#10B981` | Badge `git:(main)` |
| `status_warning` | `#F59E0B` | Favoritos / avisos |
| `status_error` | `#FFB4AB` (`bg #93000A`) | Erros / alertas |

---

## 2. Escala Tipográfica
* **Família UI**: `Inter` (Regular 400, SemiBold 600, Bold 700).
* **Família Técnica / Monospace**: `JetBrains Mono` (Medium 500, Bold 700) — obrigatória para tamanhos (`8.9 KB`), permissões (`drwxr-xr-x`), hashes (`sha256`), contagem de itens e badges de extensão (`RS`, `JSON`, `SH`).

| Nível | Fonte | Tamanho | Line Height | Uso |
| :--- | :--- | :--- | :--- | :--- |
| `headline_md` | Inter SemiBold | `18px` | `24px` | Título do Hero Card (`aether-workspace`) |
| `headline_sm` | Inter SemiBold | `14px` | `20px` | Nomes de pastas no Grid, breadcrumb ativo |
| `body_md` | Inter Regular | `13px` | `18px` | Itens da Sidebar, arquivos na tabela |
| `body_sm` | Inter Regular | `12px` | `16px` | Abas, botões de barra, subtítulos |
| `label_md` | JetBrains Mono Medium | `12px` | `16px` | Nomes de arquivos técnicos, valores de resumo |
| `label_sm` | JetBrains Mono Medium | `11px` | `14px` | Headers de seção uppercase, permissões, footer, badges |

---

## 3. Dimensões, Espaçamento e Raios
* **Base modular**: `4px` (`xs=4`, `sm=8`, `md=12`, `lg=16`, `xl=24`).
* **Estrutura fixa**:
  * Top Titlebar + Tabs: `38px`
  * Command & Navigation Bar: `42px` (total header ~`80px`)
  * Left Sidebar: `240px` (colapsada: `64px`)
  * Right Inspector: `280px` (quando aberto)
  * Bottom Status Footer: `30px`
* **Raios (`BorderRadius`)**:
  * Botões de ação / Search / Badges: `9999px` (pill)
  * Cards / Hero Card / Abas / Ícones no Grid: `8px` (`0.5rem`) a `12px` (`0.75rem`)
  * Itens de lista / Sidebar: `6px`–`8px`
  * Checkboxes: `4px`
