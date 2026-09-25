use crate::{
    components::*,
    i18n::tr,
    state::{Page, UiState},
    updates::Updates,
};
use dioxus::prelude::*;
use ledger_application::*;

#[derive(Clone, Copy)]
pub struct NotificationCenter {
    prefs: Signal<Option<AlertPreferences>>,
    items: Memo<Vec<LedgerAlert>>,
    open: Signal<bool>,
    error: Signal<Option<String>>,
}

pub fn use_notifications(store: UiState, updates: Updates) {
    let mut prefs = use_signal(|| None::<AlertPreferences>);
    let mut error = use_signal(|| None::<String>);
    let items = use_memo(move || {
        let Some(preferences) = prefs.read().clone() else {
            return Vec::new();
        };
        let mut alerts = store
            .view
            .read()
            .as_ref()
            .and_then(|v| ledger_alerts(v, &preferences).ok())
            .unwrap_or_default();
        if preferences.updates
            && let Some(release) = updates
                .checked
                .read()
                .as_ref()
                .and_then(|c| c.available.as_ref())
        {
            alerts.insert(
                0,
                LedgerAlert {
                    key: format!("update:{}", release.version),
                    kind: AlertKind::Update,
                    title: format!("Ledgers Bro {}", release.version),
                    due: None,
                    amount: None,
                },
            );
        }
        alerts
    });
    let center = NotificationCenter {
        prefs,
        items,
        open: use_signal(|| false),
        error,
    };
    use_context_provider(|| center);
    use_future(move || async move {
        let gateway = store.gateway.peek().clone();
        let mut saved = match gateway.0.alert_preferences().await {
            Ok(value) => {
                prefs.set(Some(value.clone()));
                value
            }
            Err(e) => {
                error.set(Some(e.to_string()));
                return;
            }
        };
        let mut last_check = None::<std::time::Instant>;
        let mut last_refresh = std::time::Instant::now();
        let mut attempted = Vec::<String>::new();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let Some(mut current) = prefs.peek().clone() else {
                continue;
            };
            if current.updates
                && gateway.0.supports_updates()
                && !*store.busy.peek()
                && last_check.is_none_or(|time| time.elapsed().as_secs() >= 21600)
            {
                updates.check(store);
                last_check = Some(std::time::Instant::now());
            }
            if last_refresh.elapsed().as_secs() >= 60 && !*store.busy.peek() {
                // Loading cannot mutate financial data. Refresh dates without restarting the app.
                store.send(Command::Load);
                last_refresh = std::time::Instant::now();
            }
            if current.system_enabled {
                let day = store
                    .view
                    .peek()
                    .as_ref()
                    .map(|v| v.today.to_string())
                    .unwrap_or_default();
                let fresh: Vec<_> = items
                    .peek()
                    .iter()
                    .filter(|item| !current.read.contains(&item.key))
                    .map(|item| format!("{day}:{}", item.key))
                    .filter(|key| !current.notified.contains(key) && !attempted.contains(key))
                    .collect();
                if !fresh.is_empty() {
                    // Native banners deliberately omit financial amounts and people's names.
                    let shown = gateway
                        .0
                        .system_notification(
                            tr("มีการแจ้งเตือนใหม่จาก Ledgers Bro"),
                            tr("เปิดกระดิ่งเพื่อดูอัปเดต บิลที่ต้องชำระ และรายการถึงเวลาทวงเงิน"),
                        )
                        .await;
                    match shown {
                        Ok(()) => {
                            current = prefs.peek().clone().unwrap_or(current);
                            for key in &fresh {
                                current.mark_notified(key);
                            }
                            prefs.set(Some(current.clone()));
                        }
                        Err(e) => error.set(Some(e.to_string())),
                    }
                    attempted.extend(fresh);
                    if attempted.len() > 512 {
                        attempted.drain(..attempted.len() - 512);
                    }
                }
            }
            if current != saved {
                match gateway.0.save_alert_preferences(current.clone()).await {
                    Ok(()) => saved = current,
                    Err(e) => {
                        error.set(Some(e.to_string()));
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    }
                }
            }
        }
    });
}

#[component]
pub fn NotificationBell() -> Element {
    let mut center = use_context::<NotificationCenter>();
    let mut store = use_context::<UiState>();
    let mut limit = use_signal(|| 20usize);
    let preferences = center.prefs.read().clone();
    let unread = center
        .items
        .read()
        .iter()
        .filter(|item| {
            preferences
                .as_ref()
                .is_some_and(|p| !p.read.contains(&item.key))
        })
        .count();
    rsx! {
        button { class: "notification-bell icon-button", "aria-label": tr("การแจ้งเตือน"), "aria-haspopup": "dialog", onclick: move |_| { center.open.set(true); limit.set(20); },
            Icon { name: "bell", size: 22 }
            if unread > 0 { span { class: "notification-count", "{unread.min(99)}" } }
        }
        if (center.open)() {
            dialog { id: "notifications-dialog", class: "account-dialog notifications-dialog", "aria-labelledby": "notifications-title",
                onmounted: move |_| { let _ = document::eval("document.getElementById('notifications-dialog').showModal()"); },
                oncancel: move |e| { e.prevent_default(); center.open.set(false); },
                div { class: "section-heading", h2 { id: "notifications-title", {tr("การแจ้งเตือน")} }
                    button { class: "icon-button", "aria-label": tr("ปิดหน้าต่าง"), onclick: move |_| center.open.set(false), Icon { name: "close", size: 20 } }
                }
                div { class: "notification-actions",
                    button { class: "text-button", disabled: unread == 0, onclick: move |_| {
                        if let Some(p) = center.prefs.write().as_mut() { for item in center.items.peek().iter() { p.mark_read(&item.key); } }
                    }, {tr("อ่านทั้งหมดแล้ว")} }
                    button { class: "text-button", disabled: (store.busy)(), onclick: move |_| { center.open.set(false); store.settings_tab.set(5); store.page.set(Page::Settings); }, {tr("ตั้งค่าการแจ้งเตือน")} }
                }
                if center.items.read().is_empty() { p { class: "muted", {tr("ไม่มีการแจ้งเตือนในตอนนี้")} } }
                for item in center.items.read().iter().take(limit()).cloned() {
                    { let read = preferences.as_ref().is_some_and(|p| p.read.contains(&item.key)); let key = item.key.clone();
                    rsx! { button { class: "notification-item", "data-read": read, disabled: (store.busy)(), onclick: move |_| {
                        if let Some(p) = center.prefs.write().as_mut() { p.mark_read(&key); }
                        center.open.set(false);
                        match item.kind { AlertKind::Update => { store.settings_tab.set(4); store.page.set(Page::Settings); }, AlertKind::Bill => store.page.set(Page::Recurring), AlertKind::Receivable => store.page.set(Page::Receivables) }
                    },
                        small { {tr(match item.kind { AlertKind::Update => "มีเวอร์ชันใหม่", AlertKind::Bill => "หนี้ที่ต้องชำระ", AlertKind::Receivable => "ถึงเวลาทวงเงิน" })} }
                        strong { "{item.title}" }
                        if let Some(due) = item.due { span { "{due}" } }
                        if let Some(amount) = item.amount { span { "{crate::i18n::currency_prefix()}{money_label(amount)}" } }
                    } } }
                }
                if center.items.read().len() > limit() { button { class: "soft-button", onclick: move |_| limit += 20, {tr("ดูเพิ่มเติม")} } }
                if let Some(error) = (center.error)() { p { class: "form-error", role: "alert", "{tr(&error)}" } }
            }
        }
    }
}

#[component]
pub fn NotificationSettings() -> Element {
    let mut center = use_context::<NotificationCenter>();
    let store = use_context::<UiState>();
    let preferences = center.prefs.read().clone();
    rsx! {
        h2 { class: "preferences-section-title", {tr("การแจ้งเตือน")} }
        p { class: "notification-settings-intro", {tr("เลือกสิ่งที่ต้องการให้แจ้งเตือนขณะเปิดแอป ตั้งค่าแยกสำหรับผู้ใช้แต่ละคน")} }
        section { class: "settings-group notification-settings",
            if let Some(preferences) = preferences {
                for (index, title, description, checked) in [
                    (0,"มีเวอร์ชันใหม่", "แจ้งเมื่อมี Ledgers Bro เวอร์ชันใหม่พร้อมดาวน์โหลด", preferences.updates),
                    (1,"หนี้ที่ต้องชำระ", "เตือนก่อนครบกำหนด 3 วัน และติดตามรายการที่เลยกำหนด", preferences.bills),
                    (2,"ถึงเวลาทวงเงิน", "เตือนเมื่อลูกหนี้ถึงวันนัดเก็บเงินและยังมียอดค้าง", preferences.receivables),
                    (3,"การแจ้งเตือนของระบบ", "แสดงการแจ้งเตือนบนอุปกรณ์ นอกหน้าต่างแอป", preferences.system_enabled)
                ] {
                    div { class: "notification-setting-row",
                        div { class: "notification-setting-copy",
                            h3 { id: "notification-setting-{index}", "{tr(title)}" }
                            p { id: "notification-description-{index}", "{tr(description)}" }
                        }
                        button { r#type: "button", class: "preference-switch", role: "switch", "aria-labelledby": "notification-setting-{index}", "aria-describedby": "notification-description-{index}", "aria-checked": checked, disabled: (store.busy)(), onclick: move |_| {
                            let enabled = !checked;
                            let details = format!("{} · {}", tr(title), tr(if enabled { "เปิดการแจ้งเตือน" } else { "ปิดการแจ้งเตือน" }));
                            crate::confirmation::ask(details, move |_| {
                                let gateway = store.gateway.peek().clone();
                                crate::state::spawn_session(async move {
                                    if index == 3 && enabled {
                                        match gateway.0.enable_system_notifications().await {
                                            Ok(true) => {},
                                            Ok(false) => { center.error.set(Some("อนุญาตการแจ้งเตือนในระบบ แล้วเปิดสวิตช์อีกครั้ง".into())); return; },
                                            Err(e) => { center.error.set(Some(e.to_string())); return; }
                                        }
                                    }
                                    if let Some(p) = center.prefs.write().as_mut() { match index { 0 => p.updates = enabled, 1 => p.bills = enabled, 2 => p.receivables = enabled, _ => p.system_enabled = enabled } }
                                    center.error.set(None);
                                });
                            });
                        }, span { class: "preference-switch-track", "aria-hidden": "true", span { class: "preference-switch-thumb" } } }
                    }
                }
                p { class: "notification-settings-note", {tr("การแจ้งเตือนระบบไม่แสดงยอดเงินหรือชื่อลูกหนี้ ปิดแอปแล้วจะยังไม่มีการแจ้งเตือนเบื้องหลัง")} }
            } else { p { {tr("กำลังโหลดการแจ้งเตือน…")} } }
            if let Some(error) = (center.error)() { p { class: "form-error", role: "alert", "{tr(&error)}" } }
        }
    }
}
