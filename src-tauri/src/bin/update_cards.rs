use std::path::PathBuf;

use thejudgeapp_lib::db::Database;
use thejudgeapp_lib::sync::cards_updater::{
    load_oracle_cards_from_path, load_rulings_from_path, save_oracle_cards_with_progress,
    save_rulings_with_progress, CardsUpdateError,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db_path: Option<PathBuf> = None;
    let mut json_path: Option<PathBuf> = None;
    let mut rulings_path: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => {
                db_path = Some(PathBuf::from(args.next().unwrap_or_else(|| {
                    eprintln!("--db requires a value");
                    std::process::exit(1);
                })));
            }
            "--rulings" => {
                rulings_path = Some(PathBuf::from(args.next().unwrap_or_else(|| {
                    eprintln!("--rulings requires a value");
                    std::process::exit(1);
                })));
            }
            _ if arg.starts_with("--db=") => {
                db_path = Some(PathBuf::from(arg.trim_start_matches("--db=")));
            }
            _ if arg.starts_with("--rulings=") => {
                rulings_path = Some(PathBuf::from(arg.trim_start_matches("--rulings=")));
            }
            _ if arg.starts_with("--") => {
                eprintln!("Unknown flag: {arg}");
                std::process::exit(1);
            }
            _ => {
                json_path = Some(PathBuf::from(arg));
                break;
            }
        }
    }

    let cards_path = match json_path {
        Some(p) => p,
        None => {
            eprintln!("Usage: update_cards [--db <db>] [--rulings <rulings.json>] <oracle-cards.json>");
            std::process::exit(1);
        }
    };

    let mut db = if let Some(path) = db_path {
        eprintln!("Using DB: {}", path.display());
        Database::open_or_create_at(&path)?
    } else {
        eprintln!("Using DB: {}", Database::db_path().display());
        Database::open_or_create()?
    };

    let cards = load_oracle_cards_from_path(&cards_path).map_err(map_cards_error)?;
    let total = cards.len();
    eprintln!("Loaded {} cards", total);
    if total > 0 {
        let mut last_report = 0usize;
        save_oracle_cards_with_progress(db.conn_mut(), &cards, |count| {
            if count.saturating_sub(last_report) >= 1000 {
                let pct = (count as f64 / total as f64) * 100.0;
                let filled = (pct / 5.0) as usize;
                let bar = format!("{}{}", "#".repeat(filled), "-".repeat(20 - filled));
                eprint!("\rImporting cards  [{bar}] {count}/{total} ({pct:.1}%)   ");
                last_report = count;
            }
        })
        .map_err(map_cards_error)?;
        eprintln!("\rImported {total} cards.                                    ");
    }

    if let Some(path) = rulings_path {
        let rulings = load_rulings_from_path(&path).map_err(map_cards_error)?;
        let total_r = rulings.len();
        eprintln!("Loaded {} rulings", total_r);
        let mut last_report = 0usize;
        let inserted = save_rulings_with_progress(db.conn_mut(), &rulings, |count| {
            if count >= total_r || count.saturating_sub(last_report) >= 5000 {
                let pct = (count as f64 / total_r as f64) * 100.0;
                let filled = (pct / 5.0) as usize;
                let bar = format!("{}{}", "#".repeat(filled), "-".repeat(20 - filled));
                eprint!("\rImporting rulings [{bar}] {count}/{total_r} ({pct:.1}%)   ");
                last_report = count;
            }
        })
        .map_err(map_cards_error)?;
        eprintln!("\rInserted {inserted} of {total_r} rulings matched to cards.     ");
    }

    println!("Done.");
    Ok(())
}

fn map_cards_error(err: CardsUpdateError) -> Box<dyn std::error::Error> {
    match err {
        CardsUpdateError::Io(e) => Box::new(e),
        CardsUpdateError::Json(e) => Box::new(e),
        CardsUpdateError::Db(e) => Box::new(e),
    }
}
