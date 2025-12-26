# Nihilsweeper AI Coding Instructions

## Project Overview
Minesweeper game built with Rust using the [iced](https://github.com/iced-rs/iced) GUI framework (v0.14.0). The game features customizable skins loaded from SVG assets and supports both standard and left-click chord modes.

## Architecture

### Module Structure
- `base/board.rs` - Core game logic with trait-based board abstraction (`Board` trait, `StandardBoard` implementation)
- `ui/game.rs` - Game rendering and canvas drawing (843 lines)
- `ui/skin.rs` - Skin loading system with SVG-to-image conversion (`SkinManager`, `SkinBuilder`)
- `ui/mod.rs` - Main window and state management (`MainWindow`)
- `config.rs` - Global configuration (chord mode, skin selection, cell size, board dimensions)
- `utils.rs` - Resource path resolution (checks `CARGO_MANIFEST_DIR` first, then executable directory)

### Key Design Patterns

**Trait-based Board System**: `Board` trait in [base/board.rs](src/base/board.rs) defines the game interface. `StandardBoard` implements mine generation (delayed until first click), cell opening with flood-fill for empty cells, and chord clicking (open surrounding cells when flags match number).

**Iced Application Pattern**: Uses `iced::application(MainWindow::new, MainWindow::update, MainWindow::view)` from [main.rs](src/main.rs). Message passing via `MainWindowMessage` → `GameMessage` → `BoardMessage` enum hierarchy.

**Canvas-based Rendering**: Game is drawn entirely with `iced::widget::canvas`. UI areas calculated in [game.rs](src/ui/game.rs) `Game::new()`: `game_area`, `top_area`, `board_area`, `counter_area`, `face_area`. Light/shadow paths generated for 3D border effects.

**Dynamic Skin System**: Skins stored in `assets/skin/<name>/`. Each has `skin.toml` config with scaling factors (e.g., `width_scaling`, `height_scaling`). [skin.rs](src/ui/skin.rs) `SkinBuilder::build()` converts SVG files to `iced::widget::image::Handle` at specified cell size using `resvg`, `usvg`, `tiny-skia`.

### State Management

**Board States** ([board.rs](src/base/board.rs)):
- `NotStarted` → `InProgress {opened_cells, flags}` → `Won` or `Lost {opened_cells, flags}`
- Mine placement deferred until first click to ensure safe start
- `StandardBoard::init()` called by `left_click()` or `right_click()` with optional position

**Cell States**: `Closed`, `Opening(u8)`, `Flagged`, `Blasted`

**Cell Contents**: `Empty`, `Number(u8)`, `Mine`

**Chord Mode**: `Standard` (both buttons) or `LeftClick` (single click on opened numbered cell) - configured in [config.rs](src/config.rs)

## Development Workflows

### Build & Run
```bash
# Standard run
cargo run

# With trace logging (includes cell operations, skin loading details)
RUST_LOG=nihilsweeper=trace cargo run

# With debug logging (shows skin paths, board initialization)
RUST_LOG=nihilsweeper=debug cargo run
```

### Project-specific Commands
The codebase uses Rust edition 2024 with unstable features. See [rustfmt.toml](rustfmt.toml) for formatting rules (120 char width, Unix newlines, condensed wildcards).

### Skin Development
Add new skins to `assets/skin/<skin-name>/`:
1. Create `skin.toml` with config (see [assets/skin/wom-light/skin.toml](assets/skin/wom-light/skin.toml))
2. Add SVG files referenced in config (`closed.svg`, `type1.svg`-`type8.svg`, `face_unpressed.svg`, etc.)
3. Use scaling factors (e.g., `width_scaling: 0.666667`) to adapt to different cell sizes
4. Colors specified as hex integers: `0xc0c0c0` for RGB

## Critical Implementation Details

### Error Handling
Custom `Result<T>` type defined in [error.rs](src/error.rs). Errors include `IO`, `MissingResource`, `SkinNotFound`, `FileNotFound`, `Svg`, `Image`, `PixmapCreationFailed`, `Iced`. Use `inspect_err()` pattern seen in [mod.rs](src/ui/mod.rs) `MainWindow::new()` for logging.

### Resource Loading
[utils.rs](src/utils.rs) `resource_path()` checks `CARGO_MANIFEST_DIR/assets` first (for development), then `exe_dir/assets` (for distribution). Always use this for asset loading.

### Canvas Caching
[game.rs](src/ui/game.rs) uses `canvas::Cache` for `foreground_cache` and `background_cache`. Call `.clear()` to invalidate when board state changes.

### Flood Fill Algorithm
[board.rs](src/base/board.rs) `StandardBoard::open()` recursively opens adjacent cells when encountering `CellContent::Empty`. Guards against out-of-bounds with `nx >= 0 && nx < width` checks.

### Mine Initialization
First click calls `StandardBoard::init()` which shuffles mine positions excluding clicked cell and 8 neighbors. Uses `rand::seq::SliceRandom` to select mine indices.

## Testing & Debugging

Check for errors with `RUST_LOG=trace` to see detailed operation flow. Common issues:
- Skin not loading: Check `assets/skin/<name>/skin.toml` exists and SVG paths are correct
- Wrong cell size: Verify `config.cell_size` matches skin scaling expectations
- Chord not working: Check `chord_mode` in `GlobalConfig` matches expected behavior

## Dependencies
- `iced 0.14.0` with `canvas` and `image` features
- `resvg`, `usvg`, `tiny-skia` for SVG rendering
- `log`, `env_logger` for logging (enable with `RUST_LOG`)
- `rand 0.9.2` for mine shuffling
- `serde`, `toml` for skin config deserialization
