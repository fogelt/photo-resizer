#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

//! Photo Resizer — a small, fast batch image resizer for the web.
//!
//! Drop a batch of photos onto the window, choose a folder name, and get
//! high-quality, web-friendly copies written next to the originals.

mod i18n;
mod processing;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use display_info::DisplayInfo;
use i18n::{load_quality, save_quality, Language};
use iced::futures::channel::mpsc::unbounded;
use iced::widget::image::Handle as ImageHandle;
use iced::widget::{
    button, column, container, mouse_area, progress_bar, row, scrollable, slider, text, text_input,
    tooltip, Space,
};
use iced::{
    border, theme, window, Alignment, Border, Color, Element, Length, Shadow, Size, Subscription,
    Task, Theme,
};
use iced_font_awesome::fa_icon_solid;
use processing::{
    collect_images, format_bytes, process_image, unique_output_path, validate_folder_name,
    FolderNameError, ProcessError, ProcessedImage, JPEG_QUALITY, MAX_EDGE,
};

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

const BACKGROUND: Color = Color::from_rgb8(10, 14, 24);
const SURFACE: Color = Color::from_rgb8(18, 25, 41);
const SURFACE_ALT: Color = Color::from_rgb8(27, 37, 58);
const SURFACE_HOVER: Color = Color::from_rgb8(35, 48, 74);
const BORDER: Color = Color::from_rgb8(48, 63, 94);
const TEXT: Color = Color::from_rgb8(232, 238, 250);
const MUTED: Color = Color::from_rgb8(144, 157, 184);
const ACCENT: Color = Color::from_rgb8(96, 130, 255);
const ACCENT_HOVER: Color = Color::from_rgb8(122, 152, 255);
const ACCENT_PRESSED: Color = Color::from_rgb8(78, 110, 232);
const SUCCESS: Color = Color::from_rgb8(72, 209, 150);
const WARNING: Color = Color::from_rgb8(255, 191, 92);
const DANGER: Color = Color::from_rgb8(255, 112, 132);
const DANGER_DIM: Color = Color::from_rgb8(55, 26, 37);

const FLAG_EN: &[u8] = include_bytes!("../assets/flags/en.png");
const FLAG_SV: &[u8] = include_bytes!("../assets/flags/sv.png");

static FLAG_EN_HANDLE: LazyLock<ImageHandle> = LazyLock::new(|| flag_handle(FLAG_EN));
static FLAG_SV_HANDLE: LazyLock<ImageHandle> = LazyLock::new(|| flag_handle(FLAG_SV));

fn flag_handle(bytes: &[u8]) -> ImageHandle {
    let image = image::load_from_memory(bytes)
        .expect("bundled flag images must be valid PNGs")
        .into_rgba8();
    let (width, height) = (image.width(), image.height());
    ImageHandle::from_rgba(width, height, image.into_raw())
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

fn main() -> iced::Result {
    // Decoding large photos is memory-hungry. Eight workers is plenty to keep
    // a fast batch moving without spiking RAM on high-core-count machines.
    let workers = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4)
        .clamp(2, 8);
    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build_global();

    iced::application(App::default, update, view)
        .title("Photo Resizer")
        .theme(app_theme())
        .window(window::Settings {
            size: initial_window_size(),
            min_size: Some(Size::new(880.0, 620.0)),
            position: window::Position::Centered,
            icon: window_icon(),
            ..window::Settings::default()
        })
        .subscription(subscription)
        .style(|_state, _theme| theme::Style {
            background_color: BACKGROUND,
            text_color: TEXT,
        })
        .antialiasing(true)
        .run()
}

/// Opens the window at roughly 60% of the primary display and never smaller
/// than a comfortable editing size.
fn initial_window_size() -> Size {
    let fallback = Size::new(1180.0, 780.0);

    let Ok(displays) = DisplayInfo::all() else {
        return fallback;
    };

    let Some(display) = displays
        .iter()
        .find(|display| display.is_primary)
        .or_else(|| displays.first())
    else {
        return fallback;
    };

    let scale = if display.scale_factor > 0.0 {
        display.scale_factor
    } else {
        1.0
    };

    let width = display.width as f32 / scale * 0.6;
    let height = display.height as f32 / scale * 0.6;

    Size::new(width.max(960.0), height.max(680.0))
}

/// Loads the app icon that ships with the binary.
fn window_icon() -> Option<window::Icon> {
    let bytes = include_bytes!("../assets/icons/icon-256.png");
    let image = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = (image.width(), image.height());
    window::icon::from_rgba(image.into_raw(), width, height).ok()
}

struct App {
    files: Vec<PhotoFile>,
    language: Language,
    quality: u8,
    output_base: Option<PathBuf>,
    folder_name: String,
    folder_error: Option<FolderNameError>,
    notice: Option<String>,
    hovered: bool,
    processing: bool,
    spinner_frame: usize,
    completed: usize,
    failed: usize,
    batch_total: usize,
    started_at: Option<Instant>,
    elapsed: Option<Duration>,
    last_output_dir: Option<PathBuf>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            language: Language::load(),
            quality: load_quality(),
            output_base: None,
            folder_name: "web-ready".to_owned(),
            folder_error: None,
            notice: None,
            hovered: false,
            processing: false,
            spinner_frame: 0,
            completed: 0,
            failed: 0,
            batch_total: 0,
            started_at: None,
            elapsed: None,
            last_output_dir: None,
        }
    }
}

struct PhotoFile {
    path: PathBuf,
    status: FileStatus,
}

enum FileStatus {
    Pending,
    Working,
    Done(ProcessedImage),
    Failed(ProcessError),
}

#[derive(Debug, Clone)]
enum Message {
    FilesDropped(Vec<PathBuf>),
    HoverChanged(bool),
    Browse,
    ChooseOutput,
    FolderNameChanged(String),
    QualityChanged(u8),
    ResetQuality,
    SetLanguage(Language),
    Start,
    FileProcessed(usize, Result<ProcessedImage, ProcessError>),
    Finished,
    Tick,
    Clear,
    OpenOutput,
    DismissNotice,
}

impl App {
    fn add_paths(&mut self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }

        if self.output_base.is_none() {
            self.output_base = output_base_for_drop(&paths);
        }

        let collected = collect_images(&paths);
        let existing: HashSet<String> = self
            .files
            .iter()
            .map(|file| file.path.to_string_lossy().to_lowercase())
            .collect();

        let mut added = 0;
        for path in collected.images {
            let key = path.to_string_lossy().to_lowercase();
            if existing.contains(&key) {
                continue;
            }

            self.files.push(PhotoFile {
                path,
                status: FileStatus::Pending,
            });
            added += 1;
        }

        if added == 0 {
            self.notice = Some(if self.files.is_empty() {
                self.language.no_supported_images().to_owned()
            } else {
                self.language.already_in_list().to_owned()
            });
        } else if collected.skipped > 0 {
            self.notice = Some(self.language.added_skipped(added, collected.skipped));
        } else {
            self.notice = None;
        }
    }

    fn start(&mut self) -> Task<Message> {
        if self.processing || self.files.is_empty() {
            return Task::none();
        }

        let folder_name = match validate_folder_name(&self.folder_name) {
            Ok(name) => name,
            Err(error) => {
                self.folder_error = Some(error);
                return Task::none();
            }
        };
        self.folder_error = None;

        let Some(base) = self.output_base.clone() else {
            self.notice = Some(self.language.choose_location_first().to_owned());
            return Task::none();
        };

        let output_dir = base.join(folder_name);

        if output_dir.exists() && !output_dir.is_dir() {
            self.notice = Some(
                self.language
                    .folder_exists(&output_dir.display().to_string()),
            );
            return Task::none();
        }

        if let Err(error) = std::fs::create_dir_all(&output_dir) {
            self.notice = Some(
                self.language
                    .folder_create_failed(&output_dir.display().to_string(), &error.to_string()),
            );
            return Task::none();
        }

        let mut taken = HashSet::new();
        let jobs: Vec<Job> = self
            .files
            .iter()
            .enumerate()
            .map(|(index, file)| {
                let file_name = file
                    .path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "image".to_owned());

                Job {
                    index,
                    input: file.path.clone(),
                    output: unique_output_path(&output_dir, &file_name, &mut taken),
                }
            })
            .collect();

        self.processing = true;
        self.completed = 0;
        self.failed = 0;
        self.batch_total = self.files.len();
        self.started_at = Some(Instant::now());
        self.elapsed = None;
        self.last_output_dir = Some(output_dir);
        self.notice = None;

        let quality = self.quality;

        for file in &mut self.files {
            file.status = FileStatus::Working;
        }

        let (sender, receiver) = unbounded();

        // Decoding and resizing happen on rayon's pool, so the UI thread stays
        // free to paint the progress bar while a large batch is processed.
        std::thread::spawn(move || {
            use rayon::prelude::*;

            jobs.par_iter().for_each(|job| {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    process_image(&job.input, &job.output, MAX_EDGE, quality)
                }))
                .unwrap_or(Err(ProcessError::Panic));

                let _ = sender.unbounded_send(Message::FileProcessed(job.index, result));
            });

            let _ = sender.unbounded_send(Message::Finished);
        });

        Task::stream(receiver)
    }

    fn finish(&mut self) {
        if !self.processing {
            return;
        }

        self.processing = false;
        self.elapsed = self.started_at.take().map(|started| started.elapsed());

        // This only matters if the worker thread died unexpectedly.
        for file in &mut self.files {
            if matches!(file.status, FileStatus::Working) {
                self.failed += 1;
                file.status = FileStatus::Failed(ProcessError::Panic);
            }
        }

        if self.completed < self.batch_total {
            self.completed = self.batch_total;
        }
    }
}

#[derive(Debug)]
struct Job {
    index: usize,
    input: PathBuf,
    output: PathBuf,
}

fn output_base_for_drop(paths: &[PathBuf]) -> Option<PathBuf> {
    let first = paths.first()?;

    first
        .parent()
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok())
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::FilesDropped(paths) => {
            app.hovered = false;
            app.add_paths(paths);
            Task::none()
        }
        Message::HoverChanged(hovered) => {
            app.hovered = hovered;
            Task::none()
        }
        Message::Browse => {
            let picked = rfd::FileDialog::new()
                .set_title(app.language.add_photos_title())
                .add_filter(
                    app.language.images_filter(),
                    &[
                        "jpg", "jpeg", "png", "webp", "tif", "tiff", "bmp", "ico", "tga", "qoi",
                        "dds", "hdr", "ff", "pnm", "pbm", "pgm", "ppm", "pam", "gif",
                    ],
                )
                .pick_files();

            if let Some(paths) = picked {
                app.add_paths(paths);
            }

            Task::none()
        }
        Message::ChooseOutput => {
            let mut dialog = rfd::FileDialog::new().set_title(app.language.choose_output_title());

            if let Some(base) = &app.output_base {
                dialog = dialog.set_directory(base);
            }

            if let Some(folder) = dialog.pick_folder() {
                app.output_base = Some(folder);
                app.notice = None;
            }

            Task::none()
        }
        Message::FolderNameChanged(name) => {
            app.folder_name = name;
            app.folder_error = None;
            Task::none()
        }
        Message::QualityChanged(quality) => {
            let quality = quality.clamp(1, 100);
            if app.quality != quality {
                app.quality = quality;
                save_quality(quality);
            }
            Task::none()
        }
        Message::ResetQuality => {
            if app.quality != JPEG_QUALITY {
                app.quality = JPEG_QUALITY;
                save_quality(JPEG_QUALITY);
            }
            Task::none()
        }
        Message::SetLanguage(language) => {
            if app.language != language {
                app.language = language;
                language.save();
            }
            Task::none()
        }
        Message::Start => app.start(),
        Message::FileProcessed(index, result) => {
            let failed = result.is_err();

            if let Some(file) = app.files.get_mut(index) {
                file.status = match result {
                    Ok(processed) => FileStatus::Done(processed),
                    Err(error) => FileStatus::Failed(error),
                };
            }

            app.completed += 1;
            if failed {
                app.failed += 1;
            }

            Task::none()
        }
        Message::Finished => {
            app.finish();
            Task::none()
        }
        Message::Tick => {
            if app.processing {
                app.spinner_frame = (app.spinner_frame + 1) % SPINNER_FRAMES.len();
            }
            Task::none()
        }
        Message::Clear => {
            if !app.processing {
                app.files.clear();
                app.completed = 0;
                app.failed = 0;
                app.batch_total = 0;
                app.elapsed = None;
                app.notice = None;
                app.folder_error = None;
            }

            Task::none()
        }
        Message::OpenOutput => {
            if let Some(directory) = &app.last_output_dir {
                reveal_in_file_manager(directory);
            }
            Task::none()
        }
        Message::DismissNotice => {
            app.notice = None;
            Task::none()
        }
    }
}

fn subscription(app: &App) -> Subscription<Message> {
    let events = iced::event::listen_with(|event, _status, _window| match event {
        iced::Event::Window(window::Event::FileDropped(path)) => {
            Some(Message::FilesDropped(vec![path]))
        }
        iced::Event::Window(window::Event::FileHovered(_)) => Some(Message::HoverChanged(true)),
        iced::Event::Window(window::Event::FilesHoveredLeft) => Some(Message::HoverChanged(false)),
        _ => None,
    });

    if app.processing {
        Subscription::batch([events, ticker()])
    } else {
        events
    }
}

fn ticker() -> Subscription<Message> {
    Subscription::run(|| {
        let (sender, receiver) = unbounded();

        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(100));

            if sender.unbounded_send(Message::Tick).is_err() {
                break;
            }
        });

        receiver
    })
}

fn reveal_in_file_manager(path: &Path) {
    #[cfg(target_os = "windows")]
    let _ = Command::new("explorer").arg(path).spawn();

    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(path).spawn();

    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = Command::new("xdg-open").arg(path).spawn();
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

fn view(app: &App) -> Element<'_, Message> {
    let mut content = column![header(app)]
        .spacing(16)
        .width(Length::Fill)
        .height(Length::Fill);

    if let Some(notice) = &app.notice {
        content = content.push(notice_view(app.language, notice));
    }

    content = content.push(drop_zone(app));

    if !app.files.is_empty() {
        content = content.push(file_list(app));
        content = content.push(progress_view(app));
    }

    content = content.push(controls(app));

    container(content)
        .padding(22)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(BACKGROUND.into()),
            ..container::Style::default()
        })
        .into()
}

fn icon(name: &str, size: f32, color: Color) -> Element<'static, Message> {
    fa_icon_solid::<Theme>(name).size(size).color(color).into()
}

fn icon_button(
    name: &'static str,
    hint: &'static str,
    message: Message,
) -> Element<'static, Message> {
    tooltip(
        button(icon(name, 14.0, TEXT))
            .on_press(message)
            .padding([7, 10])
            .style(secondary_button),
        container(text(hint).size(11).color(TEXT))
            .padding([5, 8])
            .style(tooltip_style),
        tooltip::Position::Top,
    )
    .gap(6)
    .into()
}

const SPINNER_FRAMES: [&str; 4] = ["◐", "◓", "◑", "◒"];

fn spinning(frame: usize, diameter: f32, color: Color) -> Element<'static, Message> {
    container(
        text(SPINNER_FRAMES[frame % SPINNER_FRAMES.len()])
            .size(diameter)
            .color(color),
    )
    .width(Length::Fixed(diameter))
    .height(Length::Fixed(diameter))
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .into()
}

fn status_icon(status: &FileStatus, spinner_frame: usize) -> Element<'_, Message> {
    match status {
        FileStatus::Pending => icon("clock", 14.0, MUTED),
        FileStatus::Working => spinning(spinner_frame, 18.0, WARNING),
        FileStatus::Done(_) => icon("circle-check", 14.0, SUCCESS),
        FileStatus::Failed(_) => icon("circle-xmark", 14.0, DANGER),
    }
}

fn flag_icon(handle: &'static LazyLock<ImageHandle>) -> Element<'static, Message> {
    iced::widget::image((**handle).clone())
        .width(Length::Fixed(24.0))
        .height(Length::Fixed(16.0))
        .into()
}

fn flag_button(
    language: Language,
    selected: bool,
    handle: &'static LazyLock<ImageHandle>,
    hint: &'static str,
) -> Element<'static, Message> {
    tooltip(
        button(flag_icon(handle))
            .on_press(Message::SetLanguage(language))
            .padding(4)
            .style(flag_button_style(selected)),
        container(text(hint).size(11).color(TEXT))
            .padding([5, 8])
            .style(tooltip_style),
        tooltip::Position::Bottom,
    )
    .gap(6)
    .into()
}

fn header(app: &App) -> Element<'_, Message> {
    let language = app.language;

    row![
        column![
            text(language.title()).size(30).color(TEXT),
            text(language.subtitle()).size(13).color(MUTED),
        ]
        .spacing(3),
        Space::new().width(Length::Fill),
        flag_button(
            Language::English,
            language == Language::English,
            &FLAG_EN_HANDLE,
            language.english(),
        ),
        flag_button(
            Language::Swedish,
            language == Language::Swedish,
            &FLAG_SV_HANDLE,
            language.swedish(),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Length::Fill)
    .into()
}

fn notice_view(language: Language, notice: &str) -> Element<'_, Message> {
    container(
        row![
            text(notice).size(13).color(TEXT).width(Length::Fill),
            icon_button("xmark", language.dismiss(), Message::DismissNotice),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill),
    )
    .padding([10, 14])
    .width(Length::Fill)
    .style(banner_style)
    .into()
}

fn drop_zone(app: &App) -> Element<'_, Message> {
    let language = app.language;

    let heading = if app.hovered {
        language.drop_hover()
    } else if app.files.is_empty() {
        language.drop_idle()
    } else {
        language.drop_more()
    };

    let subtitle = if app.files.is_empty() {
        language.drop_hint_empty()
    } else {
        language.drop_hint_more()
    };

    let content = column![
        icon("images", 34.0, if app.hovered { ACCENT } else { MUTED }),
        text(heading).size(20).color(TEXT),
        text(subtitle).size(13).color(MUTED),
    ]
    .spacing(5)
    .align_x(Alignment::Center);

    let zone = container(content)
        .width(Length::Fill)
        .height(if app.files.is_empty() {
            Length::Fill
        } else {
            Length::Fixed(146.0)
        })
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(drop_zone_style(app.hovered));

    mouse_area(zone).on_press(Message::Browse).into()
}

fn file_list(app: &App) -> Element<'_, Message> {
    let done = app
        .files
        .iter()
        .filter(|file| matches!(file.status, FileStatus::Done(_)))
        .count();
    let failed = app
        .files
        .iter()
        .filter(|file| matches!(file.status, FileStatus::Failed(_)))
        .count();

    let summary = if app.processing {
        app.language.finished_count(app.completed, app.batch_total)
    } else if failed > 0 {
        app.language.ready_failed(done, failed)
    } else if done > 0 {
        app.language.ready(done)
    } else {
        app.language.images_count(app.files.len())
    };

    let mut header = row![
        icon("images", 14.0, MUTED),
        text(app.language.batch()).size(15).color(TEXT),
        text(summary).size(12).color(MUTED),
        Space::new().width(Length::Fill),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    if !app.processing {
        header = header.push(icon_button(
            "trash",
            app.language.clear_batch(),
            Message::Clear,
        ));
    }

    let rows = app
        .files
        .iter()
        .map(|file| file_row(app.language, app.spinner_frame, file));
    let list = scrollable(column(rows).spacing(6).width(Length::Fill))
        .height(Length::Fill)
        .width(Length::Fill);

    container(column![header, list].spacing(10).width(Length::Fill))
        .padding(14)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(panel_style)
        .into()
}

fn file_row(language: Language, spinner_frame: usize, file: &PhotoFile) -> Element<'_, Message> {
    let name = file
        .path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.path.display().to_string());

    let (detail, detail_color) = match &file.status {
        FileStatus::Pending => (
            file.path
                .parent()
                .and_then(Path::file_name)
                .map(|parent| language.waiting_in(&parent.to_string_lossy()))
                .unwrap_or_else(|| language.waiting().to_owned()),
            MUTED,
        ),
        FileStatus::Working => (language.working().to_owned(), WARNING),
        FileStatus::Done(processed) => (
            format!(
                "{} × {} → {} × {} · {} → {}",
                processed.source_width,
                processed.source_height,
                processed.width,
                processed.height,
                format_bytes(processed.source_bytes),
                format_bytes(processed.output_bytes)
            ),
            SUCCESS,
        ),
        FileStatus::Failed(error) => (language.process_error(error), DANGER),
    };

    container(
        row![
            column![
                text(name).size(14).color(TEXT).width(Length::Fill),
                text(detail).size(12).color(detail_color),
            ]
            .spacing(2)
            .width(Length::Fill),
            status_icon(&file.status, spinner_frame),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill),
    )
    .padding([9, 12])
    .width(Length::Fill)
    .style(file_row_style)
    .into()
}

fn progress_view(app: &App) -> Element<'_, Message> {
    let mut content = column![].spacing(8).width(Length::Fill);

    if app.processing {
        let fraction = if app.batch_total == 0 {
            0.0
        } else {
            app.completed as f32 / app.batch_total as f32
        };

        content = content
            .push(
                progress_bar(0.0..=1.0, fraction)
                    .girth(6)
                    .style(progress_bar_style),
            )
            .push(
                row![
                    spinning(app.spinner_frame, 15.0, MUTED),
                    text(app.language.progress(app.completed, app.batch_total))
                        .size(12)
                        .color(MUTED),
                    Space::new().width(Length::Fill),
                    text(format!("{}%", (fraction * 100.0).round() as u32))
                        .size(12)
                        .color(MUTED),
                ]
                .spacing(6)
                .align_y(Alignment::Center)
                .width(Length::Fill),
            );
    } else if let Some(elapsed) = app.elapsed {
        let failed = app.failed;
        let summary_color = if failed > 0 { WARNING } else { SUCCESS };

        let mut summary = row![
            icon(
                if failed > 0 {
                    "triangle-exclamation"
                } else {
                    "circle-check"
                },
                13.0,
                summary_color
            ),
            text(
                app.language
                    .finished(app.batch_total, elapsed.as_secs_f32())
            )
            .size(12)
            .color(summary_color),
            Space::new().width(Length::Fill),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Fill);

        if failed > 0 {
            summary = summary.push(
                text(app.language.failed_count(failed))
                    .size(12)
                    .color(DANGER),
            );
            summary = summary.push(Space::new().width(10));
        }

        summary = summary.push(
            button(
                row![
                    icon("folder-open", 12.0, TEXT),
                    text(app.language.open_folder()).size(12),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .on_press(Message::OpenOutput)
            .padding([6, 12])
            .style(secondary_button),
        );

        content = content.push(summary);
    }

    content.into()
}

fn controls(app: &App) -> Element<'_, Message> {
    let language = app.language;

    let location = match (&app.output_base, validate_folder_name(&app.folder_name)) {
        (Some(base), Ok(folder)) => base.join(folder).display().to_string(),
        (Some(base), Err(_)) => base.display().to_string(),
        (None, _) => language.no_location().to_owned(),
    };

    let can_start = !app.processing && !app.files.is_empty();
    let start_label = if app.processing {
        language.resizing().to_owned()
    } else if app.files.is_empty() {
        language.resize_images().to_owned()
    } else {
        language.resize_n(app.files.len())
    };
    let start_icon_color = if can_start { Color::WHITE } else { MUTED };
    let start_glyph = if app.processing {
        spinning(app.spinner_frame, 16.0, start_icon_color)
    } else {
        icon("wand-magic-sparkles", 14.0, start_icon_color)
    };

    let folder_row = row![
        text_input(language.folder_placeholder(), &app.folder_name)
            .on_input(Message::FolderNameChanged)
            .on_submit(Message::Start)
            .padding([10, 12])
            .size(14)
            .style(text_input_style)
            .width(Length::Fill),
        button(
            row![start_glyph, text(start_label).size(14),]
                .spacing(7)
                .align_y(Alignment::Center),
        )
        .padding([11, 22])
        .style(primary_button)
        .on_press_maybe(can_start.then_some(Message::Start),),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    let quality_row = row![
        icon("sliders", 12.0, MUTED),
        text(language.quality()).size(11).color(MUTED),
        slider(1..=100, app.quality, Message::QualityChanged)
            .step(1)
            .width(Length::Fill)
            .style(quality_slider_style),
        text(app.quality.to_string())
            .size(12)
            .color(TEXT)
            .width(Length::Fixed(26.0)),
        icon_button(
            "arrow-rotate-left",
            language.reset_quality(),
            Message::ResetQuality,
        ),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    let mut form = column![
        row![
            column![
                row![
                    icon("folder", 11.0, MUTED),
                    text(language.new_folder()).size(11).color(MUTED),
                ]
                .spacing(5)
                .align_y(Alignment::Center),
                text(location).size(13).color(TEXT).width(Length::Fill),
            ]
            .spacing(2)
            .width(Length::Fill),
            icon_button(
                "folder-open",
                language.choose_folder(),
                Message::ChooseOutput,
            ),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill),
        quality_row,
        folder_row,
    ]
    .spacing(12)
    .width(Length::Fill);

    if let Some(error) = app.folder_error {
        form = form.push(text(language.folder_error(error)).size(12).color(DANGER));
    }

    container(form)
        .padding(14)
        .width(Length::Fill)
        .style(panel_style)
        .into()
}

// ---------------------------------------------------------------------------
// Styling
// ---------------------------------------------------------------------------

fn app_theme() -> Theme {
    Theme::custom(
        "Photo Resizer",
        theme::Palette {
            background: BACKGROUND,
            text: TEXT,
            primary: ACCENT,
            success: SUCCESS,
            warning: WARNING,
            danger: DANGER,
        },
    )
}

fn primary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Active => ACCENT,
        button::Status::Hovered => ACCENT_HOVER,
        button::Status::Pressed => ACCENT_PRESSED,
        button::Status::Disabled => SURFACE_ALT,
    };

    button::Style {
        background: Some(background.into()),
        text_color: if matches!(status, button::Status::Disabled) {
            MUTED
        } else {
            Color::WHITE
        },
        border: border::rounded(10),
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.25),
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        ..button::Style::default()
    }
}

fn secondary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => SURFACE_HOVER,
        _ => SURFACE_ALT,
    };

    button::Style {
        background: Some(background.into()),
        text_color: TEXT,
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 10.0.into(),
        },
        ..button::Style::default()
    }
}

fn text_input_style(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused { .. } => ACCENT,
        text_input::Status::Hovered => Color::from_rgb8(69, 88, 126),
        _ => BORDER,
    };

    text_input::Style {
        background: SURFACE_ALT.into(),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 10.0.into(),
        },
        icon: MUTED,
        placeholder: MUTED,
        value: TEXT,
        selection: ACCENT,
    }
}

fn progress_bar_style(_theme: &Theme) -> progress_bar::Style {
    progress_bar::Style {
        background: SURFACE_ALT.into(),
        bar: ACCENT.into(),
        border: border::rounded(3),
    }
}

fn quality_slider_style(_theme: &Theme, status: slider::Status) -> slider::Style {
    let handle_color = match status {
        slider::Status::Active => ACCENT,
        slider::Status::Hovered => ACCENT_HOVER,
        slider::Status::Dragged => ACCENT_PRESSED,
    };

    slider::Style {
        rail: slider::Rail {
            backgrounds: (ACCENT.into(), SURFACE_ALT.into()),
            width: 4.0,
            border: border::rounded(2),
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 6.0 },
            background: handle_color.into(),
            border_width: 2.0,
            border_color: BACKGROUND,
        },
    }
}

fn drop_zone_style(hovered: bool) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some((if hovered { SURFACE_ALT } else { SURFACE }).into()),
        border: Border {
            color: if hovered { ACCENT } else { BORDER },
            width: 2.0,
            radius: 16.0.into(),
        },
        ..container::Style::default()
    }
}

fn panel_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE.into()),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 14.0.into(),
        },
        ..container::Style::default()
    }
}

fn file_row_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE_ALT.into()),
        border: Border {
            color: Color::from_rgb8(38, 51, 76),
            width: 1.0,
            radius: 9.0.into(),
        },
        ..container::Style::default()
    }
}

fn banner_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(DANGER_DIM.into()),
        border: Border {
            color: Color::from_rgb8(122, 52, 72),
            width: 1.0,
            radius: 12.0.into(),
        },
        ..container::Style::default()
    }
}

fn tooltip_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE_HOVER.into()),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Style::default()
    }
}

fn flag_button_style(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = if selected {
            SURFACE_HOVER
        } else {
            match status {
                button::Status::Hovered | button::Status::Pressed => SURFACE_HOVER,
                _ => SURFACE,
            }
        };

        button::Style {
            background: Some(background.into()),
            text_color: TEXT,
            border: Border {
                color: if selected { ACCENT } else { BORDER },
                width: if selected { 2.0 } else { 1.0 },
                radius: 7.0.into(),
            },
            ..button::Style::default()
        }
    }
}
