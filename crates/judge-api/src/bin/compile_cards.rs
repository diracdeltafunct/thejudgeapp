/// Reads a Scryfall all-cards JSON dump and produces a compact JSON file with one
/// record per unique oracle card, including a list of all sets it has been printed in.
///
/// Usage: compile_cards <input.json> <output.json>
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};
use serde::{Deserialize as DerivDeserialize, Serialize};

// ── Scryfall input types ──────────────────────────────────────────────────────

#[derive(DerivDeserialize)]
struct ScryfallCard {
    oracle_id: Option<String>,
    name: String,
    lang: String,
    layout: String,
    released_at: String,
    oracle_text: Option<String>,
    mana_cost: Option<String>,
    cmc: Option<f64>,
    type_line: Option<String>,
    colors: Option<Vec<String>>,
    legalities: Option<BTreeMap<String, String>>,
    image_uris: Option<ImageUris>,
    card_faces: Option<Vec<CardFace>>,
    set: String,
    set_name: String,
}

#[derive(DerivDeserialize)]
struct CardFace {
    oracle_text: Option<String>,
    mana_cost: Option<String>,
    type_line: Option<String>,
    image_uris: Option<ImageUris>,
}

#[derive(DerivDeserialize)]
struct ImageUris {
    normal: Option<String>,
}

// ── Output types ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct CompactCard {
    oracle_id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    oracle_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mana_cost: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cmc: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    type_line: Option<String>,
    colors: Vec<String>,
    legalities: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    back_image_url: Option<String>,
    printings: Vec<Printing>,
}

#[derive(Serialize, Clone)]
struct Printing {
    set_code: String,
    set_name: String,
    released_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    back_image_url: Option<String>,
}

// ── Layouts to exclude (non-playable / non-oracle objects) ────────────────────

const EXCLUDED_LAYOUTS: &[&str] = &[
    "token",
    "double_faced_token",
    "art_series",
    "emblem",
    "vanguard",
    "scheme",
    "phenomenon",
    "plane",
];

// ── Streaming accumulator ─────────────────────────────────────────────────────

struct Accumulator {
    // oracle_id -> (canonical CompactCard, map of set_code -> Printing, newest released_at seen)
    groups: HashMap<String, (CompactCard, HashMap<String, Printing>, String)>,
    total: usize,
    skipped: usize,
}

impl Accumulator {
    fn new() -> Self {
        Self {
            groups: HashMap::new(),
            total: 0,
            skipped: 0,
        }
    }

    fn process(&mut self, card: ScryfallCard) {
        self.total += 1;
        if self.total % 100_000 == 0 {
            eprintln!("  {} cards processed...", self.total);
        }

        if card.lang != "en" || EXCLUDED_LAYOUTS.contains(&card.layout.as_str()) {
            self.skipped += 1;
            return;
        }
        let Some(oracle_id) = card.oracle_id else {
            self.skipped += 1;
            return;
        };

        let faces = card.card_faces.unwrap_or_default();

        let image_url = card
            .image_uris
            .and_then(|u| u.normal)
            .or_else(|| {
                faces.iter()
                    .find_map(|f| f.image_uris.as_ref().and_then(|u| u.normal.clone()))
            });

        // Second face image (only present for DFCs)
        let back_image_url = if faces.len() >= 2 {
            faces.get(1).and_then(|f| f.image_uris.as_ref()?.normal.clone())
        } else {
            None
        };

        // DFCs have no top-level oracle_text/mana_cost/type_line — pull from faces
        let oracle_text = card.oracle_text.or_else(|| {
            let texts: Vec<&str> = faces.iter()
                .filter_map(|f| f.oracle_text.as_deref())
                .collect();
            if texts.is_empty() { None } else { Some(texts.join("\n//\n")) }
        });
        let mana_cost = card.mana_cost.or_else(|| {
            faces.iter().find_map(|f| f.mana_cost.clone())
        });
        let type_line = card.type_line.or_else(|| {
            let types: Vec<&str> = faces.iter()
                .filter_map(|f| f.type_line.as_deref())
                .collect();
            if types.is_empty() { None } else { Some(types.join(" // ")) }
        });

        let printing = Printing {
            set_code: card.set.clone(),
            set_name: card.set_name,
            released_at: card.released_at.clone(),
            image_url: image_url.clone(),
            back_image_url: back_image_url.clone(),
        };

        match self.groups.entry(oracle_id.clone()) {
            std::collections::hash_map::Entry::Vacant(e) => {
                let compact = CompactCard {
                    oracle_id,
                    name: card.name,
                    oracle_text,
                    mana_cost,
                    cmc: card.cmc,
                    type_line,
                    colors: card.colors.unwrap_or_default(),
                    legalities: card.legalities.unwrap_or_default(),
                    image_url,
                    back_image_url,
                    printings: Vec::new(),
                };
                let newest = card.released_at;
                let mut map = HashMap::new();
                map.insert(card.set, printing);
                e.insert((compact, map, newest));
            }
            std::collections::hash_map::Entry::Occupied(mut e) => {
                let (existing, printings, newest) = e.get_mut();
                // Update canonical image to the most recent printing across all sets
                if card.released_at > *newest {
                    *newest = card.released_at.clone();
                    if let Some(url) = image_url.clone() {
                        existing.image_url = Some(url);
                    }
                    existing.back_image_url = back_image_url.clone();
                }
                // Keep the newest printing per set_code so its image_url stays in sync
                match printings.entry(card.set) {
                    std::collections::hash_map::Entry::Vacant(e) => {
                        e.insert(printing);
                    }
                    std::collections::hash_map::Entry::Occupied(mut e) => {
                        if printing.released_at > e.get().released_at {
                            *e.get_mut() = printing;
                        }
                    }
                }
            }
        }
    }

    fn finish(self) -> Vec<CompactCard> {
        let mut output: Vec<CompactCard> = self
            .groups
            .into_values()
            .map(|(mut card, printings_map, _)| {
                let mut printings: Vec<Printing> = printings_map.into_values().collect();
                printings.sort_by(|a, b| a.released_at.cmp(&b.released_at));
                card.printings = printings;
                card
            })
            .collect();
        output.sort_by(|a, b| a.name.cmp(&b.name));
        output
    }
}

// ── Streaming deserializer ────────────────────────────────────────────────────

/// Wraps the Accumulator so serde can drive it as a sequence visitor,
/// processing one card at a time without loading the full array into memory.
struct StreamingAccumulator(Accumulator);

impl<'de> Deserialize<'de> for StreamingAccumulator {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct AccVisitor;

        impl<'de> Visitor<'de> for AccVisitor {
            type Value = StreamingAccumulator;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "an array of Scryfall card objects")
            }

            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut acc = Accumulator::new();
                while let Some(card) = seq.next_element::<ScryfallCard>()? {
                    acc.process(card);
                }
                Ok(StreamingAccumulator(acc))
            }
        }

        de.deserialize_seq(AccVisitor)
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: compile_cards <input.json> <output.json>");
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    eprintln!("Opening {input_path}...");
    let file = File::open(input_path).expect("open input file");
    let reader = BufReader::with_capacity(8 * 1024 * 1024, file);

    eprintln!("Streaming and compiling cards...");
    let mut de = serde_json::Deserializer::from_reader(reader);
    let StreamingAccumulator(acc) =
        StreamingAccumulator::deserialize(&mut de).expect("parse JSON");

    eprintln!(
        "Done: {} total printings, {} skipped, {} unique oracle cards",
        acc.total,
        acc.skipped,
        acc.groups.len()
    );

    let output = acc.finish();

    eprintln!("Writing {output_path}...");
    let out_file = File::create(output_path).expect("create output file");
    let mut writer = BufWriter::new(out_file);
    serde_json::to_writer(&mut writer, &output).expect("serialize output");
    writer.flush().expect("flush output");

    eprintln!("Wrote {} cards to {output_path}", output.len());
}
