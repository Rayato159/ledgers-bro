use crate::{
    components::*,
    i18n::tr,
    state::{Page, UiState},
    updates::Updates,
};
use dioxus::prelude::*;
use futures_util::StreamExt;
use ledger_application::*;

enum NotificationChange {
    Archive(Vec<LedgerAlert>),
    Preference(usize, bool),
    Notified(Vec<String>),
}

#[derive(Clone, Copy)]
pub struct NotificationCenter {
    prefs: Signal<Option<AlertPreferences>>,
    items: Memo<Vec<LedgerAlert>>,
    open: Signal<bool>,
    error: Signal<Option<String>>,
    pending: Signal<usize>,
    writer: Coroutine<NotificationChange>,
}
impl NotificationCenter {
    pub fn saving(self) -> bool {
        (self.pending)() > 0
    }
    fn change(mut self, change: NotificationChange) {
        self.pending += 1;
        self.writer.send(change);
    }
}

pub fn use_notifications(store: UiState, updates: Updates) {
    let mut prefs = use_signal(|| None::<AlertPreferences>);
    let mut error = use_signal(|| None::<String>);
    let mut pending = use_signal(|| 0usize);
    let mut migrated = use_signal(|| false);
    // One writer owns all preference changes. A delayed system banner must not
    // overwrite an archive or setting saved while its native call was pending.
    let writer = use_coroutine(
        move |mut rx: UnboundedReceiver<NotificationChange>| async move {
            while let Some(change) = rx.next().await {
                let previous = prefs.peek().clone();
                if let Some(previous) = previous {
                    let mut next = previous.clone();
                    match change {
                        NotificationChange::Archive(items) => {
                            for item in &items {
                                next.archive(item);
                            }
                        }
                        NotificationChange::Preference(index, enabled) => match index {
                            0 => next.updates = enabled,
                            1 => next.bills = enabled,
                            2 => next.receivables = enabled,
                            _ => next.system_enabled = enabled,
                        },
                        NotificationChange::Notified(keys) => {
                            for key in &keys {
                                next.mark_notified(key);
                            }
                        }
                    }
                    prefs.set(Some(next.clone()));
                    let gateway = store.gateway.peek().clone();
                    match gateway.0.save_alert_preferences(next).await {
                        Ok(()) => error.set(None),
                        Err(e) => {
                            prefs.set(Some(previous));
                            error.set(Some(e.to_string()));
                        }
                    }
                }
                pending -= 1;
            }
        },
    );
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
        pending,
        writer,
    };
    use_context_provider(|| center);
    use_future(move || async move {
        let gateway = store.gateway.peek().clone();
        match gateway.0.alert_preferences().await {
            Ok(value) => {
                prefs.set(Some(value));
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
            let Some(current) = prefs.peek().clone() else {
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
                    .filter(|item| !current.is_read(&item.key))
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
                            center.change(NotificationChange::Notified(fresh.clone()));
                        }
                        Err(e) => error.set(Some(e.to_string())),
                    }
                    attempted.extend(fresh);
                    if attempted.len() > 512 {
                        attempted.drain(..attempted.len() - 512);
                    }
                }
            }
        }
    });
    // Preserve currently known read notifications from older versions.
    use_effect(move || {
        if migrated() || center.saving() || store.view.read().is_none() {
            return;
        }
        if let Some(p) = prefs.read().as_ref() {
            migrated.set(true);
            let legacy: Vec<_> = items
                .read()
                .iter()
                .filter(|a| {
                    p.read.contains(&a.key) && !p.archived.iter().any(|old| old.key == a.key)
                })
                .cloned()
                .collect();
            if !legacy.is_empty() {
                center.change(NotificationChange::Archive(legacy));
            }
        }
    });
}

#[component]
pub fn NotificationBell() -> Element {
    let mut center = use_context::<NotificationCenter>();
    let mut store = use_context::<UiState>();
    let mut limit = use_signal(|| 20usize);
    let mut archived = use_signal(|| false);
    let preferences = center.prefs.read().clone();
    let unread_items: Vec<_> = center
        .items
        .read()
        .iter()
        .filter(|item| preferences.as_ref().is_some_and(|p| !p.is_read(&item.key)))
        .cloned()
        .collect();
    let unread = unread_items.len();
    let history: Vec<_> = preferences
        .as_ref()
        .map(|p| p.archived.iter().filter_map(|a| a.alert().ok()).collect())
        .unwrap_or_default();
    let archive_count = history.len();
    let visible = if archived() {
        history
    } else {
        unread_items.clone()
    };
    let saving = center.saving();
    rsx! {
        button { class: "notification-bell icon-button", "aria-label": tr("การแจ้งเตือน"), "aria-haspopup": "dialog", onclick: move |_| { center.open.set(true); archived.set(false); limit.set(20); },
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
                div { class: "notification-tabs", role: "tablist", "aria-label": tr("การแจ้งเตือน"),
                    for (value, title, count) in [(false, "ยังไม่อ่าน", unread), (true, "เก็บถาวร", archive_count)] {
                        button { id: "notification-tab-{value}", role: "tab", "aria-selected": archived() == value, "aria-controls": "notification-list",
                            onclick: move |_| { archived.set(value); limit.set(20); },
                            onkeydown: move |e| {
                                if matches!(e.key(), Key::ArrowLeft | Key::ArrowRight | Key::Home | Key::End) {
                                    e.prevent_default();
                                    let next = match e.key() { Key::Home => false, Key::End => true, _ => !archived() };
                                    archived.set(next); limit.set(20);
                                    let _ = document::eval(&format!("document.getElementById('notification-tab-{next}').focus()"));
                                }
                            },
                            span { "{tr(title)}" } span { class: "notification-tab-count", "{count}" }
                        }
                    }
                }
                div { class: "notification-actions",
                    if !archived() { button { class: "text-button", disabled: unread == 0 || saving || (store.busy)(), onclick: move |_| {
                        center.change(NotificationChange::Archive(unread_items.clone()));
                    }, {tr("อ่านและเก็บทั้งหมด")} } }
                    button { class: "text-button", disabled: (store.busy)(), onclick: move |_| { center.open.set(false); store.settings_tab.set(5); store.page.set(Page::Settings); }, {tr("ตั้งค่าการแจ้งเตือน")} }
                }
                div { id: "notification-list", role: "tabpanel", "aria-labelledby": "notification-tab-{archived()}", "aria-busy": saving,
                    if visible.is_empty() { p { class: "notification-empty", {tr(if archived() { "ยังไม่มีการแจ้งเตือนที่เก็บไว้" } else { "อ่านครบแล้ว ไม่มีการแจ้งเตือนใหม่" })} } }
                    for item in visible.iter().take(limit()).cloned() {
                        { let snapshot = item.clone();
                        rsx! { button { key: "{item.key}", class: "notification-item", "data-read": archived(), disabled: (store.busy)() || saving, onclick: move |_| {
                            if !archived() { center.change(NotificationChange::Archive(vec![snapshot.clone()])); }
                            center.open.set(false);
                            match item.kind { AlertKind::Update => { store.settings_tab.set(4); store.page.set(Page::Settings); }, AlertKind::Bill => store.page.set(Page::Recurring), AlertKind::Receivable => store.page.set(Page::Receivables) }
                        },
                            small { {tr(match item.kind { AlertKind::Update => "มีเวอร์ชันใหม่", AlertKind::Bill => "หนี้ที่ต้องชำระ", AlertKind::Receivable => "ถึงเวลาทวงเงิน" })} }
                            strong { "{item.title}" }
                            if let Some(due) = item.due { span { "{due}" } }
                            if let Some(amount) = item.amount { span { class: "notification-amount", "{crate::i18n::currency_prefix()}{money_label(amount)}" } }
                        } } }
                    }
                    if visible.len() > limit() { button { class: "soft-button", onclick: move |_| limit += 20, {tr("ดูเพิ่มเติม")} } }
                }
                if archived() { p { class: "notification-archive-note", {tr("เก็บการแจ้งเตือนที่อ่านแล้วล่าสุด 256 รายการ ยอดเงินเป็นข้อมูล ณ ตอนที่อ่าน")} } }
                if saving { p { class: "muted", role: "status", {tr("กำลังบันทึก…")} } }
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
                        button { r#type: "button", class: "preference-switch", role: "switch", "aria-labelledby": "notification-setting-{index}", "aria-describedby": "notification-description-{index}", "aria-checked": checked, disabled: (store.busy)() || center.saving(), onclick: move |_| {
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
                                    center.change(NotificationChange::Preference(index, enabled));
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
