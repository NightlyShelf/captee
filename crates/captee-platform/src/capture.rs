//! Desktop-aware capture selection and bounded `grim`/`slurp` fallback.

use captee_core::{
    AnnotatedImage, Annotation, AnnotationBackend, AnnotationError, AnnotationResult,
    CaptureBackend, CaptureError, CaptureResult, CaptureSettings, CapturedImage,
};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const ANNOTATION_COLOR: [u8; 4] = [220, 38, 38, 255];
const TEXT_SCALE: u32 = 2;
const GLYPH_WIDTH: u32 = 5;
const GLYPH_HEIGHT: u32 = 7;
const MAX_ANNOTATION_PIXELS: usize = 16 * 1024 * 1024;
const MAX_CAPTURE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CAPTURE_ERROR_BYTES: u64 = 1024 * 1024;

/// Applies lightweight annotations to a PNG without changing the captured
/// image. Every operation decodes into a new pixel buffer and returns a new
/// PNG that remains staged until the caller confirms it.
#[derive(Debug, Clone, Copy, Default)]
pub struct PngAnnotationBackend;

impl PngAnnotationBackend {
    pub fn new() -> Self {
        Self
    }
}

impl AnnotationBackend for PngAnnotationBackend {
    fn annotate(
        &self,
        image: &CapturedImage,
        annotation: &Annotation,
    ) -> AnnotationResult<AnnotatedImage> {
        let mut bitmap = match RgbaBitmap::decode(image.bytes()) {
            Ok(bitmap) => bitmap,
            Err(error) => return AnnotationResult::Failed(error),
        };

        match annotation {
            Annotation::Pointer { x, y } => bitmap.draw_pointer(*x, *y),
            Annotation::Rectangle { x, y, width, height } => {
                bitmap.draw_rectangle(*x, *y, *width, *height)
            }
            Annotation::Text { x, y, text } => bitmap.draw_text(*x, *y, text),
        }

        match bitmap.encode() {
            Ok(bytes) => AnnotationResult::Completed(AnnotatedImage::new(bytes)),
            Err(error) => AnnotationResult::Failed(error),
        }
    }
}

#[derive(Debug)]
struct RgbaBitmap {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl RgbaBitmap {
    fn decode(bytes: &[u8]) -> Result<Self, AnnotationError> {
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder
            .read_info()
            .map_err(|error| AnnotationError::new(format!("could not decode PNG: {error}")))?;
        let output_size = reader.output_buffer_size();
        let mut output = vec![0; output_size];
        let info = reader
            .next_frame(&mut output)
            .map_err(|error| AnnotationError::new(format!("could not read PNG frame: {error}")))?;
        let pixels = match info.color_type {
            png::ColorType::Rgba => output[..info.buffer_size()].to_vec(),
            png::ColorType::Rgb => output[..info.buffer_size()]
                .chunks_exact(3)
                .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
                .collect(),
            png::ColorType::Grayscale => output[..info.buffer_size()]
                .iter()
                .flat_map(|value| [*value, *value, *value, 255])
                .collect(),
            png::ColorType::GrayscaleAlpha => output[..info.buffer_size()]
                .chunks_exact(2)
                .flat_map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]])
                .collect(),
            png::ColorType::Indexed => {
                return Err(AnnotationError::new("indexed PNG output is unsupported"))
            }
        };
        let expected = pixel_len(info.width, info.height)?;
        if pixels.len() != expected {
            return Err(AnnotationError::new("PNG pixel buffer has an invalid size"));
        }
        Ok(Self { width: info.width, height: info.height, pixels })
    }

    fn encode(self) -> Result<Vec<u8>, AnnotationError> {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|error| AnnotationError::new(format!("could not encode PNG: {error}")))?;
        writer
            .write_image_data(&self.pixels)
            .map_err(|error| AnnotationError::new(format!("could not encode PNG: {error}")))?;
        drop(writer);
        Ok(bytes)
    }

    fn draw_pointer(&mut self, x: u32, y: u32) {
        let radius: i32 = 8;
        for offset in -radius..=radius {
            self.paint(x as i32 + offset, y as i32, ANNOTATION_COLOR);
            self.paint(x as i32, y as i32 + offset, ANNOTATION_COLOR);
        }
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= radius * radius && (dx.abs() + dy.abs()) % 2 == 0 {
                    self.paint(x as i32 + dx, y as i32 + dy, ANNOTATION_COLOR);
                }
            }
        }
    }

    fn draw_rectangle(&mut self, x: u32, y: u32, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        let right = x.saturating_add(width.saturating_sub(1));
        let bottom = y.saturating_add(height.saturating_sub(1));
        for offset in 0i32..3 {
            for point in x as i32..=right as i32 {
                self.paint(point, y as i32 + offset, ANNOTATION_COLOR);
                self.paint(point, bottom as i32 - offset, ANNOTATION_COLOR);
            }
            for point in y as i32..=bottom as i32 {
                self.paint(x as i32 + offset, point, ANNOTATION_COLOR);
                self.paint(right as i32 - offset, point, ANNOTATION_COLOR);
            }
        }
    }

    fn draw_text(&mut self, x: u32, y: u32, text: &str) {
        let mut cursor = x;
        let mut line_y = y;
        for character in text.chars() {
            if character == '\n' {
                cursor = x;
                line_y = line_y.saturating_add((GLYPH_HEIGHT + 1) * TEXT_SCALE);
                continue;
            }
            let glyph = glyph(character);
            for (row, bits) in glyph.iter().enumerate() {
                for column in 0..GLYPH_WIDTH {
                    if bits & (1 << (GLYPH_WIDTH - 1 - column)) == 0 {
                        continue;
                    }
                    for dy in 0..TEXT_SCALE {
                        for dx in 0..TEXT_SCALE {
                            self.paint(
                                cursor.saturating_add(column * TEXT_SCALE + dx) as i32,
                                line_y.saturating_add(row as u32 * TEXT_SCALE + dy) as i32,
                                ANNOTATION_COLOR,
                            );
                        }
                    }
                }
            }
            cursor = cursor.saturating_add((GLYPH_WIDTH + 1) * TEXT_SCALE);
        }
    }

    fn paint(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return;
        }
        let index = ((y as u32 * self.width + x as u32) * 4) as usize;
        let alpha = color[3] as u16;
        let inverse = 255 - alpha;
        for (channel, value) in color.iter().take(3).enumerate() {
            self.pixels[index + channel] = ((*value as u16 * alpha
                + self.pixels[index + channel] as u16 * inverse)
                / 255) as u8;
        }
        self.pixels[index + 3] = (alpha + self.pixels[index + 3] as u16 * inverse / 255) as u8;
    }
}

fn pixel_len(width: u32, height: u32) -> Result<usize, AnnotationError> {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .filter(|_| (width as usize).saturating_mul(height as usize) <= MAX_ANNOTATION_PIXELS)
        .ok_or_else(|| AnnotationError::new("PNG dimensions are too large"))
}

fn glyph(character: char) -> [u8; 7] {
    match character.to_ascii_uppercase() {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110],
        '6' => [0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
        '?' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        ' ' => [0; 7],
        _ => [0b11111, 0b10001, 0b10101, 0b10001, 0b10101, 0b10001, 0b11111],
    }
}

/// Linux screenshot portal adapter. The portal request is interactive, so the
/// desktop can offer a screen, window, or region picker appropriate for the
/// active compositor.
#[derive(Debug, Clone, Copy, Default)]
pub struct XdgPortalCapture;

impl XdgPortalCapture {
    pub fn new() -> Self {
        Self
    }
}

impl CaptureBackend for XdgPortalCapture {
    fn capture(&self) -> CaptureResult<CapturedImage> {
        let screenshot = async_io::block_on(async {
            let request = ashpd::desktop::screenshot::Screenshot::request()
                .interactive(true)
                .modal(true)
                .send()
                .await?;
            request.response().map(|response| response.uri().as_str().to_owned())
        });

        match screenshot {
            Ok(uri) => load_portal_capture(&uri),
            Err(error) if portal_cancelled(&error) => CaptureResult::Cancelled,
            Err(error) => CaptureResult::Failed(CaptureError::new(format!(
                "screenshot portal failed: {error}"
            ))),
        }
    }
}

fn portal_cancelled(error: &ashpd::Error) -> bool {
    matches!(
        error,
        ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled)
            | ashpd::Error::Portal(ashpd::PortalError::Cancelled(_))
    )
}

fn load_portal_capture(uri: &str) -> CaptureResult<CapturedImage> {
    let url = match url::Url::parse(uri) {
        Ok(url) => url,
        Err(error) => {
            return CaptureResult::Failed(CaptureError::new(format!(
                "screenshot portal returned an invalid URI: {error}"
            )))
        }
    };
    let path = match url.to_file_path() {
        Ok(path) => path,
        Err(()) => {
            return CaptureResult::Failed(CaptureError::new(
                "screenshot portal returned a non-local file URI",
            ))
        }
    };
    let file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(error) => {
            return CaptureResult::Failed(CaptureError::new(format!(
                "could not open portal screenshot {}: {error}",
                path.display()
            )))
        }
    };
    let mut bytes = Vec::new();
    if let Err(error) = file.take(MAX_CAPTURE_BYTES + 1).read_to_end(&mut bytes) {
        return CaptureResult::Failed(CaptureError::new(format!(
            "could not read portal screenshot {}: {error}",
            path.display()
        )));
    }
    if bytes.len() as u64 > MAX_CAPTURE_BYTES {
        return CaptureResult::Failed(CaptureError::new(format!(
            "portal screenshot exceeds the {} MiB capture limit",
            MAX_CAPTURE_BYTES / (1024 * 1024)
        )));
    }
    if bytes.is_empty() {
        return CaptureResult::Failed(CaptureError::new(
            "screenshot portal returned an empty file",
        ));
    }
    CaptureResult::Completed(CapturedImage::new(bytes))
}

/// Selects enabled capture backends in desktop-appropriate order. Cancellation
/// is returned immediately and never starts the other backend.
#[derive(Debug, Clone)]
pub struct CaptureSelector<P, F> {
    portal: P,
    fallback: F,
    settings: CaptureSettings,
    fallback_first: bool,
}

impl<P, F> CaptureSelector<P, F> {
    pub fn new(portal: P, fallback: F, settings: CaptureSettings) -> Self {
        Self { portal, fallback, settings, fallback_first: false }
    }

    pub fn with_fallback_first(mut self, fallback_first: bool) -> Self {
        self.fallback_first = fallback_first;
        self
    }
}

impl<P: CaptureBackend, F: CaptureBackend> CaptureBackend for CaptureSelector<P, F> {
    fn capture(&self) -> CaptureResult<CapturedImage> {
        if self.fallback_first && self.settings.fallback_enabled {
            match self.fallback.capture() {
                CaptureResult::Completed(image) => return CaptureResult::Completed(image),
                CaptureResult::Cancelled => return CaptureResult::Cancelled,
                CaptureResult::Failed(error) if !self.settings.portal_enabled => {
                    return CaptureResult::Failed(error)
                }
                CaptureResult::Failed(_) => {}
            }
        }

        if self.settings.portal_enabled {
            match self.portal.capture() {
                CaptureResult::Completed(image) => return CaptureResult::Completed(image),
                CaptureResult::Cancelled => return CaptureResult::Cancelled,
                CaptureResult::Failed(error)
                    if !self.settings.fallback_enabled || self.fallback_first =>
                {
                    return CaptureResult::Failed(error)
                }
                CaptureResult::Failed(_) => {}
            }
        }

        if self.settings.fallback_enabled && !self.fallback_first {
            return self.fallback.capture();
        }

        CaptureResult::Failed(CaptureError::new("no capture backend is enabled"))
    }
}

/// Hyprland's interactive screenshot portal may not present a region picker,
/// while `slurp`/`grim` is its native bounded selection path.
pub fn current_desktop_prefers_fallback_capture() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .is_ok_and(|desktop| desktop_prefers_fallback_capture(&desktop))
}

fn desktop_prefers_fallback_capture(desktop: &str) -> bool {
    desktop.split(':').any(|name| name.eq_ignore_ascii_case("hyprland"))
}

/// Bounded Linux fallback using `slurp` for region selection and `grim` for
/// PNG capture. The executable paths are configurable for packaging and tests.
#[derive(Debug, Clone)]
pub struct GrimSlurpCapture {
    slurp: PathBuf,
    grim: PathBuf,
    timeout: Duration,
}

impl GrimSlurpCapture {
    pub fn new(timeout: Duration) -> Self {
        Self::with_paths("slurp", "grim", timeout)
    }

    pub fn with_paths(
        slurp: impl Into<PathBuf>,
        grim: impl Into<PathBuf>,
        timeout: Duration,
    ) -> Self {
        Self { slurp: slurp.into(), grim: grim.into(), timeout }
    }
}

impl CaptureBackend for GrimSlurpCapture {
    fn capture(&self) -> CaptureResult<CapturedImage> {
        let selection = match run_bounded(&self.slurp, &[], self.timeout) {
            Ok(output) => output,
            Err(error) => return CaptureResult::Failed(error),
        };
        if !selection.status.success() {
            if selection.status.code() == Some(1) {
                return CaptureResult::Cancelled;
            }
            return CaptureResult::Failed(command_failure("slurp", &selection));
        }

        let geometry = String::from_utf8_lossy(&selection.stdout).trim().to_owned();
        if geometry.is_empty() {
            return CaptureResult::Cancelled;
        }

        let grim_args = vec!["-g".to_owned(), geometry, "-".to_owned()];
        let image = match run_bounded(&self.grim, &grim_args, self.timeout) {
            Ok(output) => output,
            Err(error) => return CaptureResult::Failed(error),
        };
        if !image.status.success() {
            return CaptureResult::Failed(command_failure("grim", &image));
        }
        CaptureResult::Completed(CapturedImage::new(image.stdout))
    }
}

fn run_bounded(program: &Path, args: &[String], timeout: Duration) -> Result<Output, CaptureError> {
    let mut command = Command::new(program);
    command.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| {
        CaptureError::new(format!("could not start {}: {error}", program.display()))
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CaptureError::new("capture command stdout was not available"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| CaptureError::new("capture command stderr was not available"))?;
    let stdout_reader = spawn_pipe_reader(stdout, MAX_CAPTURE_BYTES, "stdout")?;
    let stderr_reader = spawn_pipe_reader(stderr, MAX_CAPTURE_ERROR_BYTES, "stderr")?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = join_pipe_reader(stdout_reader, "stdout")?;
                let stderr = join_pipe_reader(stderr_reader, "stderr")?;
                return Ok(Output { status, stdout, stderr });
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(CaptureError::new(format!(
                    "capture command timed out after {} ms",
                    timeout.as_millis()
                )));
            }
            Ok(None) => thread::sleep(Duration::from_millis(5)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(CaptureError::new(format!(
                    "could not monitor capture command: {error}"
                )));
            }
        }
    }
}

fn spawn_pipe_reader(
    reader: impl Read + Send + 'static,
    limit: u64,
    stream: &'static str,
) -> Result<thread::JoinHandle<std::io::Result<Vec<u8>>>, CaptureError> {
    thread::Builder::new()
        .name(format!("captee-capture-{stream}"))
        .spawn(move || {
            let mut bytes = Vec::new();
            reader.take(limit + 1).read_to_end(&mut bytes)?;
            if bytes.len() as u64 > limit {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("capture command {stream} exceeded {limit} bytes"),
                ));
            }
            Ok(bytes)
        })
        .map_err(|error| CaptureError::new(format!("could not monitor capture {stream}: {error}")))
}

fn join_pipe_reader(
    reader: thread::JoinHandle<std::io::Result<Vec<u8>>>,
    stream: &str,
) -> Result<Vec<u8>, CaptureError> {
    reader
        .join()
        .map_err(|_| CaptureError::new(format!("capture {stream} reader panicked")))?
        .map_err(|error| CaptureError::new(format!("could not read capture {stream}: {error}")))
}

fn command_failure(command: &str, output: &Output) -> CaptureError {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if stderr.is_empty() {
        CaptureError::new(format!("{command} exited with status {}", output.status))
    } else {
        CaptureError::new(format!("{command} failed: {stderr}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct FakeBackend {
        result: CaptureResult<CapturedImage>,
        calls: Arc<Mutex<u32>>,
    }

    impl CaptureBackend for FakeBackend {
        fn capture(&self) -> CaptureResult<CapturedImage> {
            *self.calls.lock().expect("calls lock") += 1;
            self.result.clone()
        }
    }

    fn backend(result: CaptureResult<CapturedImage>) -> (FakeBackend, Arc<Mutex<u32>>) {
        let calls = Arc::new(Mutex::new(0));
        (FakeBackend { result, calls: calls.clone() }, calls)
    }

    fn test_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
        let root = std::env::temp_dir().join(format!("captee-capture-{name}-{suffix}"));
        fs::create_dir_all(&root).expect("temporary root");
        root
    }

    #[test]
    fn portal_success_is_preferred_over_fallback() {
        let (portal, portal_calls) =
            backend(CaptureResult::Completed(CapturedImage::new(b"portal")));
        let (fallback, fallback_calls) =
            backend(CaptureResult::Completed(CapturedImage::new(b"fallback")));
        let selector = CaptureSelector::new(portal, fallback, CaptureSettings::default());

        assert_eq!(selector.capture(), CaptureResult::Completed(CapturedImage::new(b"portal")));
        assert_eq!(*portal_calls.lock().expect("portal calls"), 1);
        assert_eq!(*fallback_calls.lock().expect("fallback calls"), 0);
    }

    #[test]
    fn portal_failure_uses_enabled_fallback_but_cancellation_does_not() {
        let (portal, _) = backend(CaptureResult::Failed(CaptureError::new("portal unavailable")));
        let (fallback, fallback_calls) =
            backend(CaptureResult::Completed(CapturedImage::new(b"fallback")));
        let selector = CaptureSelector::new(portal, fallback, CaptureSettings::default());
        assert_eq!(selector.capture(), CaptureResult::Completed(CapturedImage::new(b"fallback")));
        assert_eq!(*fallback_calls.lock().expect("fallback calls"), 1);

        let (portal, _) = backend(CaptureResult::Cancelled);
        let (fallback, fallback_calls) =
            backend(CaptureResult::Completed(CapturedImage::new(b"fallback")));
        let selector = CaptureSelector::new(portal, fallback, CaptureSettings::default());
        assert_eq!(selector.capture(), CaptureResult::Cancelled);
        assert_eq!(*fallback_calls.lock().expect("fallback calls"), 0);
    }

    #[test]
    fn hyprland_order_prefers_fallback_and_preserves_cancellation() {
        let (portal, portal_calls) =
            backend(CaptureResult::Completed(CapturedImage::new(b"portal")));
        let (fallback, fallback_calls) =
            backend(CaptureResult::Completed(CapturedImage::new(b"fallback")));
        let selector = CaptureSelector::new(portal, fallback, CaptureSettings::default())
            .with_fallback_first(true);

        assert_eq!(selector.capture(), CaptureResult::Completed(CapturedImage::new(b"fallback")));
        assert_eq!(*portal_calls.lock().expect("portal calls"), 0);
        assert_eq!(*fallback_calls.lock().expect("fallback calls"), 1);

        let (portal, portal_calls) =
            backend(CaptureResult::Completed(CapturedImage::new(b"portal")));
        let (fallback, _) = backend(CaptureResult::Cancelled);
        let selector = CaptureSelector::new(portal, fallback, CaptureSettings::default())
            .with_fallback_first(true);
        assert_eq!(selector.capture(), CaptureResult::Cancelled);
        assert_eq!(*portal_calls.lock().expect("portal calls"), 0);
    }

    #[test]
    fn desktop_detection_handles_hyprland_desktop_lists() {
        assert!(desktop_prefers_fallback_capture("Hyprland"));
        assert!(desktop_prefers_fallback_capture("wlroots:Hyprland"));
        assert!(desktop_prefers_fallback_capture("hyprland"));
        assert!(!desktop_prefers_fallback_capture("GNOME"));
    }

    #[test]
    fn disabled_backends_return_an_explicit_failure() {
        let (portal, portal_calls) =
            backend(CaptureResult::Completed(CapturedImage::new(b"portal")));
        let (fallback, fallback_calls) =
            backend(CaptureResult::Completed(CapturedImage::new(b"fallback")));
        let selector = CaptureSelector::new(
            portal,
            fallback,
            CaptureSettings { portal_enabled: false, fallback_enabled: false },
        );

        assert!(matches!(selector.capture(), CaptureResult::Failed(_)));
        assert_eq!(*portal_calls.lock().expect("portal calls"), 0);
        assert_eq!(*fallback_calls.lock().expect("fallback calls"), 0);
    }

    #[test]
    fn portal_file_uri_is_decoded_and_loaded() {
        let root = test_root("portal uri");
        let path = root.join("capture image.png");
        fs::write(&path, b"PNG fixture").expect("portal fixture");
        let uri = url::Url::from_file_path(&path).expect("file URI");

        assert_eq!(
            load_portal_capture(uri.as_str()),
            CaptureResult::Completed(CapturedImage::new(b"PNG fixture"))
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn portal_loader_rejects_remote_empty_and_oversized_results() {
        assert!(matches!(
            load_portal_capture("https://example.invalid/capture.png"),
            CaptureResult::Failed(_)
        ));

        let root = test_root("portal-invalid");
        let empty = root.join("empty.png");
        fs::write(&empty, []).expect("empty fixture");
        let empty_uri = url::Url::from_file_path(&empty).expect("empty URI");
        assert!(matches!(load_portal_capture(empty_uri.as_str()), CaptureResult::Failed(_)));

        let oversized = root.join("oversized.png");
        let file = fs::File::create(&oversized).expect("oversized fixture");
        file.set_len(MAX_CAPTURE_BYTES + 1).expect("oversized length");
        let oversized_uri = url::Url::from_file_path(&oversized).expect("oversized URI");
        assert!(matches!(load_portal_capture(oversized_uri.as_str()), CaptureResult::Failed(_)));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn portal_response_cancellation_is_recognized() {
        let error = ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled);
        assert!(portal_cancelled(&error));
    }

    #[cfg(unix)]
    #[test]
    fn grim_slurp_adapter_runs_selection_then_capture_with_a_bound() {
        use std::os::unix::fs::PermissionsExt;

        let root = test_root("commands");
        let slurp = root.join("slurp");
        let grim = root.join("grim");
        fs::write(&slurp, "#!/bin/sh\nprintf '0,0 10x10'\n").expect("slurp script");
        fs::write(&grim, "#!/bin/sh\nprintf 'PNG fixture'\n").expect("grim script");
        for path in [&slurp, &grim] {
            let mut permissions = fs::metadata(path).expect("script metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).expect("script permissions");
        }

        let capture = GrimSlurpCapture::with_paths(slurp, grim, Duration::from_secs(1));
        assert_eq!(capture.capture(), CaptureResult::Completed(CapturedImage::new(b"PNG fixture")));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn grim_output_larger_than_a_pipe_buffer_is_drained_while_running() {
        use std::os::unix::fs::PermissionsExt;

        let root = test_root("large-output");
        let slurp = root.join("slurp");
        let grim = root.join("grim");
        fs::write(&slurp, "#!/bin/sh\nprintf '0,0 10x10'\n").expect("slurp script");
        fs::write(&grim, "#!/bin/sh\ndd if=/dev/zero bs=1024 count=256 2>/dev/null\n")
            .expect("grim script");
        for path in [&slurp, &grim] {
            let mut permissions = fs::metadata(path).expect("script metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).expect("script permissions");
        }

        let capture = GrimSlurpCapture::with_paths(slurp, grim, Duration::from_secs(2));
        let CaptureResult::Completed(image) = capture.capture() else {
            panic!("large piped capture should complete");
        };
        assert_eq!(image.bytes().len(), 256 * 1024);
        fs::remove_dir_all(root).expect("cleanup");
    }

    fn fixture_png(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
        let pixels = color.repeat((width * height) as usize);
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PNG header");
        writer.write_image_data(&pixels).expect("PNG pixels");
        drop(writer);
        bytes
    }

    fn decoded_pixels(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().expect("PNG info");
        let mut output = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut output).expect("PNG frame");
        (info.width, info.height, output[..info.buffer_size()].to_vec())
    }

    #[test]
    fn annotations_return_new_pngs_and_preserve_the_capture() {
        let original_bytes = fixture_png(32, 24, [240, 240, 240, 255]);
        let captured = CapturedImage::new(original_bytes.clone());
        let annotator = PngAnnotationBackend::new();

        for annotation in [
            Annotation::Pointer { x: 5, y: 6 },
            Annotation::Rectangle { x: 8, y: 7, width: 12, height: 9 },
            Annotation::Text { x: 2, y: 2, text: "A1".to_owned() },
        ] {
            let result = annotator.annotate(&captured, &annotation);
            let AnnotationResult::Completed(annotated) = result else {
                panic!("annotation should succeed");
            };
            let (width, height, pixels) = decoded_pixels(annotated.bytes());
            assert_eq!((width, height), (32, 24));
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[0] > 200 && pixel[1] < 100));
            assert_eq!(captured.bytes(), original_bytes.as_slice());
        }
    }

    #[test]
    fn annotations_clip_coordinates_and_reject_malformed_input() {
        let captured = CapturedImage::new(fixture_png(4, 4, [240, 240, 240, 255]));
        let annotator = PngAnnotationBackend::new();
        assert!(matches!(
            annotator.annotate(&captured, &Annotation::Pointer { x: u32::MAX, y: u32::MAX }),
            AnnotationResult::Completed(_)
        ));
        assert!(matches!(
            annotator
                .annotate(&CapturedImage::new(b"not a png"), &Annotation::Pointer { x: 0, y: 0 }),
            AnnotationResult::Failed(_)
        ));
    }
}
