---
name: Obsidian Flow
colors:
  surface: '#111319'
  surface-dim: '#111319'
  surface-bright: '#373940'
  surface-container-lowest: '#0c0e14'
  surface-container-low: '#191b22'
  surface-container: '#1e1f26'
  surface-container-high: '#282a30'
  surface-container-highest: '#33343b'
  on-surface: '#e2e2eb'
  on-surface-variant: '#c2c6d6'
  inverse-surface: '#e2e2eb'
  inverse-on-surface: '#2e3037'
  outline: '#8c909f'
  outline-variant: '#424754'
  surface-tint: '#adc6ff'
  primary: '#adc6ff'
  on-primary: '#002e6a'
  primary-container: '#4d8eff'
  on-primary-container: '#00285d'
  inverse-primary: '#005ac2'
  secondary: '#a4c9ff'
  on-secondary: '#00315d'
  secondary-container: '#0267b8'
  on-secondary-container: '#d6e5ff'
  tertiary: '#d0bcff'
  on-tertiary: '#3c0091'
  tertiary-container: '#a078ff'
  on-tertiary-container: '#340080'
  error: '#ffb4ab'
  on-error: '#690005'
  error-container: '#93000a'
  on-error-container: '#ffdad6'
  primary-fixed: '#d8e2ff'
  primary-fixed-dim: '#adc6ff'
  on-primary-fixed: '#001a42'
  on-primary-fixed-variant: '#004395'
  secondary-fixed: '#d4e3ff'
  secondary-fixed-dim: '#a4c9ff'
  on-secondary-fixed: '#001c39'
  on-secondary-fixed-variant: '#004883'
  tertiary-fixed: '#e9ddff'
  tertiary-fixed-dim: '#d0bcff'
  on-tertiary-fixed: '#23005c'
  on-tertiary-fixed-variant: '#5516be'
  background: '#111319'
  on-background: '#e2e2eb'
  surface-variant: '#33343b'
typography:
  headline-lg:
    fontFamily: Inter
    fontSize: 24px
    fontWeight: '600'
    lineHeight: 32px
    letterSpacing: -0.02em
  headline-md:
    fontFamily: Inter
    fontSize: 18px
    fontWeight: '600'
    lineHeight: 24px
    letterSpacing: -0.015em
  headline-sm:
    fontFamily: Inter
    fontSize: 15px
    fontWeight: '600'
    lineHeight: 20px
    letterSpacing: -0.01em
  body-lg:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '400'
    lineHeight: 20px
  body-md:
    fontFamily: Inter
    fontSize: 13px
    fontWeight: '400'
    lineHeight: 18px
  body-sm:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: '400'
    lineHeight: 16px
  label-md:
    fontFamily: JetBrains Mono
    fontSize: 12px
    fontWeight: '500'
    lineHeight: 16px
  label-sm:
    fontFamily: JetBrains Mono
    fontSize: 11px
    fontWeight: '500'
    lineHeight: 14px
  caption:
    fontFamily: Inter
    fontSize: 11px
    fontWeight: '400'
    lineHeight: 14px
rounded:
  sm: 0.25rem
  DEFAULT: 0.5rem
  md: 0.75rem
  lg: 1rem
  xl: 1.5rem
  full: 9999px
spacing:
  gutter: 0.75rem
  margin: 1rem
  space-xs: 0.25rem
  space-sm: 0.5rem
  space-md: 0.75rem
  space-lg: 1rem
  space-xl: 1.5rem
---

## Brand & Style

This design system targets power users, software engineers, digital creators, and Linux desktop enthusiasts demanding an ultra-refined, fluid desktop experience. It merges the bespoke structural elegance of macOS (subtle translucent vibrancy, pill selections, uncluttered structural chrome) with the modular operational efficiency of Windows 11 Fluent Design (Mica/acrylic backdrop blurs, command ribbons, multi-tab workspaces, integrated breadcrumb paths).

The design style is **Fluent Glass Minimalist**: deep obsidian and slate foundations layered beneath softly blurred translucent sheens, crisp hairline borders, and neon-tinged indigo/blue precision telemetry. The interface feels lightweight, responsive, and tactile without introducing visual fatigue during prolonged low-light usage.

## Colors

The palette is engineered exclusively for an immersive, distraction-free dark workspace:
- **Base Canvas (`#0F1117`)**: Primary background representing the deepest layer of window chrome, terminal drawers, and canvas backplates.
- **Surface Elevation (`#1A1D24`)**: File viewport, active panes, inspector cards, and context surfaces.
- **Structural Dividers (`#2B303C`)**: 1px translucent borders providing crisp edge boundaries across toolbars, sidebars, and split views without harsh contrast.
- **Primary Interactive (`#3B82F6`)**: Applied to active file states, high-priority command buttons, and progress indicators.
- **Secondary Highlight (`#60A5FA`)**: Focused outline strokes, hovered item fills, and icon accent states.
- **Tertiary Accent (`#8B5CF6`)**: Subtle violet telemetry, tag badges, and metadata classifications.
- **Mica Overlay**: `rgba(26, 29, 36, 0.72)` paired with hardware-accelerated background saturation.

## Typography

Typography prioritizes density, optical balance, and monospace technical metadata readability:
- **Headings & Body**: Inter provides universal cross-platform legibility, vertical rhythm, and neutral aesthetic consistency across file trees and view columns.
- **Labels & System Meta**: JetBrains Mono is assigned to file sizes, byte offsets, file permissions (`rwxr-xr-x`), hashes, and path breadcrumbs to preserve tabular numerical alignment.
- **Letter Spacing**: Subtle negative tracking on titles balances large type; labels maintain default tabular spacing.

## Layout & Spacing

The layout operates on a desktop-native three-pane configuration within a fixed-shell application window:
- **Left Navigation Drawer**: 240px default width (collapsible to 64px icon-only rail).
- **Central File Canvas**: Fluid flexible viewport spanning grid, detailed table, and column flow displays.
- **Right Inspector Drawer**: 280px default width (dockable or overlay toggled via command action).
- **Top Shell**: 40px integrated window titlebar + 44px command ribbon and path breadcrumbs.
- **Spacing Rhythm**: Scaled on a 4px/8px modular base. Compact gaps (`space-xs` to `space-sm`) maintain high data density inside list rows and toolbars, while `space-md` isolates distinct functional groupings.

## Elevation & Depth

Visual hierarchy is maintained via acrylic blur, tonal layering, and hairline border illumination rather than stark drop shadows:
- **Level 0 (Window Canvas)**: Pure `#0F1117` base layer.
- **Level 1 (Side Panels & Titlebar)**: Acrylic surface (`rgba(26, 29, 36, 0.65)`) with a 24px backdrop blur (`backdrop-filter: blur(24px)`), bound by a 1px border (`#2B303C`).
- **Level 2 (Active Tab & Elevated Cards)**: `#1A1D24` solid fill with faint 1px internal highlight stroke (`rgba(255, 255, 255, 0.05)`).
- **Level 3 (Context Menus, Popovers & Dropdowns)**: Acrylic layer (`rgba(31, 36, 46, 0.85)`) with 32px backdrop blur, bordered by `rgba(96, 165, 250, 0.2)` and grounded by an ambient shadow (`0 12px 32px rgba(0, 0, 0, 0.45)`).
- **Focus Rings**: 1px outer stroke of `rgba(96, 165, 250, 0.65)` accompanied by a 2px diffused ambient glow (`0 0 8px rgba(59, 130, 246, 0.35)`).

## Shapes

The interface employs smooth modern radii balancing precision with organic soft-pill ergonomics:
- **Base Surfaces & Cards**: `rounded-md` (0.5rem / 8px) for inspector modules, split viewports, and context panels.
- **Interactive Controls**: Pill-shaped treatments (`rounded-full`) for quick filter tags, search bars, and primary command bar triggers.
- **List & Tree Items**: 6px inner radius for tree nodes and file table item selections.
- **Window Frame**: 12px outer perimeter matching contemporary desktop composite window managers.

## Components

### Titlebar & Window Controls
- **Traffic Lights**: Left-aligned macOS-style close, minimize, and maximize dots with hover glyph micro-interactions (`#EF4444`, `#F59E0B`, `#10B981`) seated in a 40px integrated acrylic titlebar.
- **Tab Bar**: Horizontal multi-tab row inline with or directly below traffic controls. Active tab features a solid `#1A1D24` background, subtle rounded pill edges, 1px top highlight, smooth entry/exit animations, and an embedded close button.

### Command Bar & Breadcrumbs
- **Command Buttons**: Pill-shaped action triggers with 8px horizontal padding, subtle hover transitions (`rgba(255, 255, 255, 0.06)`), and dedicated shortcuts.
- **Breadcrumb Navigator**: Interactive segment chain displaying current path (`/home/user/projects/...`), using chevron dividers with hover-activated dropdown menus for directory leaping.

### Navigation Drawer (Sidebar)
- **Hierarchy Sections**: Favorited folders, mounted drives, tags, and network locations grouped under subtle monospace uppercase headers.
- **Selection States**: Pill highlight with `rgba(59, 130, 246, 0.15)` fill, vibrant blue icon tint, and a left accent tick.

### File View & Data Table
- **Grid View**: Rounded file tiles with centered high-contrast icons, thumbnail previews, and two-line truncated labels.
- **Detail Table**: Alternating subtle hover states, sticky header row with resizable columns, monospace file sizes, and permission hashes.

### File Inspector Pane
- **Metadata Cards**: Collapsible cards rendering live media previews, EXIF/ID3 data, Git branch states, checksum verifications, and permission toggles.

### Inputs & Controls
- **Search Bar**: Centered rounded-pill input with integrated filter toggle and `JetBrains Mono` shortcut indicator (`Ctrl + K`).
- **Checkboxes & Radios**: 4px rounded selection boxes with crisp blue fill and high-contrast checkmarks.