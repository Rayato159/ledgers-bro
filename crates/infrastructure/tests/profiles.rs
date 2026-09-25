#![allow(clippy::expect_used, clippy::panic)]
use futures_executor::block_on;
use ledger_application::*;
use ledger_domain::*;
use ledger_infrastructure::{ProfileWorker, RandomIds, SqliteLedger, SystemClock};

fn request(worker: &ProfileWorker, command: ProfileCommand) -> Result<ProfileResponse, AppError> {
    block_on(worker.request(command))
}
fn authenticated(response: ProfileResponse) -> ProfileSession {
    match response {
        ProfileResponse::Authenticated(s) => s,
        _ => panic!("session"),
    }
}
fn users(worker: &ProfileWorker) -> Vec<UserProfile> {
    match request(worker, ProfileCommand::List).expect("list") {
        ProfileResponse::Profiles(v) => v,
        _ => panic!("list"),
    }
}
fn create(worker: &ProfileWorker, name: &str) -> ProfileSession {
    authenticated(
        request(
            worker,
            ProfileCommand::Create {
                remember: false,
                username: name.into(),
                password: "test password 123".into(),
            },
        )
        .expect("create"),
    )
}

#[test]
fn legacy_adoption_is_idempotent_preserves_rows_and_requires_initial_password() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("ledger.sqlite3");
    let mut app = LedgerApplication::new(
        SqliteLedger::open(&path).expect("db"),
        SystemClock,
        RandomIds,
    );
    app.execute(Command::CreateAccount {
        name: "เงินเก่า".into(),
        kind: AccountKind::Cash,
        opening: "9876.54".into(),
        credit_cycle: None,
    })
    .expect("seed");
    let Response::Dashboard(before) = app.execute(Command::Load).expect("load") else {
        panic!("view")
    };
    drop(app);
    let worker = ProfileWorker::start(dir.path()).expect("worker");
    let list = users(&worker);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].username, "Lookhin");
    assert!(list[0].needs_password);
    assert!(
        request(
            &worker,
            ProfileCommand::Login {
                remember: false,
                id: list[0].id.clone(),
                password: String::new()
            }
        )
        .is_err()
    );
    assert!(worker.ledger("invented token").is_err());
    let session = authenticated(
        request(
            &worker,
            ProfileCommand::ClaimLegacy {
                remember: false,
                id: list[0].id.clone(),
                username: "เจ้าของ".into(),
                password: "รหัสทดสอบ12345".into(),
            },
        )
        .expect("claim"),
    );
    let ledger = worker.ledger(&session.token).expect("ledger");
    let Response::Dashboard(after) = block_on(ledger.request(Command::Load)).expect("load") else {
        panic!("view")
    };
    assert_eq!(before, after);
    assert!(
        request(
            &worker,
            ProfileCommand::ClaimLegacy {
                remember: false,
                id: list[0].id.clone(),
                username: "attacker".into(),
                password: "otherpassword".into()
            }
        )
        .is_err()
    );
    let restarted = ProfileWorker::start(dir.path()).expect("restart");
    assert_eq!(users(&restarted), vec![session.profile]);
    assert!(path.is_file());
}

#[test]
fn profiles_are_isolated_and_revoked_workers_cannot_read_or_write() {
    let dir = tempfile::tempdir().expect("directory");
    let worker = ProfileWorker::start(dir.path()).expect("worker");
    let one = create(&worker, "Alice");
    let first = worker.ledger(&one.token).expect("first");
    block_on(first.request(Command::CreateAccount {
        name: "Alice cash".into(),
        kind: AccountKind::Cash,
        opening: "100".into(),
        credit_cycle: None,
    }))
    .expect("seed");
    let two = create(&worker, "Bob");
    assert!(block_on(first.request(Command::Load)).is_err());
    assert!(
        block_on(first.request(Command::CreateAccount {
            name: "late write".into(),
            kind: AccountKind::Cash,
            opening: "1".into(),
            credit_cycle: None
        }))
        .is_err()
    );
    assert!(worker.ledger(&one.token).is_err());
    let second = worker.ledger(&two.token).expect("second");
    let Response::Dashboard(view) = block_on(second.request(Command::Load)).expect("load") else {
        panic!("view")
    };
    assert!(view.accounts.is_empty());
    assert!(request(&worker, ProfileCommand::Logout { token: one.token }).is_err());
    assert!(block_on(second.request(Command::Load)).is_ok());
    request(&worker, ProfileCommand::Logout { token: two.token }).expect("logout");
    assert!(block_on(second.request(Command::Load)).is_err());
    let back = authenticated(
        request(
            &worker,
            ProfileCommand::Login {
                remember: false,
                id: one.profile.id,
                password: "test password 123".into(),
            },
        )
        .expect("login"),
    );
    let Response::Dashboard(view) = block_on(
        worker
            .ledger(&back.token)
            .expect("back")
            .request(Command::Load),
    )
    .expect("load") else {
        panic!("view")
    };
    assert_eq!(view.accounts.len(), 1);
    assert_eq!(view.accounts[0].account.name().as_str(), "Alice cash");
}

#[test]
fn edits_require_current_password_and_hashes_are_salted_not_plaintext() {
    let dir = tempfile::tempdir().expect("directory");
    let worker = ProfileWorker::start(dir.path()).expect("worker");
    let alice = create(&worker, "Alice");
    let bob = create(&worker, "Bob");
    assert!(
        request(
            &worker,
            ProfileCommand::Create {
                remember: false,
                username: " alice ".into(),
                password: "another password".into()
            }
        )
        .is_err()
    );
    for (token, current) in [
        (alice.token, "test password 123"),
        (bob.token.clone(), "wrong"),
    ] {
        assert!(
            request(
                &worker,
                ProfileCommand::Edit {
                    token,
                    username: "Renamed".into(),
                    current_password: current.into(),
                    new_password: Some("new password 123".into())
                }
            )
            .is_err()
        );
    }
    let db = rusqlite::Connection::open(dir.path().join("user-profiles.sqlite3")).expect("db");
    let hashes: Vec<String> = db
        .prepare("SELECT password_hash FROM profiles")
        .expect("query")
        .query_map([], |r| r.get(0))
        .expect("rows")
        .collect::<Result<_, _>>()
        .expect("hashes");
    assert_ne!(hashes[0], hashes[1]);
    assert!(
        hashes
            .iter()
            .all(|h| h.starts_with("$argon2id$") && !h.contains("test password"))
    );
    let renamed = authenticated(
        request(
            &worker,
            ProfileCommand::Edit {
                token: bob.token,
                username: "บ๊อบ".into(),
                current_password: "test password 123".into(),
                new_password: Some("new password 123".into()),
            },
        )
        .expect("edit"),
    );
    assert_eq!(renamed.profile.id, bob.profile.id);
    assert_eq!(renamed.profile.username, "บ๊อบ");
    request(
        &worker,
        ProfileCommand::Logout {
            token: renamed.token,
        },
    )
    .expect("logout");
    assert!(
        request(
            &worker,
            ProfileCommand::Login {
                remember: false,
                id: bob.profile.id.clone(),
                password: "test password 123".into()
            }
        )
        .is_err()
    );
    assert!(
        request(
            &worker,
            ProfileCommand::Login {
                remember: false,
                id: bob.profile.id,
                password: "new password 123".into()
            }
        )
        .is_ok()
    );
}

#[test]
fn repeated_failed_passwords_are_throttled_across_restart() {
    let dir = tempfile::tempdir().expect("directory");
    let worker = ProfileWorker::start(dir.path()).expect("worker");
    let user = create(&worker, "Throttle");
    request(&worker, ProfileCommand::Logout { token: user.token }).expect("logout");
    for _ in 0..5 {
        assert!(
            request(
                &worker,
                ProfileCommand::Login {
                    remember: false,
                    id: user.profile.id.clone(),
                    password: "wrong".into()
                }
            )
            .is_err()
        );
    }
    let restarted = ProfileWorker::start(dir.path()).expect("restart");
    let error = request(
        &restarted,
        ProfileCommand::Login {
            remember: false,
            id: user.profile.id,
            password: "test password 123".into(),
        },
    )
    .expect_err("throttled");
    assert!(error.to_string().contains("30"));
}

fn remembered_user(worker: &ProfileWorker) -> ProfileSession {
    authenticated(
        request(
            worker,
            ProfileCommand::Create {
                username: "Remembered user".into(),
                password: "test password 123".into(),
                remember: true,
            },
        )
        .expect("create remembered user"),
    )
}

fn registry(dir: &std::path::Path) -> rusqlite::Connection {
    rusqlite::Connection::open(dir.join("user-profiles.sqlite3")).expect("registry")
}

fn resume(dir: &std::path::Path) -> ProfileResponse {
    let worker = ProfileWorker::start(dir).expect("restart");
    request(&worker, ProfileCommand::Resume).expect("resume")
}

#[test]
fn remembered_login_survives_restart_without_storing_password_or_extending_expiry() {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().expect("directory");
    let worker = ProfileWorker::start(dir.path()).expect("worker");
    let original = remembered_user(&worker);
    let token = std::fs::read(dir.path().join("remembered-login.token")).expect("token");
    assert_eq!(token.len(), 32);
    let db = registry(dir.path());
    let (hash, issued, expires): (Vec<u8>, i64, i64) = db
        .query_row(
            "SELECT token_hash,issued_at,expires_at FROM remembered_login",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .expect("stored login");
    assert_ne!(hash, token);
    assert_eq!(hash, Sha256::digest(&token).to_vec());
    assert_eq!(expires - issued, 7 * 24 * 60 * 60);
    for _ in 0..2 {
        let restored = authenticated(resume(dir.path()));
        assert_eq!(restored.profile, original.profile);
        assert_ne!(restored.token, original.token);
        assert_eq!(
            db.query_row("SELECT expires_at FROM remembered_login", [], |r| r
                .get::<_, i64>(0))
                .expect("deadline"),
            expires
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(dir.path().join("remembered-login.token"))
                .expect("permissions")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn opt_out_and_wrong_password_do_not_leave_a_remembered_login() {
    let dir = tempfile::tempdir().expect("directory");
    let worker = ProfileWorker::start(dir.path()).expect("worker");
    let user = create(&worker, "Manual login");
    assert_eq!(resume(dir.path()), ProfileResponse::SignedOut);
    assert!(
        request(
            &worker,
            ProfileCommand::Login {
                id: user.profile.id.clone(),
                password: "wrong password".into(),
                remember: true,
            }
        )
        .is_err()
    );
    assert_eq!(resume(dir.path()), ProfileResponse::SignedOut);
    let remembered = authenticated(
        request(
            &worker,
            ProfileCommand::Login {
                id: user.profile.id.clone(),
                password: "test password 123".into(),
                remember: true,
            },
        )
        .expect("login"),
    );
    assert_eq!(
        authenticated(resume(dir.path())).profile,
        remembered.profile
    );
    request(
        &worker,
        ProfileCommand::Login {
            id: user.profile.id,
            password: "test password 123".into(),
            remember: false,
        },
    )
    .expect("opt out");
    assert_eq!(resume(dir.path()), ProfileResponse::SignedOut);
}

#[test]
fn logout_and_password_change_revoke_remembered_tokens_even_if_the_file_is_replayed() {
    for change_password in [false, true] {
        let dir = tempfile::tempdir().expect("directory");
        let worker = ProfileWorker::start(dir.path()).expect("worker");
        let session = remembered_user(&worker);
        let file = dir.path().join("remembered-login.token");
        let old_token = std::fs::read(&file).expect("token");
        let command = if change_password {
            ProfileCommand::Edit {
                token: session.token,
                username: session.profile.username,
                current_password: "test password 123".into(),
                new_password: Some("changed password 123".into()),
            }
        } else {
            ProfileCommand::Logout {
                token: session.token,
            }
        };
        request(&worker, command).expect("revoke");
        assert!(!file.exists());
        std::fs::write(&file, old_token).expect("replayed file");
        assert_eq!(resume(dir.path()), ProfileResponse::SignedOut);
        assert!(!file.exists());
    }
}

#[test]
fn rename_keeps_remembered_identity_and_other_user_login_replaces_it() {
    let dir = tempfile::tempdir().expect("directory");
    let worker = ProfileWorker::start(dir.path()).expect("worker");
    let original = remembered_user(&worker);
    request(
        &worker,
        ProfileCommand::Edit {
            token: original.token,
            username: "New name".into(),
            current_password: "test password 123".into(),
            new_password: None,
        },
    )
    .expect("rename");
    let renamed = authenticated(resume(dir.path()));
    assert_eq!(renamed.profile.id, original.profile.id);
    assert_eq!(renamed.profile.username, "New name");
    let other = authenticated(
        request(
            &worker,
            ProfileCommand::Create {
                username: "Other".into(),
                password: "other password 123".into(),
                remember: true,
            },
        )
        .expect("other"),
    );
    assert_eq!(authenticated(resume(dir.path())).profile, other.profile);
    create(&worker, "Not remembered");
    assert_eq!(resume(dir.path()), ProfileResponse::SignedOut);
}

#[test]
fn corrupt_missing_and_invented_remembered_tokens_return_to_login_preserving_profiles() {
    for bytes in [
        None,
        Some(vec![]),
        Some(vec![7; 31]),
        Some(vec![7; 32]),
        Some(vec![7; 33]),
    ] {
        let dir = tempfile::tempdir().expect("directory");
        let worker = ProfileWorker::start(dir.path()).expect("worker");
        let user = remembered_user(&worker);
        let file = dir.path().join("remembered-login.token");
        match bytes {
            Some(bytes) => std::fs::write(&file, bytes).expect("corrupt"),
            None => std::fs::remove_file(&file).expect("remove"),
        }
        assert_eq!(resume(dir.path()), ProfileResponse::SignedOut);
        assert_eq!(users(&worker), vec![user.profile]);
        assert_eq!(
            registry(dir.path())
                .query_row("SELECT COUNT(*) FROM remembered_login", [], |r| r
                    .get::<_, i64>(0))
                .expect("count"),
            0
        );
    }
}

#[test]
fn expired_extended_or_clock_rollback_remembered_login_is_rejected() {
    for sql in [
        "UPDATE remembered_login SET issued_at=issued_at-604801,expires_at=expires_at-604801,last_used_at=last_used_at-604801",
        "UPDATE remembered_login SET expires_at=expires_at+604800",
        "UPDATE remembered_login SET last_used_at=last_used_at+3600",
    ] {
        let dir = tempfile::tempdir().expect("directory");
        let worker = ProfileWorker::start(dir.path()).expect("worker");
        let user = remembered_user(&worker);
        registry(dir.path()).execute(sql, []).expect("alter time");
        assert_eq!(resume(dir.path()), ProfileResponse::SignedOut);
        assert_eq!(users(&worker), vec![user.profile]);
    }
}

#[test]
fn registry_v1_upgrade_keeps_identity_and_does_not_auto_login_without_consent() {
    let dir = tempfile::tempdir().expect("directory");
    let worker = ProfileWorker::start(dir.path()).expect("worker");
    let user = create(&worker, "Existing user");
    let db = registry(dir.path());
    db.execute_batch("DROP TABLE remembered_login; PRAGMA user_version=1;")
        .expect("v1 fixture");
    assert_eq!(resume(dir.path()), ProfileResponse::SignedOut);
    let restarted = ProfileWorker::start(dir.path()).expect("restart");
    assert_eq!(users(&restarted), vec![user.profile.clone()]);
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .expect("version"),
        2
    );
    assert!(
        request(
            &restarted,
            ProfileCommand::Login {
                id: user.profile.id,
                password: "test password 123".into(),
                remember: false
            }
        )
        .is_ok()
    );
}

#[test]
fn claimed_legacy_user_can_opt_in_without_moving_the_existing_ledger() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("ledger.sqlite3");
    drop(SqliteLedger::open(&path).expect("legacy ledger"));
    let worker = ProfileWorker::start(dir.path()).expect("worker");
    let user = users(&worker).remove(0);
    let claimed = authenticated(
        request(
            &worker,
            ProfileCommand::ClaimLegacy {
                id: user.id,
                username: "Legacy".into(),
                password: "legacy password 123".into(),
                remember: true,
            },
        )
        .expect("claim"),
    );
    assert_eq!(authenticated(resume(dir.path())).profile, claimed.profile);
    assert!(path.exists());
}

#[test]
fn credential_storage_failure_does_not_switch_the_active_user() {
    let dir = tempfile::tempdir().expect("directory");
    let worker = ProfileWorker::start(dir.path()).expect("worker");
    let first = create(&worker, "First");
    let second = create(&worker, "Second");
    // A directory cannot be replaced by the credential file.
    std::fs::create_dir(dir.path().join("remembered-login.token")).expect("block file");
    assert!(
        request(
            &worker,
            ProfileCommand::Login {
                id: first.profile.id,
                password: "test password 123".into(),
                remember: true,
            }
        )
        .is_err()
    );
    assert!(worker.ledger(&first.token).is_err());
    assert!(
        block_on(
            worker
                .ledger(&second.token)
                .expect("same user")
                .request(Command::Load)
        )
        .is_ok()
    );
    assert_eq!(
        registry(dir.path())
            .query_row("SELECT COUNT(*) FROM remembered_login", [], |r| r
                .get::<_, i64>(0))
            .expect("no grant"),
        0
    );
}
