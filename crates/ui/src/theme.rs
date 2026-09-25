use crate::{
    components::Icon,
    i18n::{Language, Locale, tr},
    state::UiState,
};
use dioxus::prelude::*;
use ledger_application::{Command, Response, UserPreferences};

#[derive(Clone, Copy)]
pub struct Theme(pub Signal<UserPreferences>);

const PALETTES: [(&str, u32); 5] = [
    ("Lavender", 0xbda0ff),
    ("Ocean", 0x63b5ed),
    ("Mint", 0x60c5a2),
    ("Peach", 0xefac7c),
    ("Rose", 0xe996bc),
];

pub fn persist(mut store: UiState, mut theme: Theme, mut locale: Locale, next: UserPreferences) {
    if next == *theme.0.peek() {
        return;
    }
    crate::confirmation::ask(
        tr("บันทึกการตั้งค่าที่เลือก"),
        move |_| {
            if *store.busy.peek() {
                return;
            }
            store.busy.set(true);
            let gateway = store.gateway.peek().clone();
            crate::state::spawn_session(async move {
                match gateway.0.request(Command::SetPreferences(next)).await {
                    Ok(Response::Preferences(saved)) => {
                        theme.0.set(saved);
                        locale.0.set(if saved.english {
                            Language::English
                        } else {
                            Language::Thai
                        });
                    }
                    _ => store
                        .notice
                        .set(Some((true, "บันทึกการตั้งค่าไม่ได้ กรุณาลองอีกครั้ง".into()))),
                }
                store.busy.set(false);
            });
        },
    );
}

#[component]
pub fn ThemePicker() -> Element {
    let theme = use_context::<Theme>();
    let locale = use_context::<Locale>();
    let store = use_context::<UiState>();
    let saved = *theme.0.read();
    let mut color_text = use_signal(|| format!("#{:06x}", saved.primary_color));
    use_effect(move || color_text.set(format!("#{:06x}", theme.0.read().primary_color)));
    let parsed = parse_color(&color_text());
    rsx! {
        div { class: "setting-row",
            div { class: "setting-copy", h3 { {tr("โหมดการแสดงผล")} } p { {tr("เลือกบรรยากาศที่สบายตา")} } }
            div { class: "setting-control",
                select { id: "theme-mode", "aria-label": tr("โหมดการแสดงผล"), value: if saved.dark { "dark" } else { "light" }, disabled: *store.busy.read(), onchange: move |e| {
                    let current = if theme.0.peek().dark { "dark" } else { "light" };
                    let _ = document::eval(&format!("document.getElementById('theme-mode').value='{current}'"));
                    persist(store, theme, locale, UserPreferences { dark: e.value() == "dark", ..*theme.0.peek() });
                },
                    option { value: "light", selected: !saved.dark, {tr("สว่าง")} }
                    option { value: "dark", selected: saved.dark, {tr("มืด")} }
                }
            }
        }
        div { class: "setting-row setting-row-palette",
        h3 { class: "theme-section-title", {tr("ชุดสี")} }
        div { class: "palette-options", role: "group", "aria-label": tr("ชุดสี"),
            for (name, color) in PALETTES {
                button { class: "palette-option", r#type: "button", "aria-pressed": saved.primary_color == color, disabled: *store.busy.read(),
                    onclick: move |_| persist(store, theme, locale, UserPreferences { primary_color: color, ..*theme.0.peek() }),
                    span { class: "palette-preview", style: preview_style(color, saved.dark, saved.gradient),
                        span { class: "palette-preview-bar" } span { class: "palette-preview-card" } span { class: "palette-preview-dot" }
                        if saved.primary_color == color { span { class: "palette-check", Icon { name: "check", size: 16 } } }
                    }
                    span { "{name}" }
                }
            }
        }
        }
        div { class: "setting-row",
            div { class: "setting-copy", h3 { label { r#for: "theme-color-text", {tr("ปรับสีหลักเอง")} } } p { {tr("ปรับเฉพาะสีหลัก สีพื้นและตัวอักษรจะปรับตามเพื่อให้อ่านง่าย")} } }
            div { class: "setting-control custom-color-row",
                input { class: "color-picker", r#type: "color", "aria-label": tr("เลือกสีหลัก"), value: format!("#{:06x}", parsed.unwrap_or(saved.primary_color)), disabled: *store.busy.read(), oninput: move |e| color_text.set(e.value()) }
                input { id: "theme-color-text", r#type: "text", maxlength: "7", spellcheck: "false", value: color_text(), placeholder: "#BDA0FF", oninput: move |e| color_text.set(e.value()) }
                button { class: "soft-button", disabled: *store.busy.read() || parsed.is_none() || parsed == Some(saved.primary_color),
                    onclick: move |_| { if let Some(primary_color) = parse_color(&color_text()) { persist(store, theme, locale, UserPreferences { primary_color, ..*theme.0.peek() }); } }, {tr("ใช้สีนี้")}
                }
            }
        }
        if parsed.is_none() { p { class: "field-hint", role: "status", {tr("ใส่สีแบบ #RRGGBB เช่น #BDA0FF")} } }
        div { class: "setting-row",
            div { class: "setting-copy", h3 { {tr("ไล่สี Gradient")} } p { {tr("ใช้สีหลักคู่กับสีข้างเคียง หรือปิดเพื่อใช้สีเดียว")} } }
            button { class: "preference-switch", r#type: "button", role: "switch", "aria-label": tr("ไล่สี Gradient"), "aria-checked": saved.gradient, disabled: *store.busy.read(), onclick: move |_| persist(store, theme, locale, UserPreferences { gradient: !saved.gradient, ..*theme.0.peek() }), span { class: "preference-switch-track", "aria-hidden": "true", span { class: "preference-switch-thumb" } } }
        }
    }
}

fn parse_color(input: &str) -> Option<u32> {
    let hex = input.strip_prefix('#')?;
    (hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| u32::from_str_radix(hex, 16).ok())
        .flatten()
}

fn rgb(color: u32) -> [f64; 3] {
    [
        ((color >> 16) & 255) as f64 / 255.0,
        ((color >> 8) & 255) as f64 / 255.0,
        (color & 255) as f64 / 255.0,
    ]
}
fn hue(color: u32) -> f64 {
    let [r, g, b] = rgb(color);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    if delta < 0.0001 {
        return 270.0;
    }
    (60.0
        * if max == r {
            (g - b) / delta
        } else if max == g {
            (b - r) / delta + 2.0
        } else {
            (r - g) / delta + 4.0
        })
    .rem_euclid(360.0)
}
fn hsl(hue: f64, saturation: f64, lightness: f64) -> u32 {
    let a = saturation * lightness.min(1.0 - lightness);
    let channel = |n: f64| {
        let k = (n + hue / 30.0).rem_euclid(12.0);
        ((lightness - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0)) * 255.0).round() as u32
    };
    (channel(0.0) << 16) | (channel(8.0) << 8) | channel(4.0)
}
fn luminance(color: u32) -> f64 {
    let linear = rgb(color).map(|v| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    });
    linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722
}
fn readable_ink(color: u32) -> u32 {
    if luminance(color) > 0.179 {
        0x000000
    } else {
        0xffffff
    }
}
struct Colors {
    canvas: u32,
    paper: u32,
    surface: u32,
    raised: u32,
    field: u32,
    ink: u32,
    muted: u32,
    line: u32,
    accent: u32,
    secondary: u32,
    action_ink: u32,
    hero: u32,
    hero_end: u32,
}
fn colors(p: UserPreferences) -> Colors {
    let h = hue(p.primary_color);
    let c = |s, l| hsl(h, s, l);
    let secondary_hue = (h + 35.0).rem_euclid(360.0);
    // Related hues have equal lightness; preserve the primary's luminance for readable buttons.
    let secondary = if luminance(p.primary_color) > 0.179 {
        hsl(secondary_hue, 0.65, 0.78)
    } else {
        hsl(secondary_hue, 0.50, 0.27)
    };
    Colors {
        canvas: c(0.18, if p.dark { 0.09 } else { 0.975 }),
        paper: c(0.17, if p.dark { 0.135 } else { 0.998 }),
        surface: c(0.21, if p.dark { 0.19 } else { 0.945 }),
        raised: c(0.25, if p.dark { 0.24 } else { 0.90 }),
        field: c(0.18, if p.dark { 0.105 } else { 1.0 }),
        ink: c(0.20, if p.dark { 0.94 } else { 0.16 }),
        muted: c(0.15, if p.dark { 0.83 } else { 0.31 }),
        line: c(0.19, if p.dark { 0.31 } else { 0.83 }),
        accent: p.primary_color,
        secondary,
        action_ink: readable_ink(p.primary_color),
        hero: c(0.45, if p.dark { 0.20 } else { 0.91 }),
        hero_end: hsl(secondary_hue, 0.48, if p.dark { 0.22 } else { 0.95 }),
    }
}
fn preview_style(color: u32, dark: bool, gradient: bool) -> String {
    let c = colors(UserPreferences {
        primary_color: color,
        dark,
        gradient,
        ..UserPreferences::default()
    });
    let action = if gradient {
        format!(
            "linear-gradient(115deg,#{:06x},#{:06x})",
            c.accent, c.secondary
        )
    } else {
        format!("#{:06x}", c.accent)
    };
    format!(
        "--preview-bg:#{:06x};--preview-surface:#{:06x};--preview-action:{};--preview-ink:#{:06x}",
        c.canvas, c.raised, action, c.action_ink
    )
}
pub fn stylesheet(p: UserPreferences) -> String {
    let c = colors(p);
    let action = if p.gradient {
        format!(
            "linear-gradient(110deg,#{:06x},#{:06x})",
            c.accent, c.secondary
        )
    } else {
        format!("#{:06x}", c.accent)
    };
    let hero = if p.gradient {
        format!(
            "linear-gradient(140deg,#{:06x},#{:06x})",
            c.hero, c.hero_end
        )
    } else {
        format!("#{:06x}", c.hero)
    };
    format!(
        ":root, :root[data-theme] {{ --canvas:#{:06x};--paper:#{:06x};--surface:#{:06x};--raised:#{:06x};--field:#{:06x};--ink:#{:06x};--muted:#{:06x};--line:#{:06x};--accent:#{:06x};--accent-ink:#{:06x};--action-gradient:{};--hero-gradient:{};--tone-hero:#{:06x}; color-scheme:{}; }}",
        c.canvas,
        c.paper,
        c.surface,
        c.raised,
        c.field,
        c.ink,
        c.muted,
        c.line,
        c.accent,
        c.action_ink,
        action,
        hero,
        c.hero,
        if p.dark { "dark" } else { "light" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn contrast(a: u32, b: u32) -> f64 {
        let a = luminance(a);
        let b = luminance(b);
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
    #[test]
    fn presets_and_extreme_custom_colors_stay_readable_in_both_modes() {
        for primary in PALETTES
            .map(|(_, c)| c)
            .into_iter()
            .chain([0, 0xffffff, 0xff0000, 0x00ff00, 0x0000ff, 0x777777])
        {
            for dark in [false, true] {
                let c = colors(UserPreferences {
                    primary_color: primary,
                    dark,
                    ..UserPreferences::default()
                });
                for background in [
                    c.paper, c.canvas, c.surface, c.raised, c.hero, c.hero_end, c.field,
                ] {
                    assert!(contrast(c.ink, background) >= 4.5);
                    assert!(contrast(c.muted, background) >= 4.5);
                }
                assert!(contrast(c.action_ink, c.accent) >= 4.5);
                assert!(contrast(c.action_ink, c.secondary) >= 4.5);
            }
        }
    }
    #[test]
    fn custom_color_is_strict_and_solid_mode_has_no_gradient() {
        assert_eq!(parse_color("#BDA0FF"), Some(0xbda0ff));
        for invalid in ["red", "#fff", "#1234567", "#gggggg", "#12345;"] {
            assert_eq!(parse_color(invalid), None);
        }
        assert!(
            !stylesheet(UserPreferences {
                gradient: false,
                ..UserPreferences::default()
            })
            .contains("linear-gradient")
        );
    }
}
