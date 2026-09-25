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
            id: user.profile.id,
            password: "test password 123".into(),
        },
    )
    .expect_err("throttled");
    assert!(error.to_string().contains("30"));
}
