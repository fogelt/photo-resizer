//! Image discovery and the batch resizing pipeline.
//!
//! Everything in this module is independent from the GUI so it can be tested
//! without creating a window.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};

/// The longest edge of every resized image, in pixels.
pub const MAX_EDGE: u32 = 1920;

/// Default JPEG quality used when re-encoding photos.
pub const JPEG_QUALITY: u8 = 92;

/// Why a user-entered folder name cannot be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderNameError {
    Empty,
    NotAllowed,
    InvalidCharacters,
    TrailingDotOrSpace,
    TooLong,
    Reserved,
}

/// Why an image could not be processed.
#[derive(Debug, Clone)]
pub enum ProcessError {
    Open(String),
    Read(String),
    Unsupported,
    Gif,
    Decode(String),
    Copy(String),
    Encode(String),
    Panic,
}

/// A successfully processed file.
#[derive(Debug, Clone)]
pub struct ProcessedImage {
    pub source_width: u32,
    pub source_height: u32,
    pub width: u32,
    pub height: u32,
    pub source_bytes: u64,
    pub output_bytes: u64,
}

/// The files found after expanding a drop.
#[derive(Debug, Default)]
pub struct CollectedImages {
    pub images: Vec<PathBuf>,
    /// Files and folders that could not be read or are not images.
    pub skipped: usize,
}

/// Expands files and folders into a sorted, de-duplicated list of images.
pub fn collect_images(paths: &[PathBuf]) -> CollectedImages {
    let mut collected = CollectedImages::default();
    let mut seen = HashSet::new();

    for path in paths {
        visit(path, 0, &mut collected, &mut seen);
    }

    collected
        .images
        .sort_by_key(|path| path.file_name().map(|name| name.to_ascii_lowercase()));

    collected
}

fn visit(path: &Path, depth: usize, collected: &mut CollectedImages, seen: &mut HashSet<String>) {
    if depth > 16 {
        collected.skipped += 1;
        return;
    }

    if path.is_dir() {
        match fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    visit(&entry.path(), depth + 1, collected, seen);
                }
            }
            Err(_) => collected.skipped += 1,
        }
        return;
    }

    if !path.is_file() {
        collected.skipped += 1;
        return;
    }

    if !is_supported_extension(path) {
        collected.skipped += 1;
        return;
    }

    // Windows paths are case-insensitive; normalising also prevents duplicates
    // when someone drops both a folder and one of its files.
    let key = path.to_string_lossy().to_lowercase();
    if seen.insert(key) {
        collected.images.push(path.to_path_buf());
    } else {
        collected.skipped += 1;
    }
}

fn is_supported_extension(path: &Path) -> bool {
    matches!(
        ImageFormat::from_path(path),
        Ok(ImageFormat::Jpeg
            | ImageFormat::Png
            | ImageFormat::WebP
            | ImageFormat::Gif
            | ImageFormat::Bmp
            | ImageFormat::Tiff
            | ImageFormat::Ico
            | ImageFormat::Pnm
            | ImageFormat::Tga
            | ImageFormat::Dds
            | ImageFormat::Hdr
            | ImageFormat::Farbfeld
            | ImageFormat::Qoi)
    )
}

/// Calculates the output size, only ever shrinking an image.
pub fn fit_within(width: u32, height: u32, max_edge: u32) -> (u32, u32) {
    if width == 0 || height == 0 || max_edge == 0 || width.max(height) <= max_edge {
        return (width, height);
    }

    let (width64, height64, max_edge64) =
        (u64::from(width), u64::from(height), u64::from(max_edge));

    if width >= height {
        let scaled_height = ((height64 * max_edge64 + width64 / 2) / width64).max(1) as u32;
        (max_edge, scaled_height)
    } else {
        let scaled_width = ((width64 * max_edge64 + height64 / 2) / height64).max(1) as u32;
        (scaled_width, max_edge)
    }
}

/// Decodes, resizes, and re-encodes a single image.
pub fn process_image(
    input: &Path,
    output: &Path,
    max_edge: u32,
    jpeg_quality: u8,
) -> Result<ProcessedImage, ProcessError> {
    let reader = ImageReader::open(input)
        .map_err(|error| ProcessError::Open(error.to_string()))?
        .with_guessed_format()
        .map_err(|error| ProcessError::Read(error.to_string()))?;

    let source_format = reader.format().ok_or(ProcessError::Unsupported)?;
    let source_bytes = fs::metadata(input)
        .map(|metadata| metadata.len())
        .unwrap_or_default();

    if source_format == ImageFormat::Gif {
        return Err(ProcessError::Gif);
    }

    let mut decoder = reader
        .into_decoder()
        .map_err(|error| ProcessError::Decode(error.to_string()))?;

    // Cameras store rotation in EXIF instead of rotating the pixels. Read the
    // orientation first so the dimensions we reason about match what a browser
    // will display.
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let (raw_width, raw_height) = decoder.dimensions();
    let (source_width, source_height) = if rotates_dimensions(orientation) {
        (raw_height, raw_width)
    } else {
        (raw_width, raw_height)
    };
    let (width, height) = fit_within(source_width, source_height, max_edge);
    let output_format = ImageFormat::from_path(output).unwrap_or(source_format);

    let quality_differs = output_format == ImageFormat::Jpeg && jpeg_quality != JPEG_QUALITY;

    if (width, height) == (source_width, source_height)
        && output_format == source_format
        && !quality_differs
    {
        // Nothing to resize: copying the original bytes avoids a generation of
        // JPEG loss and keeps the file size (and metadata) exactly as it was.
        drop(decoder);
        fs::copy(input, output).map_err(|error| ProcessError::Copy(error.to_string()))?;

        let output_bytes = fs::metadata(output)
            .map(|metadata| metadata.len())
            .unwrap_or_default();

        return Ok(ProcessedImage {
            source_width,
            source_height,
            width,
            height,
            source_bytes,
            output_bytes,
        });
    }

    let mut image = DynamicImage::from_decoder(decoder)
        .map_err(|error| ProcessError::Decode(error.to_string()))?;
    image.apply_orientation(orientation);

    if (width, height) != (source_width, source_height) {
        image = image.resize_exact(width, height, FilterType::Lanczos3);
    }

    save_image(&image, output, output_format, jpeg_quality)?;

    let output_bytes = fs::metadata(output)
        .map(|metadata| metadata.len())
        .unwrap_or_default();

    Ok(ProcessedImage {
        source_width,
        source_height,
        width,
        height,
        source_bytes,
        output_bytes,
    })
}

fn rotates_dimensions(orientation: image::metadata::Orientation) -> bool {
    use image::metadata::Orientation;

    matches!(
        orientation,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    )
}

fn save_image(
    image: &DynamicImage,
    output: &Path,
    format: ImageFormat,
    jpeg_quality: u8,
) -> Result<(), ProcessError> {
    match format {
        ImageFormat::Jpeg => {
            // JPEG has no alpha channel; flatten to RGB before encoding.
            let rgb = image.to_rgb8();
            let file =
                File::create(output).map_err(|error| ProcessError::Encode(error.to_string()))?;
            let mut encoder =
                JpegEncoder::new_with_quality(BufWriter::new(file), jpeg_quality.clamp(1, 100));
            encoder
                .encode_image(&rgb)
                .map_err(|error| ProcessError::Encode(error.to_string()))
        }
        _ => image
            .save_with_format(output, format)
            .map_err(|error| ProcessError::Encode(error.to_string())),
    }
}

/// Picks a file name that does not overwrite an existing file.
///
/// `taken` contains lower-cased names already reserved during this batch.
pub fn unique_output_path(dir: &Path, file_name: &str, taken: &mut HashSet<String>) -> PathBuf {
    let source = Path::new(file_name);
    let stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("image");
    let extension = source.extension().and_then(|extension| extension.to_str());

    for number in 1..=10_000 {
        let candidate = match (number, extension) {
            (1, Some(extension)) => format!("{stem}.{extension}"),
            (1, None) => stem.to_owned(),
            (_, Some(extension)) => format!("{stem} ({number}).{extension}"),
            (_, None) => format!("{stem} ({number})"),
        };

        let key = candidate.to_lowercase();
        let path = dir.join(&candidate);

        if !taken.contains(&key) && !path.exists() {
            taken.insert(key);
            return path;
        }
    }

    // Practically unreachable, but never loop forever.
    let fallback = format!("{stem}-{}", std::process::id());
    taken.insert(fallback.to_lowercase());
    dir.join(fallback)
}

/// Checks a user-entered folder name and returns it without surrounding space.
pub fn validate_folder_name(name: &str) -> Result<String, FolderNameError> {
    let name = name.trim();

    if name.is_empty() {
        return Err(FolderNameError::Empty);
    }

    if name == "." || name == ".." {
        return Err(FolderNameError::NotAllowed);
    }

    if name.chars().any(|character| {
        character.is_control()
            || matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            )
    }) {
        return Err(FolderNameError::InvalidCharacters);
    }

    if name.ends_with('.') || name.ends_with(' ') {
        return Err(FolderNameError::TrailingDotOrSpace);
    }

    if name.chars().count() > 120 {
        return Err(FolderNameError::TooLong);
    }

    // CON, NUL, COM1, etc. are reserved on Windows even when an extension is
    // added, so compare the part before the first dot.
    let reserved_stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    let is_reserved = matches!(reserved_stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || reserved_stem
            .strip_prefix("COM")
            .and_then(|number| number.parse::<u8>().ok())
            .is_some_and(|number| (1..=9).contains(&number))
        || reserved_stem
            .strip_prefix("LPT")
            .and_then(|number| number.parse::<u8>().ok())
            .is_some_and(|number| (1..=9).contains(&number));

    if is_reserved {
        return Err(FolderNameError::Reserved);
    }

    Ok(name.to_owned())
}

/// Formats a byte count for the file list.
pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;

    let bytes = bytes as f64;

    if bytes >= MIB {
        format!("{:.1} MB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.0} KB", bytes / KIB)
    } else {
        format!("{bytes:.0} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    #[test]
    fn fit_within_shrinks_landscape_images() {
        assert_eq!(fit_within(4000, 3000, 1920), (1920, 1440));
    }

    #[test]
    fn fit_within_shrinks_portrait_images() {
        assert_eq!(fit_within(3000, 4000, 1920), (1440, 1920));
    }

    #[test]
    fn fit_within_never_enlarges_images() {
        assert_eq!(fit_within(800, 600, 1920), (800, 600));
        assert_eq!(fit_within(1920, 1080, 1920), (1920, 1080));
    }

    #[test]
    fn fit_within_keeps_tiny_dimension() {
        assert_eq!(fit_within(10_000, 1, 1920), (1920, 1));
    }

    #[test]
    fn folder_names_are_validated() {
        assert_eq!(
            validate_folder_name("  web-ready  "),
            Ok("web-ready".to_owned())
        );
        assert!(validate_folder_name("").is_err());
        assert!(validate_folder_name("   ").is_err());
        assert!(validate_folder_name("bad/name").is_err());
        assert!(validate_folder_name("bad:name").is_err());
        assert!(validate_folder_name("trailing.").is_err());
        assert!(validate_folder_name("CON").is_err());
        assert!(validate_folder_name("lpt9.images").is_err());
        assert!(validate_folder_name("console").is_ok());
    }

    #[test]
    fn output_names_do_not_collide() {
        let temp = tempfile::tempdir().unwrap();
        let mut taken = HashSet::new();

        let first = unique_output_path(temp.path(), "photo.jpg", &mut taken);
        let second = unique_output_path(temp.path(), "PHOTO.JPG", &mut taken);
        let third = unique_output_path(temp.path(), "photo.jpg", &mut taken);

        assert_eq!(first.file_name().unwrap(), "photo.jpg");
        assert_eq!(second.file_name().unwrap(), "PHOTO (2).JPG");
        assert_eq!(third.file_name().unwrap(), "photo (3).jpg");
    }

    #[test]
    fn output_names_avoid_existing_files() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("photo.png"), b"existing").unwrap();

        let mut taken = HashSet::new();
        let path = unique_output_path(temp.path(), "photo.png", &mut taken);

        assert_eq!(path.file_name().unwrap(), "photo (2).png");
    }

    #[test]
    fn collects_images_recursively() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("nested");
        fs::create_dir(&nested).unwrap();
        fs::write(temp.path().join("one.png"), b"not really an image").unwrap();
        fs::write(nested.join("two.JPG"), b"not really an image").unwrap();
        fs::write(temp.path().join("notes.txt"), b"ignore me").unwrap();

        let collected = collect_images(&[temp.path().to_path_buf()]);

        assert_eq!(collected.images.len(), 2);
        assert_eq!(collected.skipped, 1);
    }

    #[test]
    fn processes_a_real_image() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("large.png");
        let output = temp.path().join("web-ready.png");

        let mut source = RgbImage::new(3000, 2000);
        source.put_pixel(0, 0, Rgb([255, 0, 0]));
        source.save(&input).unwrap();

        let result = process_image(&input, &output, 1920, 92).unwrap();

        assert_eq!((result.source_width, result.source_height), (3000, 2000));
        assert_eq!((result.width, result.height), (1920, 1280));
        assert!(output.exists());
        assert!(result.output_bytes > 0);
    }

    #[test]
    fn leaves_small_images_untouched() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("small.jpg");
        let output = temp.path().join("copy.jpg");

        RgbImage::new(640, 480).save(&input).unwrap();
        let result = process_image(&input, &output, 1920, 92).unwrap();

        assert_eq!((result.width, result.height), (640, 480));
        assert_eq!(
            fs::read(&input).unwrap(),
            fs::read(&output).unwrap(),
            "images that need no resizing should be copied without re-encoding"
        );
    }
}
