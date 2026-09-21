//! Local-only OCR adapter. No shell interpolation, URLs, or ledger access.
use image::ImageDecoder;
use ledger_application::{AppError, MAX_OCR_TEXT_BYTES, ReceiptImage, ReceiptOcr};
use std::{
    io::{Cursor, Read},
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct TesseractOcr {
    directory: PathBuf,
}
impl TesseractOcr {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }
    pub fn scan(
        &self,
        image: &ReceiptImage,
        cancel: &AtomicBool,
    ) -> Result<(ReceiptImage, String), AppError> {
        let normalized =
            crate::ReceiptImageNormalizer::new(self.directory.clone()).normalize(image, cancel)?;
        let text = self.recognize(&normalized, cancel)?;
        Ok((normalized, text))
    }
}
impl ReceiptOcr for TesseractOcr {
    fn recognize(&self, image: &ReceiptImage, cancel: &AtomicBool) -> Result<String, AppError> {
        if cancel.load(Ordering::Relaxed) {
            return Err(cancelled());
        }
        // Keep a normal absolute Windows path. Some native OCR libraries do not
        // accept the verbatim \\?\ prefix produced by canonicalize().
        let directory = std::path::absolute(&self.directory).map_err(|_| {
            AppError::Input("ยังไม่พบตัวอ่านใบเสร็จในเครื่อง กรุณาติดตั้งชุด OCR ของแอป".into())
        })?;
        let executable = directory.join(if cfg!(windows) {
            "tesseract.exe"
        } else {
            "tesseract"
        });
        let models = directory.join("tessdata");
        if !executable.is_file()
            || !models.join("tha.traineddata").is_file()
            || !models.join("eng.traineddata").is_file()
        {
            return Err(AppError::Input(
                "ยังไม่พบตัวอ่านใบเสร็จในเครื่อง กรุณาติดตั้งชุด OCR ของแอป".into(),
            ));
        }
        // Decode and re-encode before invoking the native reader. This rejects
        // malformed inputs and bounds decompression; file extensions are untrusted.
        let mut reader = image::ImageReader::new(Cursor::new(image.bytes()))
            .with_guessed_format()
            .map_err(|_| invalid_image())?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(6000);
        limits.max_image_height = Some(6000);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        let mut decoder = reader.into_decoder().map_err(|_| invalid_image())?;
        let orientation = decoder.orientation().map_err(|_| invalid_image())?;
        let mut decoded =
            image::DynamicImage::from_decoder(decoder).map_err(|_| invalid_image())?;
        decoded.apply_orientation(orientation);
        if u64::from(decoded.width()) * u64::from(decoded.height()) > 20_000_000 {
            return Err(invalid_image());
        }
        let temp = tempfile::tempdir().map_err(|_| unavailable())?;
        let input = temp.path().join("receipt.png");
        decoded
            .save_with_format(&input, image::ImageFormat::Png)
            .map_err(|_| unavailable())?;
        let output = temp.path().join("read");
        let mut command = Command::new(&executable);
        command
            .current_dir(&directory)
            .arg(&input)
            .arg(&output)
            .arg("--tessdata-dir")
            .arg(&models)
            .args(["-l", "tha+eng", "--oem", "1", "--psm", "6"])
            .env("OMP_THREAD_LIMIT", "2")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let mut child = command.spawn().map_err(|_| unavailable())?;
        let deadline = Instant::now() + Duration::from_secs(45);
        loop {
            if cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(if cancel.load(Ordering::Relaxed) {
                    cancelled()
                } else {
                    AppError::Input("อ่านภาพนานเกินไป ลองใช้ภาพที่ครอปเฉพาะใบเสร็จ".into())
                });
            }
            match child.try_wait() {
                Ok(Some(status)) if status.success() => break,
                Ok(Some(_)) => return Err(invalid_image()),
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(unavailable());
                }
            }
        }
        let file = std::fs::File::open(output.with_extension("txt")).map_err(|_| unavailable())?;
        let mut text = String::new();
        file.take((MAX_OCR_TEXT_BYTES + 1) as u64)
            .read_to_string(&mut text)
            .map_err(|_| invalid_image())?;
        if text.len() > MAX_OCR_TEXT_BYTES {
            return Err(invalid_image());
        }
        if text.trim().is_empty() {
            return Err(AppError::Input(
                "ยังอ่านข้อความไม่เจอ ลองถ่ายให้ตรงและมีแสงเพียงพอ".into(),
            ));
        }
        // TempDir removes the working image and OCR output when this scope ends.
        Ok(text)
    }
}
fn invalid_image() -> AppError {
    AppError::Input("อ่านภาพนี้ไม่ได้ ใช้ JPG/PNG ไม่เกิน 20 ล้านพิกเซลและด้านละ 6,000 พิกเซล".into())
}
fn unavailable() -> AppError {
    AppError::Input("ตัวอ่านใบเสร็จทำงานไม่สำเร็จ กรุณาลองใหม่".into())
}
fn cancelled() -> AppError {
    AppError::Input("ยกเลิกการอ่านใบเสร็จแล้ว".into())
}
