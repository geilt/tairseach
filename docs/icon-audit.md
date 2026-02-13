# Icon Audit — Tairseach

Date: 2026-02-12
Scope: `src/assets/icons/` only
Agent: Fedelm (subagent)

## Executive summary

- Audited all 30 icon files originally present in `src/assets/icons/`.
- **All icons are square**.
  - 29 files were `1024×1024`
  - `logo.png` is `48×48` (still square; likely intentionally small app logo asset)
- Confirmed iteration artifacts around `permission-contacts-*` were unreferenced and removed.
- Kept the production contacts icon: `permission-contacts.png`.
- No code files were modified.

## Reference check (artifact variants)

Command run:

```bash
cd ~/environment/tairseach && rg "permission-contacts-backup\|dalle3-raw\|dalle3-transparent\|gpt-transparent\|gpt\.\|raw\.\|test\." src/
```

Result: no matches (exit code 1), indicating the variant artifact names are not referenced in `src/`.

## Style/coherence assessment

Confidence: **medium** (grounded by filename lineage, dimensions/modes, and asset family consistency).

Observed pattern in kept icons:
- Consistent resolution (`1024×1024` for all main icons)
- Mostly `RGBA` with transparent-friendly usage
- Cohesive symbolic/iconographic set and naming conventions

Likely generation/origin assessment:
- Core set appears to be a consistent custom icon pack (possibly AI-assisted in pipeline history), but functionally cohesive in current retained assets.
- Contacts variant files explicitly labeled `dalle3`/`gpt` are clear iteration artifacts, not final production assets.

## Inventory (kept)

| File | Size | Mode | Square | Assessment |
|---|---:|---|---|---|
| activity-config.png | 1024×1024 | RGBA | Yes | Final icon, consistent family style |
| activity-connected.png | 1024×1024 | RGBA | Yes | Final icon, consistent family style |
| activity-denied.png | 1024×1024 | RGBA | Yes | Final icon, consistent family style |
| activity-event.png | 1024×1024 | RGBA | Yes | Final icon, consistent family style |
| activity-granted.png | 1024×1024 | RGBA | Yes | Final icon, consistent family style |
| auth-services.png | 1024×1024 | RGBA | Yes | Referenced, consistent family style |
| coming-soon-config.png | 1024×1024 | RGBA | Yes | Final icon, consistent family style |
| coming-soon-profiles.png | 1024×1024 | RGBA | Yes | Final icon, consistent family style |
| logo.png | 48×48 | RGBA | Yes | App/logo asset; intentionally smaller |
| monitor-header.png | 1024×1024 | RGBA | Yes | Referenced, consistent family style |
| permission-accessibility.png | 1024×1024 | RGBA | Yes | Referenced, consistent family style |
| permission-automation.png | 1024×1024 | RGBA | Yes | Referenced, consistent family style |
| permission-calendar.png | 1024×1024 | RGBA | Yes | Referenced, consistent family style |
| permission-camera.png | 1024×1024 | RGBA | Yes | Referenced, consistent family style |
| permission-contacts.png | 1024×1024 | RGBA | Yes | **Chosen final contacts icon** |
| permission-disk.png | 1024×1024 | RGBA | Yes | Referenced, consistent family style |
| permission-location.png | 1024×1024 | RGBA | Yes | Referenced, consistent family style |
| permission-microphone.png | 1024×1024 | RGBA | Yes | Referenced, consistent family style |
| permission-photos.png | 1024×1024 | RGBA | Yes | Referenced, consistent family style |
| permission-reminders.png | 1024×1024 | RGBA | Yes | Referenced, consistent family style |
| permission-screen.png | 1024×1024 | RGBA | Yes | Referenced, consistent family style |
| profile-inner.png | 1024×1024 | RGBA | Yes | Referenced, consistent family style |
| tokens-today.png | 1024×1024 | RGBA | Yes | Final icon, consistent family style |

## Inventory (removed artifact variants)

| File removed | Size | Mode | Why removed |
|---|---:|---|---|
| permission-contacts-backup.png | 1024×1024 | RGBA | Backup artifact; unreferenced |
| permission-contacts-dalle3-raw.png | 1024×1024 | RGB | DALL-E raw iteration artifact; unreferenced |
| permission-contacts-dalle3-transparent.png | 1024×1024 | RGBA | DALL-E transparent iteration artifact; unreferenced |
| permission-contacts-gpt-transparent.png | 1024×1024 | RGBA | GPT transparent iteration artifact; unreferenced |
| permission-contacts-gpt.png | 1024×1024 | RGB | GPT iteration artifact; unreferenced |
| permission-contacts-raw.png | 1024×1024 | RGB | Raw iteration artifact; unreferenced |
| permission-contacts-test.png | 1024×1024 | RGBA | Test artifact; unreferenced |

## Generation/replacement actions

- No new icon generation performed in this pass.
- Rationale: retained production icons are square and cohesive; cleanup target was iteration artifact removal.

If regeneration is desired in a future pass, use:
- **Style brief:** geometric flat iconography, clean strokes, minimal/no gradients, transparent background, Tairseach palette consistency.
- **Target size:** 1024×1024 source (or 512×512 minimum), exported PNG RGBA.

## Git notes

- This audit intentionally changes assets/docs only.
- No source code modifications were made by this task.
