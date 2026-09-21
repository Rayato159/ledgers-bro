//! Bundled presentation fonts: no network request or system-font installation.
use base64::{Engine, engine::general_purpose::STANDARD};
use std::sync::OnceLock;

pub fn stylesheet() -> &'static str {
    static STYLESHEET: OnceLock<String> = OnceLock::new();
    STYLESHEET.get_or_init(|| {
        let faces: [(&str, u16, &[u8]); 4] = [
            ("normal", 400, include_bytes!("../assets/fonts/THSarabunNew.ttf")),
            ("normal", 700, include_bytes!("../assets/fonts/THSarabunNew Bold.ttf")),
            ("italic", 400, include_bytes!("../assets/fonts/THSarabunNew Italic.ttf")),
            ("italic", 700, include_bytes!("../assets/fonts/THSarabunNew BoldItalic.ttf")),
        ];
        faces.into_iter().map(|(style, weight, bytes)| {
            format!(
                "@font-face {{ font-family: 'TH Sarabun New'; font-style: {style}; font-weight: {weight}; font-display: swap; src: url(data:font/ttf;base64,{}) format('truetype'); }}",
                STANDARD.encode(bytes)
            )
        }).collect()
    })
}
