//! Refresh the seed database from the repository's Riftbound rules and cards.
//! Usage: cargo run --bin update_riftbound -- --db scripts/resources/fresh_judge.db
use std::{fs::File, path::PathBuf};
use thejudgeapp_lib::{
    db::Database,
    sync::{
        riftbound_cards_updater,
        riftbound_importer::{self, RiftboundSection},
    },
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("--db") {
        return Err("Usage: update_riftbound --db <database path>".into());
    }
    let path = PathBuf::from(args.next().ok_or("--db requires a path")?);
    let manifest: serde_json::Value = serde_json::from_reader(File::open("data-manifest.json")?)?;
    // Validate all JSON before touching the database.
    let mut documents = Vec::new();
    for kind in ["cr", "tr", "ep"] {
        let doc_type = format!("riftbound_{kind}");
        let sections: Vec<RiftboundSection> =
            serde_json::from_reader(File::open(format!("data/{doc_type}.json"))?)?;
        if sections.is_empty() {
            return Err(format!("{doc_type} is empty").into());
        }
        let version = manifest[&doc_type]["version"]
            .as_str()
            .ok_or("Missing rules version")?
            .to_string();
        documents.push((doc_type, version, sections));
    }
    let cards = riftbound_cards_updater::load_riftbound_cards_from_path(std::path::Path::new(
        "data/riftbound_cards.json",
    ))
    .map_err(|error| error.to_string())?;
    if cards.is_empty() {
        return Err("Card dataset is empty".into());
    }
    if !path.exists() {
        let compressed = File::open("scripts/resources/fresh_judge.db.zst")?;
        let mut output = File::create(&path)?;
        zstd::stream::copy_decode(compressed, &mut output)?;
    }
    let mut db = Database::open_or_create_at(&path)?;
    for (doc_type, version, sections) in documents {
        riftbound_importer::reimport(db.conn_mut(), &doc_type, &version, &sections)?;
        println!("Imported {doc_type}: {version}");
    }
    riftbound_cards_updater::save_riftbound_cards_with_progress(db.conn_mut(), &cards, |_| {})
        .map_err(|error| error.to_string())?;
    riftbound_cards_updater::record_riftbound_cards_version(db.conn_mut(), "20261009")
        .map_err(|error| error.to_string())?;
    db.conn()
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
    println!(
        "Imported {} Riftbound cards into {}",
        cards.len(),
        path.display()
    );
    Ok(())
}
