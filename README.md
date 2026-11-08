# Photo Resizer

A fast, lightweight batch image resizer for the web, written in Rust with
[iced](https://iced.rs). Drop a folder or dozens of photos, choose a folder
name, and get high-quality, web-friendly copies next to the originals.

## Features

- **Large batches** — drag in 50+ images or an entire folder tree. Files are
  discovered recursively and duplicates are ignored.
- **Parallel processing** — images are decoded and resized on a rayon pool while
  the window stays responsive.
- **Web-friendly output** — the longest edge is capped at **1920 px** and images
  are never enlarged. Resizing uses the **Lanczos3** filter.
- **Quality preserved** — original formats are kept, JPEGs are re-encoded at
  quality 92, and EXIF orientation is applied before resizing so phone photos
  are not sideways. Images that already fit are copied byte-for-byte.
- **Custom output folder** — enter any folder name; the folder is created for
  you. Existing files are never overwritten (collisions get ` (2)`, ` (3)`, …).
- **Clear feedback** — live per-file progress plus a summary, with unsupported
  files and permission errors reported instead of aborting the whole batch.
- **Compact UI** — Font Awesome icons and tooltips replace text-heavy controls;
  statuses are color-coded icons in the file list.
- **Sensible window size** — opens at about 60% of the primary display and is
  resizable and draggable. The window and executable use the custom app icon.

## Run

```powershell
cargo run --release
```

The first build downloads and compiles the GUI stack, so it takes a few minutes.
Subsequent builds are fast.

## Use

1. Drop photos (or folders) anywhere on the drop zone. Clicking it opens a file
   picker as an alternative.
2. Use the folder button on the right of **New folder** to choose where the new
   folder should be created. It defaults to the folder containing the first
   dropped file.
3. Enter a folder name. `web-ready` is prefilled.
4. Press **Resize N images** (or Enter in the name field).
5. When it finishes, press **Open folder** to reveal the results.

The window is also the drop target, so you can keep adding files at any time.
Hover the icon-only buttons (change location, clear, dismiss) to see tooltips.

## Supported formats

JPG, PNG, WebP, TIFF, BMP, ICO, TGA, QOI, DDS, HDR, Farbfeld, PNM, and other
raster formats supported by the `image` crate. GIFs are rejected because
resizing them would discard their animation. HEIC/AVIF are not enabled in this
build.

## App icon

The icon is generated from code, not hand-edited:

```powershell
py -m pip install pillow
py tools/make_icon.py
```

This writes `icon.png`, PNG sizes from 32 to 512 px, and a multi-resolution
`icon.ico` into `assets/icons/`. The PNG is used as the window icon at runtime,
the ICO is embedded into the Windows executable by `build.rs`, and the release
workflow includes the icon assets in every archive.

## Releases

`.github/workflows/release.yml` runs on `v*` tags (and can be started manually):

- tests on Linux first;
- builds release binaries for:
  - Linux x86_64,
  - Windows x86_64,
  - macOS Intel (x86_64),
  - macOS Apple Silicon (arm64);
- packages each binary with the README and icon assets;
- publishes a GitHub Release with all archives attached.

To cut a release:

```powershell
git tag v0.1.0
git push origin v0.1.0
```

## Project layout

```
src/main.rs                 Window sizing, app state, update/view logic, styling
src/processing.rs           Image discovery, resizing, encoding, naming, tests
build.rs                    Windows icon embedding
assets/icons/               Generated app icon (PNG sizes + ICO)
tools/make_icon.py          Python/Pillow icon generator
.github/workflows/release.yml
```

Build and test:

```powershell
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Notes

- Output filenames keep their original extension and format.
- Images that already fit within 1920 px are copied byte-for-byte, so their
  quality and metadata are untouched.
- The JPEG quality (92) and maximum edge (1920 px) are constants in
  `src/processing.rs` if you want to tune them.
- Icons come from the bundled `iced_font_awesome` package (Font Awesome 7), so
  no network request is made at runtime.
