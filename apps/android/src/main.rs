#[cfg(target_os = "android")]
mod platform;

#[cfg(target_os = "android")]
mod app {
    use super::platform;
    use dioxus::prelude::*;
    use ledger_ui::{ArtAssets, Gateway, HostInfo};
    use std::sync::Arc;

    pub fn launch() {
        let host = HostInfo {
            art: Arc::new(ArtAssets::bundled()),
            isolated: true,
            receipt_ocr_available: true,
            preview_label: "สมุดบัญชีแยกสำหรับทดลอง • Android",
        };
        LaunchBuilder::mobile()
            .with_cfg(dioxus::mobile::Config::new().with_background_color((255, 227, 165, 255)))
            .with_context(host)
            .launch(AndroidRoot);
    }

    #[component]
    fn AndroidRoot() -> Element {
        let mut bootstrap = use_resource(platform::initialize);
        match bootstrap.read().as_ref() {
            Some(Ok(gateway)) => rsx! { Ready { gateway: gateway.clone() } },
            Some(Err(_)) => rsx! {
                main { style: "padding: 24px; color: #4a2d1b; background: #ffe3a5; font-size: 20px;",
                    h1 { "เปิดสมุดบัญชีไม่ได้" }
                    p { "ลองเปิดอีกครั้ง ข้อมูลเดิมจะไม่ถูกลบทิ้ง" }
                    button { onclick: move |_| { bootstrap.restart(); }, "ลองอีกครั้ง" }
                }
            },
            None => {
                rsx! { main { style: "padding: 24px; color: #4a2d1b; background: #ffe3a5; font-size: 20px;", "กำลังเปิดสมุดบัญชีในเครื่อง…" } }
            }
        }
    }

    #[component]
    fn Ready(gateway: Gateway) -> Element {
        use_context_provider(|| gateway);
        rsx! { ledger_ui::App {} }
    }
}

#[cfg(target_os = "android")]
fn main() {
    app::launch();
}

#[cfg(not(target_os = "android"))]
fn main() {
    eprintln!("Use scripts/build-android.ps1 to build this Android host.");
}
#[cfg(target_os = "android")]
mod receipt;
#[cfg(target_os = "android")]
mod voice;
