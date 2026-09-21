//! Offline image normalization shared by desktop preview and OCR.
use image::{ImageDecoder, ImageEncoder};
use ledger_application::{AppError, MAX_RECEIPT_BYTES, ReceiptImage};
use std::{
    io::{Cursor, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub struct ReceiptImageNormalizer {
    directory: PathBuf,
}
impl ReceiptImageNormalizer {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    pub fn normalize(
        &self,
        image: &ReceiptImage,
        cancel: &AtomicBool,
    ) -> Result<ReceiptImage, AppError> {
        check_cancel(cancel)?;
        let converted;
        let bytes = if image.mime() == "image/heif" {
            converted = self.decode_heif(image, cancel)?;
            converted.as_slice()
        } else {
            image.bytes()
        };
        let mut reader = image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|_| invalid())?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(10000);
        limits.max_image_height = Some(10000);
        limits.max_alloc = Some(256 * 1024 * 1024);
        reader.limits(limits);
        let mut decoder = reader.into_decoder().map_err(|_| invalid())?;
        let (width, height) = decoder.dimensions();
        if u64::from(width) * u64::from(height) > 50_000_000 {
            return Err(invalid());
        }
        let orientation = decoder.orientation().map_err(|_| invalid())?;
        let mut decoded = image::DynamicImage::from_decoder(decoder).map_err(|_| invalid())?;
        decoded.apply_orientation(orientation);
        check_cancel(cancel)?;
        if decoded.width() > 4000 || decoded.height() > 4000 {
            decoded = decoded.thumbnail(4000, 4000);
        }
        // No EXIF/GPS survives re-encoding. Both the user and OCR see these pixels.
        let rgb = decoded.to_rgb8();
        let mut output = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, 95)
            .write_image(
                &rgb,
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|_| invalid())?;
        check_cancel(cancel)?;
        ReceiptImage::new(output)
    }

    fn decode_heif(&self, image: &ReceiptImage, cancel: &AtomicBool) -> Result<Vec<u8>, AppError> {
        let executable = std::path::absolute(self.directory.join("image-decoder/magick.exe"))
            .map_err(|_| invalid())?;
        if !executable.is_file() {
            return Err(AppError::Input(
                "ยังไม่มีตัวอ่าน HEIC/HEIF กรุณาติดตั้งชุด OCR ของแอป หรือใช้ JPG/PNG".into(),
            ));
        }
        let temp = tempfile::tempdir().map_err(|_| invalid())?;
        std::fs::write(temp.path().join("input.heic"), image.bytes()).map_err(|_| invalid())?;
        std::fs::write(
            temp.path().join("policy.xml"),
            include_str!("receipt-image-policy.xml"),
        )
        .map_err(|_| invalid())?;
        let deadline = Instant::now() + Duration::from_secs(45);
        run_decoder(
            &executable,
            temp.path(),
            &[
                "-ping",
                "HEIC:input.heic[0]",
                "-format",
                "%w %h",
                "info:dimensions.txt",
            ],
            cancel,
            deadline,
        )?;
        let dimensions = read_bounded(&temp.path().join("dimensions.txt"))?;
        let dimensions = std::str::from_utf8(&dimensions).map_err(|_| invalid())?;
        let values: Vec<u64> = dimensions
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<_, _>>()
            .map_err(|_| invalid())?;
        if values.len() != 2
            || values[0] == 0
            || values[1] == 0
            || values[0] > 10000
            || values[1] > 10000
            || values[0] * values[1] > 50_000_000
        {
            return Err(invalid());
        }
        run_decoder(
            &executable,
            temp.path(),
            &[
                "HEIC:input.heic[0]",
                "-auto-orient",
                "-thumbnail",
                "4000x4000>",
                "-strip",
                "-depth",
                "8",
                "PNG24:output.png",
            ],
            cancel,
            deadline,
        )?;
        read_bounded(&temp.path().join("output.png"))
    }
}

fn run_decoder(
    executable: &Path,
    directory: &Path,
    arguments: &[&str],
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<(), AppError> {
    check_cancel(cancel)?;
    let mut command = Command::new(executable);
    command
        .current_dir(directory)
        .env("MAGICK_CONFIGURE_PATH", directory)
        .env("MAGICK_TEMPORARY_PATH", directory)
        .env("MAGICK_THREAD_LIMIT", "2")
        .args([
            "-limit",
            "thread",
            "2",
            "-define",
            "heic:max-number-of-tiles=256",
            "-define",
            "heic:max-items=512",
        ])
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().map_err(|_| invalid())?;
    loop {
        if cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            check_cancel(cancel)?;
            return Err(AppError::Input("แปลงภาพนานเกินไป ลองครอปเฉพาะใบเสร็จ".into()));
        }
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) => return Err(invalid()),
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(invalid());
            }
        }
    }
    Ok(())
}
fn read_bounded(path: &Path) -> Result<Vec<u8>, AppError> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| invalid())?
        .take((MAX_RECEIPT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    if bytes.len() > MAX_RECEIPT_BYTES {
        return Err(invalid());
    }
    Ok(bytes)
}
fn invalid() -> AppError {
    AppError::Input("อ่านภาพไม่ได้ ใช้ JPG/PNG/HEIC ไม่เกิน 50 ล้านพิกเซลและด้านละ 10,000 พิกเซล".into())
}
fn check_cancel(cancel: &AtomicBool) -> Result<(), AppError> {
    if cancel.load(Ordering::Relaxed) {
        Err(AppError::Input("ยกเลิกการอ่านใบเสร็จแล้ว".into()))
    } else {
        Ok(())
    }
}
