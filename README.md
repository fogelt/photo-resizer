# Photo Resizer

A fast desktop app for turning batches of photos into web-ready copies without
losing quality. Drop in 50+ images, choose a folder name, and go.

## Download

Grab the latest installer for your platform from the
[Releases page](https://github.com/fogelt/photo-resizer/releases):

- **Windows** — `.exe` installer
- **macOS** — `.dmg` (drag the app into Applications)
- **Linux** — `.deb` package or `.AppImage`

## Use

1. Drop photos or whole folders onto the window. Click the drop zone to browse
   instead.
2. Choose where to save the new folder with the folder button next to
   **New folder**.
3. Type a folder name — `web-ready` is prefilled — then press
   **Resize N images** (or Enter).
4. Press **Open folder** when it finishes.

You can keep adding files while the app is open. Use the flags in the top-right
to switch between English and Svenska; the choice is remembered.

## What it does

- Caps the longest edge at **1920 px** and never enlarges images.
- Keeps the original format and uses high-quality Lanczos3 resizing.
- Re-encodes JPEGs at quality 92.
- Fixes phone photos that are rotated with EXIF orientation.
- Copies images that already fit byte-for-byte, so they stay untouched.
- Creates a new folder with your chosen name and never overwrites existing
  files (duplicates get ` (2)`, ` (3)`, and so on).
- Processes images in parallel and reports progress, skipped files, and errors
  without stopping the whole batch.

## Supported formats

JPG, PNG, WebP, TIFF, BMP, ICO, TGA, QOI, DDS, HDR, Farbfeld, and PNM.

GIFs are not resized because that would drop their animation. HEIC and AVIF are
not supported.
