# Design Contract

## Approved Direction

- Name: `Ops Premium Clean`
- Tone: professional, modern, quiet, operational
- Theme: neutral light-first with system dark-mode support
- Primary layout: Full HD table-first workspace with a compact operational bar
- Scope: full UX refresh without changing the Tauri command contract

## Color Palette

- App background: `#f6f7f9`
- Surface: `#ffffff`
- Subtle surface: `#f1f3f6`
- Primary text: `#171a20`
- Secondary text: `#626b7a`
- Accent: `#1f6feb`
- Success: `#16803c`
- Warning: `#9a5b00`
- Danger: `#c53434`
- Dark mode base: `#101216`
- Dark mode surface: `#181b21`

## Typography

- UI font: system sans stack (`Inter`, `ui-sans-serif`, `Segoe UI`, system)
- Data font: system mono stack for ports, versions, paths, logs and metrics
- Scale: compact product UI, 12-14px labels/body metadata, 16px base, 18-20px panel headings
- Letter spacing: default for body, slight positive tracking only for uppercase section labels

## Layout Rules

- Full HD is the primary design target; 1024x768 is the compact desktop fallback.
- Mobile: operational bar stacks; profile rows become compact cards; detail follows below the list.
- Desktop: profile table owns the main workspace; detail panel is sticky on the right.
- The first viewport shows the table as soon as possible; diagnostic content must not dominate the page.
- Use table/list density for repeated profiles; reserve cards only for summary metrics, sheets, modals and repeated option controls.
- Radius stays at 8px or below; shadows are subtle and used only for raised surfaces.

## Component Rules

- Profile rows include name, OS/connect badges, status, RAM, endpoints, bundles and quick actions.
- Secondary actions live in the detail panel and overflow menu; primary action remains visible in the row.
- `ops-bar` contains fleet title, compact health, latest operations and metrics in one unified surface.
- Operational Health uses an issue-first compact pattern inside `ops-bar`: status, primary issue, counters and quiet OK chips.
- At 1024x768, the VM table must be visible in the first viewport.
- Warning/error counters only receive strong semantic color when their count is greater than zero.
- Menus are popovers without `role="menu"` semantics; keyboard focus must remain visible.
- Install/settings are side sheets on desktop and full-width sheets on narrow screens.
- Forms use visible labels, associated inputs, semantic field groups and progressive disclosure by OS family.

## Accessibility Rules

- Preserve visible focus rings on every actionable element.
- Primary controls use 44px touch targets; compact controls must not rely on hover only.
- Status is communicated by text plus color/dot, not color alone.
- Icon-only buttons require accessible names.
- Motion is limited to short transform/opacity transitions and respects `prefers-reduced-motion`.
