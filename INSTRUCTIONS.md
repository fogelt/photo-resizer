### System Instructions for LLM Builder

**Role:** Expert Rust Developer specializing in GUI applications and image processing.

**Project Goal:**
Create a high-performance, lightweight Rust desktop application designed for batch-processing image files.

**Core Requirements:**
1. **Window Management:**
   - Use `winit` or `tauri` to detect the primary monitor resolution on launch.
   - Set the initial window size to a sensible, user-friendly proportion of the screen (e.g., 60% of the screen area).
   - Ensure the window is draggable and responsive.

2. **Core Functionality:**
   - **Drag-and-Drop:** Implement a file-drop zone that accepts large batches (e.g., 50+ images).
   - **Processing:** Integrate the `image` crate to handle resizing.
   - **Optimization:** Implement logic to resize images to "web-friendly" dimensions (e.g., max width 1920px) while maintaining aspect ratio and high visual quality (using Lanczos3 or similar filters).
   - **Output:** 
     - Prompt the user for a custom folder name.
     - Save the processed files into a new directory with the specified name.
     - Maintain original file formats where possible.

3. **Technical Stack Preferences:**
   - **Language:** Rust.
   - **GUI Framework:** Tauri (preferred for web-tech UI) or iced (for a pure Rust approach).
   - **Image Library:** `image` and `imageproc`.
   - **Concurrency:** Utilize `rayon` or `tokio` to process the batch of 50+ images in parallel to ensure the UI remains responsive.

4. **Design Philosophy:**
   - **Simplicity:** Keep the UI minimal—focus purely on the drag-drop zone and the output folder naming input.
   - **Performance:** Ensure low memory overhead despite processing large batches.
   - **Reliability:** Include basic error handling for unsupported file types or write-permission issues.

**Deliverables:**
- A `Cargo.toml` with necessary dependencies.
- A concise `main.rs` (and necessary modules) that handles the window lifecycle, file system interaction, and the image processing pipeline.
window that uses monitorsize to open a appropriate size window where users can drag edited photos and make them web-friendly in size without losing quality. main purpose to drag in big batches like 50 images and then make a folder with user-selected name to put them in.

### Final Build & Packaging (added)

**Release workflow:**
- Provide a GitHub Actions workflow (`.github/workflows/release.yml`) that runs on `v*` tags.
- Build and test on Linux, macOS (Intel and Apple Silicon), and Windows.
- Publish a GitHub Release containing native installers, not zip archives:
  Windows `.exe` (NSIS), macOS `.dmg`, Linux `.deb` and `.AppImage`.

**App icon:**
- Provide a Python script (`tools/make_icon.py`) that draws the app icon from scratch and exports PNG sizes plus a Windows `.ico`.
- Commit the generated icons under `assets/icons/`.
- Use the icon for the window at runtime and embed it in the Windows executable.

**Icon package / UI polish:**
- Use an icon package (Font Awesome via `iced_font_awesome`) instead of plain text where an icon is clearer.
- Prefer compact icon-only controls with tooltips for secondary actions (change folder, clear batch, dismiss).
- Replace text status badges with color-coded status icons so the file list stays clean and scannable.
- Keep the screen minimal: the drop zone, the folder name input, and the primary resize action should remain the focus.
