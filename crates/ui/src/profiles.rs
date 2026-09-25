use crate::{Gateway, HostInfo, components::Icon, i18n::tr};
use dioxus::prelude::*;
use ledger_application::*;

#[derive(Clone, Copy)]
struct LoginState {
    session: Signal<Option<ProfileSession>>,
    refresh: Signal<u64>,
}

#[component]
pub fn LoginRoot() -> Element {
    let gateway = use_context::<Gateway>();
    let host = use_context::<HostInfo>();
    let mut state = LoginState {
        session: use_signal(|| None),
        refresh: use_signal(|| 0),
    };
    use_context_provider(|| state);
    let mut language = use_signal(crate::i18n::Language::default);
    use_context_provider(|| crate::i18n::Locale(language));
    let loader = gateway.clone();
    let profiles = use_resource(move || {
        let _ = (state.refresh)();
        let gateway = loader.clone();
        async move { gateway.0.profiles(ProfileCommand::List).await }
    });
    let mut selected = use_signal(|| None::<UserProfile>);
    let mut creating = use_signal(|| false);
    let mut resume_finished = use_signal(|| false);
    let mut resume_error = use_signal(|| None::<String>);
    let resume_gateway = gateway.clone();
    use_future(move || {
        let gateway = resume_gateway.clone();
        async move {
            match gateway.0.profiles(ProfileCommand::Resume).await {
                Ok(ProfileResponse::Authenticated(session)) => state.session.set(Some(session)),
                Ok(_) => {}
                Err(error) => resume_error.set(Some(error.to_string())),
            }
            resume_finished.set(true);
        }
    });
    use_effect(move || {
        if (state.session)().is_none() {
            selected.set(None);
            creating.set(false);
            let _ = document::eval("document.documentElement.dataset.theme='light'");
        }
    });
    if let Some(session) = (state.session)() {
        return match gateway.0.authenticated(&session.token) {
            Ok(bound) => rsx! { AuthenticatedApp { key:"{session.token}", gateway:bound } },
            Err(error) => {
                rsx! { p { role:"alert", "{error}" } button { onclick:move |_|state.session.set(None), {tr("กลับไปเข้าสู่ระบบ")} } }
            }
        };
    }
    rsx! {
        style { {crate::typography::stylesheet()} }
        style { {include_str!("../assets/app.css")} }
        style { {crate::theme::stylesheet(UserPreferences::default())} }
        main { class:"login-shell",
            section { class:"card login-card",
                img { class:"login-mascot", src:host.art.hero.clone(), alt:"" }
                h1 { "Ledgers Bro" }
                p { class:"muted", {tr("เลือกผู้ใช้เพื่อเปิดสมุดบัญชีของตัวเอง")} }
                div { class:"login-language", select {"aria-label":"Language / ภาษา",value:if language()==crate::i18n::Language::English {"en"}else{"th"},onchange:move |e|language.set(if e.value()=="en"{crate::i18n::Language::English}else{crate::i18n::Language::Thai}),option {value:"th","ไทย"} option {value:"en","English"}} }
                if let Some(error) = resume_error() { p {class:"form-error",role:"alert",{tr(&error)}} }
                if !resume_finished() { p {role:"status",{tr("กำลังตรวจสอบการเข้าสู่ระบบที่จดจำไว้…")}} }
                else { match profiles.read().as_ref() {
                    Some(Ok(ProfileResponse::Profiles(users)))=>rsx! {
                        if !users.is_empty() {
                            div { class:"profile-list", role:"group", "aria-label":tr("ผู้ใช้ในเครื่องนี้"),
                                for user in users.clone() {
                                    { let chosen=selected().is_some_and(|p|p.id==user.id) && !creating();
                                      rsx! { button { class:if chosen {"profile-choice selected"} else {"profile-choice"}, "aria-pressed":chosen,
                                        onclick:move |_| { selected.set(Some(user.clone())); creating.set(false); },
                                        Icon { name:"user", size:22 }
                                        span { strong { "{user.username}" } if user.needs_password { small { {tr("ข้อมูลเดิม · ตั้งรหัสผ่านครั้งแรก")} } } }
                                        Icon { name:"arrow-right", size:17 }
                                      } }
                                    }
                                }
                            }
                        }
                        if creating() || users.is_empty() { LoginForm { profile:None, onlogin:move |session|state.session.set(Some(session)) } }
                        else if let Some(profile)=selected() { LoginForm { key:"{profile.id}", profile:Some(profile), onlogin:move |session|state.session.set(Some(session)) } }
                        button { class:"soft-button full-width", onclick:move |_| { creating.set(!creating()); selected.set(None); }, Icon {name:"plus",size:18} {tr("สร้างผู้ใช้ใหม่")} }
                    },
                    Some(Err(error))=>rsx! { p { class:"form-error",role:"alert","{error}" } button {class:"soft-button",onclick:move |_|state.refresh+=1,{tr("ลองอีกครั้ง")}} },
                    _=>rsx! { p {role:"status",{tr("กำลังอ่านข้อมูลผู้ใช้…")}} },
                } }
                p { class:"field-hint", {tr("ผู้ใช้และข้อมูลเก็บในเครื่องนี้ ไม่มีการส่งรหัสผ่านขึ้นเซิร์ฟเวอร์")} }
            }
        }
    }
}

#[component]
fn AuthenticatedApp(gateway: Gateway) -> Element {
    use_context_provider(|| gateway);
    rsx! { crate::App {} }
}

#[component]
fn LoginForm(profile: Option<UserProfile>, onlogin: EventHandler<ProfileSession>) -> Element {
    let gateway = use_context::<Gateway>();
    let mut username = use_signal(|| {
        profile
            .as_ref()
            .map(|p| p.username.clone())
            .unwrap_or_default()
    });
    let mut password = use_signal(String::new);
    let mut confirmation = use_signal(String::new);
    let mut remember = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let setup = profile.as_ref().is_none_or(|p| p.needs_password);
    let legacy = profile.as_ref().is_some_and(|p| p.needs_password);
    rsx! {
        form { class:"login-form", onsubmit:move |event| {
            event.prevent_default();
            if busy() {return;}
            if setup && password()!=confirmation() {error.set(Some(tr("รหัสผ่านทั้งสองช่องไม่ตรงกัน")));return;}
            let command=match &profile {
                Some(p) if p.needs_password=>ProfileCommand::ClaimLegacy {id:p.id.clone(),username:username(),password:password(),remember:remember()},
                Some(p)=>ProfileCommand::Login {id:p.id.clone(),password:password(),remember:remember()},
                None=>ProfileCommand::Create {username:username(),password:password(),remember:remember()},
            };
            let gateway=gateway.clone();busy.set(true);error.set(None);
            spawn(async move {
                let result=gateway.0.profiles(command).await;
                password.set(String::new());confirmation.set(String::new());busy.set(false);
                match result {Ok(ProfileResponse::Authenticated(session))=>onlogin.call(session),Err(e)=>error.set(Some(tr(&e.to_string()))),_=>error.set(Some(tr("เข้าสู่ระบบไม่สำเร็จ")))}
            });
        },
            if legacy {p {class:"field-hint",{tr("สมุดเดิมถูกผูกกับผู้ใช้นี้แล้ว ตั้งชื่อและรหัสผ่านเพื่อเข้าใช้งาน ข้อมูลเดิมยังอยู่ครบ")}}}
            if setup {
                label {r#for:"login-username",{tr("ชื่อผู้ใช้")}}
                input {id:"login-username",autocomplete:"username",value:username(),maxlength:"40",required:true,disabled:busy(),oninput:move |e|username.set(e.value())}
            }
            label {r#for:"login-password",{tr(if setup {"ตั้งรหัสผ่าน"} else {"รหัสผ่าน"})}}
            input {id:"login-password",r#type:"password",autocomplete:if setup {"new-password"} else {"current-password"},value:password(),required:true,disabled:busy(),oninput:move |e|password.set(e.value())}
            if setup {
                p {class:"field-hint",{tr("อย่างน้อย 8 ตัวอักษร")}}
                label {r#for:"login-confirm",{tr("ยืนยันรหัสผ่าน")}}
                input {id:"login-confirm",r#type:"password",autocomplete:"new-password",value:confirmation(),required:true,disabled:busy(),oninput:move |e|confirmation.set(e.value())}
            }
            label {class:"remember-login",r#for:"login-remember",
                input {id:"login-remember",r#type:"checkbox",checked:remember(),disabled:busy(),onchange:move |e|remember.set(e.checked()),"aria-describedby":"login-remember-hint"}
                span {{tr("จดจำฉัน 7 วัน")}}
            }
            p {id:"login-remember-hint",class:"field-hint",{tr("เปิดแอปแล้วเข้าใช้ผู้ใช้นี้อัตโนมัติบนเครื่องนี้ ออกจากระบบเพื่อยกเลิก")}}
            if let Some(message)=error() {p {class:"form-error",role:"alert","{message}"}}
            button {class:"primary full-width",r#type:"submit",disabled:busy(),{tr(if busy(){"กำลังตรวจสอบ…"}else if setup{"บันทึกและเข้าใช้งาน"}else{"เข้าสู่ระบบ"})}}
        }
    }
}

#[component]
pub fn ProfileSettings() -> Element {
    let notifications = use_context::<crate::notifications::NotificationCenter>();
    let mut state = use_context::<LoginState>();
    let store = use_context::<crate::state::UiState>();
    let gateway = use_context::<Gateway>();
    let mut editing = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let Some(session) = (state.session)() else {
        return rsx! {};
    };
    let token = session.token.clone();
    rsx! {
        div {class:"setting-row profile-settings",
            div {class:"setting-copy",
                h3 {{tr("ผู้ใช้ที่เข้าสู่ระบบ")}}
                p {class:"profile-current",Icon {name:"user",size:18} span {"{session.profile.username}"}}
            }
            div {class:"profile-settings-actions",
            button {class:"soft-button",disabled:busy()||*store.busy.read()||notifications.saving(),onclick:move |_|editing.set(true),Icon {name:"edit",size:16}{tr("แก้ไขผู้ใช้")}}
            button {class:"soft-button",disabled:busy()||*store.busy.read()||notifications.saving(),onclick:move |_| {
                let gateway=gateway.clone();let token=token.clone();busy.set(true);error.set(None);
                spawn(async move {
                    match gateway.0.profiles(ProfileCommand::Logout {token}).await {
                        Ok(ProfileResponse::SignedOut)=>{state.session.set(None);state.refresh+=1;},
                        Err(e)=>error.set(Some(tr(&e.to_string()))),_=>{},
                    }
                    busy.set(false);
                });
            },Icon {name:"arrow-right",size:16}{tr("สลับผู้ใช้ / ออกจากระบบ")}}
            if let Some(message)=error() {p {class:"form-error",role:"alert","{message}"}}
            }
        }
        if editing() { EditProfile { session, onclose:move |_|editing.set(false) } }
    }
}

#[component]
fn EditProfile(session: ProfileSession, onclose: EventHandler) -> Element {
    let gateway = use_context::<Gateway>();
    let mut state = use_context::<LoginState>();
    let mut username = use_signal(|| session.profile.username.clone());
    let mut current = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirm = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    rsx! {
        dialog {id:"profile-dialog",class:"account-dialog", "aria-labelledby":"profile-title",onmounted:move |_|{let _=document::eval("document.getElementById('profile-dialog').showModal()");},oncancel:move |e|{e.prevent_default();if !busy(){onclose.call(());}},
            form {onsubmit:move |e|{
                e.prevent_default();if busy(){return;}
                if password()!=confirm(){error.set(Some(tr("รหัสผ่านทั้งสองช่องไม่ตรงกัน")));return;}
                let command=ProfileCommand::Edit {token:session.token.clone(),username:username(),current_password:current(),new_password:(!password().is_empty()).then_some(password())};
                let mut command = Some(command);
                let gateway = gateway.clone();
                crate::confirmation::ask(format!("{}: {}", tr("ชื่อผู้ใช้"), username()), move |_| {
                let Some(command) = command.take() else { return; };
                let gateway=gateway.clone();busy.set(true);error.set(None);
                spawn(async move{
                    let result=gateway.0.profiles(command).await;
                    current.set(String::new());password.set(String::new());confirm.set(String::new());busy.set(false);
                    match result {Ok(ProfileResponse::Authenticated(s))=>{state.session.set(Some(s));onclose.call(());},Err(e)=>error.set(Some(tr(&e.to_string()))),_=>{}}
                });
                });
            },
                div {class:"section-heading",h2 {id:"profile-title",{tr("แก้ไขผู้ใช้")}} button {class:"icon-button",r#type:"button",disabled:busy(),onclick:move |_|onclose.call(()),"aria-label":tr("ปิด"),Icon {name:"close",size:20}}}
                label {r#for:"profile-name",{tr("ชื่อผู้ใช้")}} input {id:"profile-name",value:username(),maxlength:"40",required:true,disabled:busy(),oninput:move |e|username.set(e.value())}
                label {r#for:"profile-current",{tr("รหัสผ่านปัจจุบัน")}} input {id:"profile-current",r#type:"password",autocomplete:"current-password",value:current(),required:true,disabled:busy(),oninput:move |e|current.set(e.value())}
                label {r#for:"profile-password",{tr("รหัสผ่านใหม่ (เว้นว่างเพื่อใช้รหัสเดิม)")}} input {id:"profile-password",r#type:"password",autocomplete:"new-password",value:password(),disabled:busy(),oninput:move |e|password.set(e.value())}
                label {r#for:"profile-confirm",{tr("ยืนยันรหัสผ่านใหม่")}} input {id:"profile-confirm",r#type:"password",autocomplete:"new-password",value:confirm(),disabled:busy(),oninput:move |e|confirm.set(e.value())}
                if let Some(message)=error(){p {class:"form-error",role:"alert","{message}"}}
                button {class:"primary full-width",disabled:busy(),r#type:"submit",{tr("บันทึก")}}
            }
        }
    }
}
