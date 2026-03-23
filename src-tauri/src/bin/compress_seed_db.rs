/// Compresses fresh_judge.db → fresh_judge.db.zst for bundling with the installer.
/// Run from the workspace root: cargo run --bin compress_seed_db
use std::fs::File;
use std::path::Path;

fn main() {
    let input_path = Path::new("resources/fresh_judge.db");
    let output_path = Path::new("resources/fresh_judge.db.zst");

    let input = File::open(input_path)
        .unwrap_or_else(|_| panic!("Cannot open {:?} — run from the workspace root", input_path));
    let output = File::create(output_path)
        .unwrap_or_else(|_| panic!("Cannot create {:?}", output_path));

    let input_size = input.metadata().map(|m| m.len()).unwrap_or(0);

    // Level 19 gives good compression on SQLite; level 22 (max) saves a few more MB
    // but takes noticeably longer. 19 is a good balance.
    zstd::stream::copy_encode(input, output, 19).expect("Compression failed");

    let output_size = std::fs::metadata(output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    println!(
        "Compressed fresh_judge.db → fresh_judge.db.zst  ({} MB → {} MB, {:.0}% of original)",
        input_size / 1_048_576,
        output_size / 1_048_576,
        output_size as f64 / input_size as f64 * 100.0
    );
}
