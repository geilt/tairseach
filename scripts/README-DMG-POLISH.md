# DMG Polish System for Tairseach

This directory contains scripts to apply professional polish to the macOS DMG installer.

## What Gets Fixed

1. **Hidden VolumeIcon.icns** - The volume icon file is marked invisible so it doesn't appear in the DMG window
2. **Rounded Volume Icon** - A pre-rounded volume icon with macOS-style appearance (rounded corners + shadow)
3. **Preserved Layout** - DMG window layout from commit 13233b2 (app at 180,170 / Applications at 480,170 / 660x400 window)

## Scripts

### `create-rounded-volume-icon.sh`
Generates a rounded macOS-style volume icon from the source `icon-1024.png`:
- Uses 22.37% corner radius (229px at 1024px) - matches macOS app icon style
- Adds drop shadow (60% opacity, 8px blur, 16px offset)
- Outputs `VolumeIcon.icns` for DMG volume icon use

```bash
./scripts/create-rounded-volume-icon.sh
```

### `polish-dmg.sh`
Post-build script that polishes an existing DMG:
1. Converts DMG to read-write format
2. Mounts the DMG
3. Installs the rounded VolumeIcon.icns
4. Sets the invisible flag with `SetFile -a V`
5. Marks volume with custom icon flag
6. Compresses back to final format

```bash
./scripts/polish-dmg.sh
```

### `build.sh` (integrated)
The main build script now includes DMG polish as step 6/7, automatically applying polish after the build completes.

```bash
./scripts/build.sh
```

## How It Works

### The VolumeIcon Problem

When Tauri builds a DMG, it can set a volume icon, but:
1. The `.VolumeIcon.icns` file shows as a visible item in the DMG window
2. The icon appears as a raw square image (macOS doesn't automatically round volume icons)

### The Solution

**For visibility:**
- The polish script uses `SetFile -a V` to mark the file invisible
- Combined with the `.` prefix, this ensures it's truly hidden even when "Show Hidden Files" is enabled

**For appearance:**
- We pre-generate a rounded icon with corners and shadow already baked in
- This matches how macOS app icons appear in Finder
- The rounded icon is created from `src-tauri/icons/icon-1024.png` using ImageMagick

### File Attributes

The hidden VolumeIcon.icns has these attributes:
- **Type:** `icnC` (icon file)
- **Flags:** `aVbstclinmedz` (the 'V' means invisible)
- **Result:** File is hidden even in ls output with `-O` flag

## Manual Usage

If you need to manually polish a DMG:

```bash
# 1. Ensure the rounded icon exists
./scripts/create-rounded-volume-icon.sh

# 2. Build normally
cargo tauri build

# 3. Apply polish
./scripts/polish-dmg.sh
```

## Verification

To verify the polish worked:

```bash
# Mount the DMG
hdiutil attach path/to/Tairseach_0.1.0_aarch64.dmg

# Check that VolumeIcon.icns is marked hidden
ls -laO /Volumes/Tairseach | grep VolumeIcon
# Should show "hidden" in the attributes

# Check visible items (VolumeIcon should NOT appear)
ls -1 /Volumes/Tairseach
# Should only show: Applications, Tairseach.app

# Unmount
hdiutil detach /Volumes/Tairseach
```

## Requirements

- **ImageMagick** - for generating rounded icon (`brew install imagemagick`)
- **macOS Developer Tools** - for `SetFile`, `iconutil`, `sips`
- **Tauri CLI** - for building the app

## Notes

- The polish script is idempotent - running it multiple times is safe
- The rounded volume icon is generated once and reused
- DMG layout config is in `src-tauri/tauri.conf.json` under `bundle.macOS.dmg`
