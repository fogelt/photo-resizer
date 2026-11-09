//! Translation strings and the small set of persisted user preferences.

use std::fs;
use std::path::PathBuf;

use crate::processing::{FolderNameError, ProcessError, JPEG_QUALITY};

const CONFIG_DIR: &str = "photo-resizer";
const CONFIG_FILE: &str = "settings.conf";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    English,
    Swedish,
}

impl Language {
    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Swedish => "sv",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code.trim().to_ascii_lowercase().as_str() {
            "en" | "english" => Some(Self::English),
            "sv" | "svenska" | "swedish" => Some(Self::Swedish),
            _ => None,
        }
    }

    pub fn load() -> Self {
        let Some(path) = config_path() else {
            return Self::English;
        };

        let Ok(contents) = fs::read_to_string(path) else {
            return Self::English;
        };

        contents
            .lines()
            .filter_map(|line| line.trim().strip_prefix("language="))
            .find_map(Self::from_code)
            .unwrap_or(Self::English)
    }

    pub fn save(self) {
        write_setting("language", self.code());
    }

    pub fn title(self) -> &'static str {
        "Photo Resizer"
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Self::English => "Drop a batch. Pick a folder name. Get web-ready copies.",
            Self::Swedish => "Släpp in en batch. Välj ett mappnamn. Få webbklara kopior.",
        }
    }

    pub fn drop_hover(self) -> &'static str {
        match self {
            Self::English => "Let go to add these photos",
            Self::Swedish => "Släpp för att lägga till bilderna",
        }
    }

    pub fn drop_idle(self) -> &'static str {
        match self {
            Self::English => "Drop your photos here",
            Self::Swedish => "Släpp dina bilder här",
        }
    }

    pub fn drop_more(self) -> &'static str {
        match self {
            Self::English => "Drop more photos",
            Self::Swedish => "Släpp fler bilder",
        }
    }

    pub fn drop_hint_empty(self) -> &'static str {
        match self {
            Self::English => "Drag in a batch of images or click to browse · JPG, PNG, WebP, TIFF, BMP and more",
            Self::Swedish => "Dra in en batch bilder eller klicka för att bläddra · JPG, PNG, WebP, TIFF, BMP med flera",
        }
    }

    pub fn drop_hint_more(self) -> &'static str {
        match self {
            Self::English => "Drag files or folders in at any time · click to browse",
            Self::Swedish => "Dra in filer eller mappar när som helst · klicka för att bläddra",
        }
    }

    pub fn batch(self) -> &'static str {
        match self {
            Self::English => "Batch",
            Self::Swedish => "Bilder",
        }
    }

    pub fn finished_count(self, done: usize, total: usize) -> String {
        match self {
            Self::English => format!("{done} of {total} finished"),
            Self::Swedish => format!("{done} av {total} klara"),
        }
    }

    pub fn clear_batch(self) -> &'static str {
        match self {
            Self::English => "Clear batch",
            Self::Swedish => "Rensa batchen",
        }
    }

    pub fn waiting(self) -> &'static str {
        match self {
            Self::English => "Waiting",
            Self::Swedish => "Väntar",
        }
    }

    pub fn waiting_in(self, parent: &str) -> String {
        match self {
            Self::English => format!("{parent} · waiting"),
            Self::Swedish => format!("{parent} · väntar"),
        }
    }

    pub fn working(self) -> &'static str {
        match self {
            Self::English => "Resizing with Lanczos3…",
            Self::Swedish => "Ändrar storlek med Lanczos3…",
        }
    }

    pub fn open_folder(self) -> &'static str {
        match self {
            Self::English => "Open folder",
            Self::Swedish => "Öppna mappen",
        }
    }

    pub fn dismiss(self) -> &'static str {
        match self {
            Self::English => "Dismiss",
            Self::Swedish => "Stäng",
        }
    }

    pub fn new_folder(self) -> &'static str {
        match self {
            Self::English => "New folder",
            Self::Swedish => "Ny mapp",
        }
    }

    pub fn no_location(self) -> &'static str {
        match self {
            Self::English => "No location selected yet",
            Self::Swedish => "Ingen plats vald ännu",
        }
    }

    pub fn choose_folder(self) -> &'static str {
        match self {
            Self::English => "Choose output folder",
            Self::Swedish => "Välj utmapp",
        }
    }

    pub fn folder_placeholder(self) -> &'static str {
        match self {
            Self::English => "Folder name",
            Self::Swedish => "Mappnamn",
        }
    }

    pub fn quality(self) -> &'static str {
        match self {
            Self::English => "JPEG quality",
            Self::Swedish => "JPEG-kvalitet",
        }
    }

    pub fn reset_quality(self) -> &'static str {
        match self {
            Self::English => "Reset quality",
            Self::Swedish => "Återställ kvaliteten",
        }
    }

    pub fn resizing(self) -> &'static str {
        match self {
            Self::English => "Resizing…",
            Self::Swedish => "Ändrar storlek…",
        }
    }

    pub fn resize_images(self) -> &'static str {
        match self {
            Self::English => "Resize images",
            Self::Swedish => "Ändra storlek",
        }
    }

    pub fn english(self) -> &'static str {
        "English"
    }

    pub fn swedish(self) -> &'static str {
        "Svenska"
    }

    pub fn add_photos_title(self) -> &'static str {
        match self {
            Self::English => "Add photos",
            Self::Swedish => "Lägg till bilder",
        }
    }

    pub fn images_filter(self) -> &'static str {
        match self {
            Self::English => "Images",
            Self::Swedish => "Bilder",
        }
    }

    pub fn choose_output_title(self) -> &'static str {
        match self {
            Self::English => "Choose where to save the new folder",
            Self::Swedish => "Välj var den nya mappen ska sparas",
        }
    }

    pub fn images_count(self, count: usize) -> String {
        match self {
            Self::English => format!("{count} image{}", plural_en(count)),
            Self::Swedish => format!("{count} bild{}", plural_sv(count)),
        }
    }

    pub fn resize_n(self, count: usize) -> String {
        match self {
            Self::English => format!("Resize {count} image{}", plural_en(count)),
            Self::Swedish => format!("Ändra storlek på {count} bild{}", plural_sv(count)),
        }
    }

    pub fn progress(self, done: usize, total: usize) -> String {
        match self {
            Self::English => format!("Resizing {done} of {total} images…"),
            Self::Swedish => format!("Ändrar storlek på {done} av {total} bilder…"),
        }
    }

    pub fn finished(self, count: usize, seconds: f32) -> String {
        match self {
            Self::English => format!(
                "Finished {count} image{} in {seconds:.1}s",
                plural_en(count)
            ),
            Self::Swedish => format!(
                "Klart med {count} bild{} på {} s",
                plural_sv(count),
                format!("{seconds:.1}").replace('.', ",")
            ),
        }
    }

    pub fn failed_count(self, count: usize) -> String {
        match self {
            Self::English => format!("{count} failed"),
            Self::Swedish => format!("{count} misslyckades"),
        }
    }

    pub fn ready_failed(self, ready: usize, failed: usize) -> String {
        match self {
            Self::English => format!("{ready} ready · {failed} failed"),
            Self::Swedish => format!("{ready} klara · {failed} misslyckade"),
        }
    }

    pub fn ready(self, count: usize) -> String {
        match self {
            Self::English => format!("{count} ready"),
            Self::Swedish => format!("{count} klara"),
        }
    }

    pub fn added_skipped(self, added: usize, skipped: usize) -> String {
        match self {
            Self::English => format!(
                "Added {added} image{}. Skipped {skipped} unsupported or unreadable item{}.",
                plural_en(added),
                plural_en(skipped)
            ),
            Self::Swedish => format!(
                "Lade till {added} bild{}. Hoppade över {skipped} objekt som inte stöds eller inte kunde läsas.",
                plural_sv(added)
            ),
        }
    }

    pub fn no_supported_images(self) -> &'static str {
        match self {
            Self::English => "No supported images were found. Try JPG, PNG, WebP, TIFF, BMP, or another raster format.",
            Self::Swedish => "Inga bilder som stöds hittades. Prova JPG, PNG, WebP, TIFF, BMP eller ett annat rasterformat.",
        }
    }

    pub fn already_in_list(self) -> &'static str {
        match self {
            Self::English => "Those files are already in the list.",
            Self::Swedish => "Filerna finns redan i listan.",
        }
    }

    pub fn choose_location_first(self) -> &'static str {
        match self {
            Self::English => "Choose where the new folder should be created first.",
            Self::Swedish => "Välj först var den nya mappen ska skapas.",
        }
    }

    pub fn folder_exists(self, path: &str) -> String {
        match self {
            Self::English => {
                format!("“{path}” already exists as a file. Pick another folder name.")
            }
            Self::Swedish => format!("”{path}” finns redan som en fil. Välj ett annat mappnamn."),
        }
    }

    pub fn folder_create_failed(self, path: &str, error: &str) -> String {
        match self {
            Self::English => format!("Could not create “{path}”: {error}"),
            Self::Swedish => format!("Kunde inte skapa ”{path}”: {error}"),
        }
    }

    pub fn folder_error(self, error: FolderNameError) -> &'static str {
        match (self, error) {
            (Self::English, FolderNameError::Empty) => "Enter a folder name.",
            (Self::English, FolderNameError::NotAllowed) => "That name is not allowed.",
            (Self::English, FolderNameError::InvalidCharacters) => {
                r#"Folder names cannot contain < > : " / \ | ? *"#
            }
            (Self::English, FolderNameError::TrailingDotOrSpace) => {
                "Folder names cannot end with a dot or a space."
            }
            (Self::English, FolderNameError::TooLong) => {
                "Use a shorter folder name (120 characters or fewer)."
            }
            (Self::English, FolderNameError::Reserved) => {
                "That name is reserved by Windows. Pick another one."
            }
            (Self::Swedish, FolderNameError::Empty) => "Ange ett mappnamn.",
            (Self::Swedish, FolderNameError::NotAllowed) => "Det namnet är inte tillåtet.",
            (Self::Swedish, FolderNameError::InvalidCharacters) => {
                r#"Mappnamn får inte innehålla < > : " / \ | ? *"#
            }
            (Self::Swedish, FolderNameError::TrailingDotOrSpace) => {
                "Mappnamn får inte sluta med en punkt eller ett mellanslag."
            }
            (Self::Swedish, FolderNameError::TooLong) => {
                "Använd ett kortare mappnamn (högst 120 tecken)."
            }
            (Self::Swedish, FolderNameError::Reserved) => {
                "Namnet är reserverat av Windows. Välj ett annat."
            }
        }
    }

    pub fn process_error(self, error: &ProcessError) -> String {
        match self {
            Self::English => match error {
                ProcessError::Open(detail) => format!("Could not open the file: {detail}"),
                ProcessError::Read(detail) => format!("Could not read the file: {detail}"),
                ProcessError::Unsupported => "Unsupported image format.".to_owned(),
                ProcessError::Gif => "GIFs are not resized because doing so would drop their animation. Export a PNG or JPEG first.".to_owned(),
                ProcessError::Decode(detail) => format!("Could not decode the image: {detail}"),
                ProcessError::Copy(detail) => format!("Could not copy the image to the output folder: {detail}"),
                ProcessError::Encode(detail) => format!("Could not save the image: {detail}"),
                ProcessError::Panic => "Unexpected error while processing this image.".to_owned(),
            },
            Self::Swedish => match error {
                ProcessError::Open(detail) => format!("Kunde inte öppna filen: {detail}"),
                ProcessError::Read(detail) => format!("Kunde inte läsa filen: {detail}"),
                ProcessError::Unsupported => "Bildformatet stöds inte.".to_owned(),
                ProcessError::Gif => "GIF-bilder ändras inte eftersom animeringen då skulle försvinna. Exportera som PNG eller JPEG först.".to_owned(),
                ProcessError::Decode(detail) => format!("Kunde inte avkoda bilden: {detail}"),
                ProcessError::Copy(detail) => format!("Kunde inte kopiera bilden till utmappen: {detail}"),
                ProcessError::Encode(detail) => format!("Kunde inte spara bilden: {detail}"),
                ProcessError::Panic => "Oväntat fel när bilden bearbetades.".to_owned(),
            },
        }
    }
}

/// Loads the remembered JPEG quality, falling back to the default.
pub fn load_quality() -> u8 {
    let Some(path) = config_path() else {
        return JPEG_QUALITY;
    };

    let Ok(contents) = fs::read_to_string(path) else {
        return JPEG_QUALITY;
    };

    contents
        .lines()
        .filter_map(|line| line.trim().strip_prefix("quality="))
        .find_map(|value| value.trim().parse::<u8>().ok())
        .map(|quality| quality.clamp(1, 100))
        .unwrap_or(JPEG_QUALITY)
}

/// Remembers the chosen JPEG quality without disturbing other settings.
pub fn save_quality(quality: u8) {
    write_setting("quality", &quality.clamp(1, 100).to_string());
}

fn write_setting(key: &str, value: &str) {
    let Some(path) = config_path() else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut lines: Vec<String> = fs::read_to_string(&path)
        .map(|contents| contents.lines().map(str::to_owned).collect())
        .unwrap_or_default();

    let prefix = format!("{key}=");
    let mut replaced = false;
    lines.retain_mut(|line| {
        if line.trim_start().starts_with(&prefix) {
            if replaced {
                return false;
            }

            *line = format!("{key}={value}");
            replaced = true;
        }

        true
    });

    if !replaced {
        lines.push(format!("{key}={value}"));
    }

    let mut contents = lines.join("\n");
    contents.push('\n');
    let _ = fs::write(path, contents);
}

fn plural_en(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

fn plural_sv(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "er"
    }
}

fn config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(|base| PathBuf::from(base).join(CONFIG_DIR).join(CONFIG_FILE))
    }

    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|base| {
            PathBuf::from(base)
                .join("Library")
                .join("Application Support")
                .join(CONFIG_DIR)
                .join(CONFIG_FILE)
        })
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|base| PathBuf::from(base).join(".config")))
            .map(|base| base.join(CONFIG_DIR).join(CONFIG_FILE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_language_codes() {
        assert_eq!(Language::from_code("en"), Some(Language::English));
        assert_eq!(Language::from_code("SV"), Some(Language::Swedish));
        assert_eq!(Language::from_code("de"), None);
    }

    #[test]
    fn formats_swedish_plurals() {
        assert_eq!(Language::Swedish.images_count(1), "1 bild");
        assert_eq!(Language::Swedish.images_count(3), "3 bilder");
        assert_eq!(
            Language::Swedish.finished(2, 1.25),
            "Klart med 2 bilder på 1,2 s"
        );
    }
}
