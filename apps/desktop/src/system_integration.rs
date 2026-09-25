use ledger_application::AppError;
use std::path::Path;

pub fn install(path: &Path) -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        let root = std::env::var_os("SystemRoot").ok_or(AppError::WorkerStopped)?;
        std::process::Command::new(Path::new(&root).join("System32/msiexec.exe"))
            .arg("/i")
            .arg(path)
            .arg("/norestart")
            .spawn()
            .map_err(|_| AppError::Input("เปิดตัวติดตั้งไม่ได้ กรุณาลองอีกครั้ง".into()))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Err(AppError::WorkerStopped)
    }
}

#[cfg(windows)]
const APP_ID: &str = "com.dancingwithmycode.ledgersbro";

pub fn enable_notifications() -> Result<bool, AppError> {
    #[cfg(windows)]
    {
        let root = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        let (key, _) = root
            .create_subkey(format!("Software\\Classes\\AppUserModelId\\{APP_ID}"))
            .map_err(|_| notification_error())?;
        key.set_value("DisplayName", &"Ledgers Bro")
            .map_err(|_| notification_error())?;
        Ok(true)
    }
    #[cfg(not(windows))]
    {
        Ok(false)
    }
}

pub fn notify(title: &str, body: &str) -> Result<(), AppError> {
    #[cfg(windows)]
    {
        use windows::{
            Data::Xml::Dom::XmlDocument,
            UI::Notifications::{ToastNotification, ToastNotificationManager},
            core::HSTRING,
        };
        let document = XmlDocument::new().map_err(|_| notification_error())?;
        let xml = format!(
            "<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>",
            escape(title),
            escape(body)
        );
        document
            .LoadXml(&HSTRING::from(xml))
            .map_err(|_| notification_error())?;
        let toast = ToastNotification::CreateToastNotification(&document)
            .map_err(|_| notification_error())?;
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_ID))
            .and_then(|notifier| notifier.Show(&toast))
            .map_err(|_| notification_error())
    }
    #[cfg(not(windows))]
    {
        let _ = (title, body);
        Ok(())
    }
}
#[cfg(windows)]
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
#[cfg(windows)]
fn notification_error() -> AppError {
    AppError::Input("แสดงการแจ้งเตือนระบบไม่ได้ ตรวจการอนุญาตในตั้งค่า Windows".into())
}
