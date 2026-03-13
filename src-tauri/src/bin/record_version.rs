/// Records a document version in the database without re-importing content.
/// Useful for one-off fixes when cards/rules were imported before version tracking existed.
///
/// Usage:
///   cargo run --bin record_version -- --doc-type cards --version 20260312
///   cargo run --bin record_version -- --db fresh_judge.db --doc-type cards --version 20260312
use std::path::PathBuf;
use thejudgeapp_lib::db::Database;
use thejudgeapp_lib::sync::cards_updater::record_cards_version;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db_path: Option<PathBuf> = None;
    let mut doc_type: Option<String> = None;
    let mut version: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => db_path = Some(PathBuf::from(args.next().expect("--db requires a value"))),
            "--doc-type" => doc_type = Some(args.next().expect("--doc-type requires a value")),
            "--version" => version = Some(args.next().expect("--version requires a value")),
            _ if arg.starts_with("--db=") => db_path = Some(PathBuf::from(arg.trim_start_matches("--db="))),
            _ if arg.starts_with("--doc-type=") => doc_type = Some(arg.trim_start_matches("--doc-type=").to_string()),
            _ if arg.starts_with("--version=") => version = Some(arg.trim_start_matches("--version=").to_string()),
            _ => { eprintln!("Unknown argument: {arg}"); std::process::exit(1); }
        }
    }

    let doc_type = doc_type.unwrap_or_else(|| { eprintln!("--doc-type is required"); std::process::exit(1); });
    let version = version.unwrap_or_else(|| { eprintln!("--version is required"); std::process::exit(1); });

    let mut db = match db_path {
        Some(path) => { eprintln!("Using DB: {}", path.display()); Database::open_or_create_at(&path)? }
        None => { eprintln!("Using DB: {}", Database::db_path().display()); Database::open_or_create()? }
    };

    if doc_type == "cards" {
        record_cards_version(db.conn_mut(), &version)
            .map_err(|e| format!("{e:?}"))?;
    } else {
        eprintln!("Only 'cards' doc-type is supported by this tool (rules docs have their own import scripts)");
        std::process::exit(1);
    }

    eprintln!("Recorded {doc_type} version: {version}");
    Ok(())
}
