/// update_ipg — download and import the Magic Infraction Procedure Guide into judge.db
///
/// Usage:
///   cargo run --bin update_ipg                          # uses default DB path + latest IPG URL
///   cargo run --bin update_ipg -- --db path/to/judge.db
///   cargo run --bin update_ipg -- --file path/to/ipg.pdf
///   cargo run --bin update_ipg -- --url https://... --db path/to/judge.db

use pdf_extract::extract_text_from_mem;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use thejudgeapp_lib::parser::ipg_parser::parse_ipg;

const IPG_URL: &str =
    "https://media.wizards.com/ContentResources/WPN/MTG_IPG_2024Sep23_EN.pdf";

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut db_path: Option<PathBuf> = None;
    let mut file_path: Option<PathBuf> = None;
    let mut url: Option<String> = None;
    let mut dump_path: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--db" => {
                i += 1;
                db_path = Some(PathBuf::from(&args[i]));
            }
            "--file" => {
                i += 1;
                file_path = Some(PathBuf::from(&args[i]));
            }
            "--url" => {
                i += 1;
                url = Some(args[i].clone());
            }
            "--dump" => {
                i += 1;
                dump_path = Some(PathBuf::from(&args[i]));
            }
            _ => {}
        }
        i += 1;
    }

    // Resolve DB path
    let db_path = db_path.unwrap_or_else(default_db_path);
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).expect("Could not create DB directory");
    }
    println!("Database: {}", db_path.display());

    // Get the IPG PDF bytes
    let pdf_bytes = if let Some(path) = file_path {
        println!("Reading from file: {}", path.display());
        std::fs::read(&path).expect("Could not read IPG file")
    } else {
        let target_url = url.as_deref().unwrap_or(IPG_URL);
        println!("Downloading IPG from: {}", target_url);
        download_bytes(target_url)
    };

    println!("Extracting text from PDF...");
    let text = extract_text_from_mem(&pdf_bytes).expect("PDF text extraction failed");

    // --dump path/to/out.txt  →  write raw extracted text and exit (for debugging)
    if let Some(path) = dump_path {
        std::fs::write(&path, &text).expect("Could not write dump file");
        println!("Dumped {} bytes of extracted text to {}", text.len(), path.display());
        return;
    }

    println!("Parsing...");
    let parsed = parse_ipg(&text);

    println!(
        "Parsed: {} rules (version: {})",
        parsed.rules.len(),
        parsed.version
    );

    println!("Importing into database...");
    let mut conn = Connection::open(&db_path).expect("Could not open database");
    run_schema(&conn);
    let doc_id = import(&mut conn, &parsed);

    println!(
        "Done. Inserted {} rules (doc_id={}).",
        parsed.rules.len(),
        doc_id
    );
}

fn download_bytes(url: &str) -> Vec<u8> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("thejudgeapp/0.1 update_ipg")
        .build()
        .expect("Could not build HTTP client");

    let response = client.get(url).send().expect("HTTP request failed");
    if !response.status().is_success() {
        panic!("HTTP error: {}", response.status());
    }
    response.bytes().expect("Could not read response bytes").to_vec()
}

fn run_schema(conn: &Connection) {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS documents (
            id         INTEGER PRIMARY KEY,
            doc_type   TEXT NOT NULL,
            version    TEXT NOT NULL,
            fetched_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS rules (
            id         INTEGER PRIMARY KEY,
            doc_id     INTEGER NOT NULL REFERENCES documents(id),
            number     TEXT NOT NULL,
            title      TEXT,
            body       TEXT NOT NULL,
            body_html  TEXT NOT NULL,
            parent     TEXT,
            sort_order INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_rules_number ON rules(number);
        CREATE INDEX IF NOT EXISTS idx_rules_doc_id ON rules(doc_id);

        CREATE VIRTUAL TABLE IF NOT EXISTS rules_fts USING fts5(
            number, title, body, content='rules', content_rowid='id'
        );

        CREATE TABLE IF NOT EXISTS glossary (
            id         INTEGER PRIMARY KEY,
            doc_id     INTEGER NOT NULL REFERENCES documents(id),
            term       TEXT NOT NULL,
            definition TEXT NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS glossary_fts USING fts5(
            term, definition, content='glossary', content_rowid='id'
        );
        ",
    )
    .expect("Schema creation failed");
}

fn import(
    conn: &mut Connection,
    parsed: &thejudgeapp_lib::parser::ipg_parser::ParsedIPG,
) -> i64 {
    let tx = conn.transaction().expect("Could not start transaction");

    // Remove any existing IPG document
    tx.execute(
        "DELETE FROM rules WHERE doc_id IN (SELECT id FROM documents WHERE doc_type='ipg')",
        [],
    )
    .unwrap();
    tx.execute(
        "DELETE FROM documents WHERE doc_type='ipg'",
        [],
    )
    .unwrap();

    // Insert document record
    tx.execute(
        "INSERT INTO documents (doc_type, version) VALUES ('ipg', ?1)",
        params![parsed.version],
    )
    .expect("Insert document failed");
    let doc_id = tx.last_insert_rowid();

    // Insert rules
    {
        let mut rule_stmt = tx
            .prepare(
                "INSERT INTO rules (doc_id, number, title, body, body_html, parent, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .expect("Prepare rule insert failed");

        for (i, rule) in parsed.rules.iter().enumerate() {
            rule_stmt
                .execute(params![
                    doc_id,
                    rule.number,
                    rule.title,
                    rule.body,
                    rule.body_html,
                    rule.parent,
                    i as i64
                ])
                .expect("Rule insert failed");
        }
    }

    // Rebuild FTS index
    tx.execute_batch("INSERT INTO rules_fts(rules_fts) VALUES('rebuild');")
        .expect("FTS rebuild failed");

    tx.commit().expect("Transaction commit failed");
    doc_id
}

fn default_db_path() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join("thejudgeapp").join("judge.db")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("thejudgeapp")
            .join("judge.db")
    } else {
        PathBuf::from("judge.db")
    }
}
