pub mod cards_repo;
pub mod migrations;
pub mod rules_repo;

use crate::models::card::CardResult;
use crate::models::rule::{GlossaryEntry, RuleDetail, RuleResult, TocEntry};
use rusqlite::Connection;
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open_or_create() -> Result<Self, rusqlite::Error> {
        let path = Self::db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(&path)?;
        let db = Database { conn };
        db.run_migrations()?;
        Ok(db)
    }

    fn db_path() -> PathBuf {
        // Use platform-appropriate data directory
        if let Some(data_dir) = dirs_next() {
            data_dir.join("thejudgeapp").join("judge.db")
        } else {
            PathBuf::from("judge.db")
        }
    }

    fn run_migrations(&self) -> Result<(), rusqlite::Error> {
        migrations::run(&self.conn)
    }

    pub fn search_rules(
        &self,
        query: &str,
        doc_type: Option<&str>,
    ) -> Result<Vec<RuleResult>, rusqlite::Error> {
        rules_repo::search_rules(&self.conn, query, doc_type)
    }

    pub fn get_rule(&self, number: &str) -> Result<RuleDetail, rusqlite::Error> {
        rules_repo::get_rule(&self.conn, number)
    }

    pub fn get_toc(&self) -> Result<Vec<TocEntry>, rusqlite::Error> {
        rules_repo::get_toc(&self.conn)
    }

    pub fn get_rule_section(
        &self,
        prefix: &str,
        doc_type: &str,
    ) -> Result<Vec<RuleDetail>, rusqlite::Error> {
        rules_repo::get_rule_section(&self.conn, prefix, doc_type)
    }

    pub fn get_glossary_term(&self, term: &str) -> Result<GlossaryEntry, rusqlite::Error> {
        rules_repo::get_glossary_term(&self.conn, term)
    }

    pub fn get_rules_doc(&self, doc_type: &str) -> Result<Vec<RuleDetail>, rusqlite::Error> {
        rules_repo::get_rules_doc(&self.conn, doc_type)
    }

    pub fn search_cards(&self, query: &str) -> Result<Vec<CardResult>, rusqlite::Error> {
        cards_repo::search_cards(&self.conn, query)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

fn dirs_next() -> Option<PathBuf> {
    #[cfg(target_os = "android")]
    {
        // On Android, use the app's internal storage
        std::env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| Some(PathBuf::from("/data/data/com.thejudgeapp.app")))
    }
    #[cfg(not(target_os = "android"))]
    {
        std::env::var("APPDATA")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".local/share"))
            })
    }
}
