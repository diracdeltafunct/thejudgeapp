use crate::models::rule::RuleDetail;
use regex::Regex;
use std::collections::HashSet;

pub struct ParsedMTR {
    pub version: String,
    pub rules: Vec<RuleDetail>,
}

/// Reject obviously incomplete or structurally corrupted MTR parses before they
/// can replace a known-good document in the database. PDF text layout changes
/// are common, so parsing success alone is not sufficient validation.
pub fn validate_mtr(parsed: &ParsedMTR) -> Result<(), String> {
    if parsed.version == "unknown" {
        return Err("the document effective date could not be read".to_string());
    }

    let appendix_e = parsed
        .rules
        .iter()
        .find(|rule| rule.number == "Appendix E")
        .ok_or_else(|| "Appendix E is missing".to_string())?;

    let row_count = appendix_e
        .body_html
        .matches("<tr>")
        .count()
        .saturating_sub(1);
    if !appendix_e.body_html.contains("<table") || row_count < 3 {
        return Err(format!(
            "Appendix E's recommended Swiss rounds table was not parsed correctly (found {row_count} data rows)"
        ));
    }

    if appendix_e.body_html.contains("Appendix F") {
        return Err("Appendix F content leaked into Appendix E".to_string());
    }

    Ok(())
}

pub fn parse_mtr(raw: &str) -> ParsedMTR {
    // Normalize line endings
    let text = raw.replace("\r\n", "\n").replace('\r', "\n");
    // PDF extraction sometimes splits "Upgrade" / "Downgrade" labels from their
    // colon onto the next line ("Upgrade\n: "). Rejoin so they land in the same
    // paragraph as "Upgrade: " rather than two separate paragraphs.
    let re_join_upgrade = Regex::new(r"(?m)^(Upgrade|Downgrade)\n: ?").unwrap();
    let text = re_join_upgrade.replace_all(&text, "$1: ").into_owned();

    // Section header: "1. Tournament Basics" — tested against the RAW line (not trimmed)
    // so that indented list items like "  1. Each player..." don't match.
    let re_section = Regex::new(r"^(\d+)\.\s+(.+)$").unwrap();
    // Subsection header: "1.1 Tournament Terminology" or "1.1.2 Something"
    let re_subsection = Regex::new(r"^(\d+\.\d+(?:\.\d+)*)\s+(.+)$").unwrap();
    // Bare integer line (page numbers from PDF extraction)
    let re_only_digits = Regex::new(r"^\d+$").unwrap();
    // Version date
    let re_version =
        Regex::new(r"(?i)effective\s+(?:as\s+of\s+)?([A-Za-z]+\s+\d+,?\s+\d{4})").unwrap();
    // Appendix header: "Appendix A—Title" or "Appendix A — Title"
    let re_appendix = Regex::new(r"^(Appendix\s+[A-Z])\s*\u{2014}\s*(.+)$").unwrap();
    // Cross-references to other MTR sections
    let re_xref = Regex::new(r"\bsection\s+(\d+(?:\.\d+)*)").unwrap();

    let mut version = String::from("unknown");
    let mut rules: Vec<RuleDetail> = Vec::new();
    let mut sort_order: i64 = 0;

    // We skip everything until we see the first "real" section header.
    let mut past_toc = false;

    // Monotonic section counter: section N can only appear after section N-1.
    // This prevents numbered list items ("3. Each player...") from being
    // mistaken for top-level section headers when they happen to match the
    // next expected section number.
    let mut last_section_num: u32 = 0;

    // Track seen subsection numbers to avoid treating duplicate-looking
    // lines as new subsections.
    let mut seen_subsections: HashSet<String> = HashSet::new();

    // Buffer for accumulating lines of a paragraph before flushing.
    let mut para_buf = String::new();

    // Appendix E gets special treatment: collect raw lines, build a table at the end.
    let mut in_appendix_e = false;
    let mut appendix_e_lines: Vec<String> = Vec::new();

    macro_rules! flush_para {
        () => {
            if !para_buf.is_empty() {
                append_paragraph(&para_buf, &mut rules, &re_xref);
                para_buf.clear();
            }
        };
    }

    macro_rules! finalize_appendix_e {
        () => {
            if in_appendix_e {
                if let Some(rule) = rules.iter_mut().find(|r| r.number == "Appendix E") {
                    rule.body_html = build_rounds_table_html(&appendix_e_lines);
                }
                appendix_e_lines.clear();
            }
        };
    }

    for line in text.lines() {
        let trimmed = line.trim();

        // Skip bare page numbers
        if re_only_digits.is_match(trimmed) {
            continue;
        }

        // Skip repeated page headers / footers
        if is_header_footer(trimmed) {
            continue;
        }

        // Version detection — only the first match is the document version
        if version == "unknown" {
            if let Some(caps) = re_version.captures(trimmed) {
                version = caps[1].to_string();
            }
        }

        if !past_toc {
            if let Some(caps) = re_section.captures(trimmed) {
                let title_part = caps[2].trim();
                if !title_part
                    .chars()
                    .last()
                    .map_or(false, |c| c.is_ascii_digit())
                    && looks_like_section_title(title_part)
                {
                    past_toc = true;
                    let number = caps[1].to_string();
                    last_section_num = number.parse().unwrap_or(0);
                    let title = clean_title(title_part);
                    sort_order += 1;
                    rules.push(RuleDetail {
                        id: sort_order,
                        number: number.clone(),
                        title: Some(title.clone()),
                        body: title,
                        body_html: String::new(),
                        parent: None,
                    });
                }
            }
            continue;
        }

        if trimmed.is_empty() {
            if in_appendix_e {
                appendix_e_lines.push(String::new()); // paragraph separator
            } else {
                flush_para!();
            }
            continue;
        }

        // New appendix heading — finalize Appendix E if we were in it, then start the new one.
        if let Some(caps) = re_appendix.captures(trimmed) {
            flush_para!();
            finalize_appendix_e!();
            in_appendix_e = false;
            let letter = caps[1]
                .trim()
                .chars()
                .last()
                .unwrap_or('A')
                .to_ascii_uppercase();
            let number = format!("Appendix {}", letter);
            let title = clean_title(caps[2].trim());
            sort_order += 1;
            rules.push(RuleDetail {
                id: sort_order,
                number: number.clone(),
                title: Some(title.clone()),
                body: title,
                body_html: String::new(),
                parent: None,
            });
            if letter == 'E' {
                in_appendix_e = true;
            }
            continue;
        }

        // When inside Appendix E, collect every non-empty line individually.
        if in_appendix_e {
            appendix_e_lines.push(trimmed.to_owned());
            continue;
        }

        let is_section = if let Some(caps) = re_section.captures(trimmed) {
            let n: u32 = caps[1].parse().unwrap_or(0);
            let title = caps[2].trim();
            n == last_section_num + 1 && looks_like_section_title(title)
        } else {
            false
        };

        if is_section {
            flush_para!();
            let caps = re_section.captures(trimmed).unwrap();
            let number = caps[1].to_string();
            last_section_num = number.parse().unwrap_or(last_section_num);
            let title = clean_title(caps[2].trim());
            sort_order += 1;
            rules.push(RuleDetail {
                id: sort_order,
                number: number.clone(),
                title: Some(title.clone()),
                body: title,
                body_html: String::new(),
                parent: None,
            });
        } else if let Some(caps) = re_subsection.captures(trimmed) {
            let number = caps[1].to_string();
            if !seen_subsections.contains(&number) {
                flush_para!();
                seen_subsections.insert(number.clone());
                let title = clean_title(caps[2].trim());
                let parent = parent_of(&number);
                sort_order += 1;
                rules.push(RuleDetail {
                    id: sort_order,
                    number: number.clone(),
                    title: Some(title.clone()),
                    body: title,
                    body_html: String::new(),
                    parent,
                });
            } else {
                if starts_list_item(trimmed) {
                    flush_para!();
                } else if !para_buf.is_empty() {
                    para_buf.push(' ');
                }
                para_buf.push_str(trimmed);
            }
        } else {
            if starts_list_item(trimmed) {
                flush_para!();
            } else if !para_buf.is_empty() {
                para_buf.push(' ');
            }
            para_buf.push_str(trimmed);
        }
    }

    flush_para!();
    finalize_appendix_e!();

    ParsedMTR { version, rules }
}

// ── Appendix E table builder ──────────────────────────────────────────────────

fn build_rounds_table_html(lines: &[String]) -> String {
    // PDF may output column headers as a single merged line or individually — skip all forms.
    const SKIP: &[&str] = &[
        "Players (Teams) Swiss Rounds Playoff",
        "Players (Teams)",
        "Swiss Rounds",
        "Playoff",
        "Players",
        "Teams",
    ];

    let mut blocks: Vec<Vec<String>> = vec![vec![]];
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !blocks.last().is_none_or(Vec::is_empty) {
                blocks.push(vec![]);
            }
        } else {
            blocks.last_mut().unwrap().push(trimmed.to_string());
        }
    }

    let mut pre_paras: Vec<String> = Vec::new();
    let mut table_rows: Vec<[String; 3]> = Vec::new();
    let mut post_paras: Vec<String> = Vec::new();
    let mut table_started = false;

    for block in blocks.into_iter().filter(|block| !block.is_empty()) {
        let joined = block.join(" ");
        if SKIP
            .iter()
            .any(|header| header.eq_ignore_ascii_case(&joined))
        {
            continue;
        }

        if let Some(row) = parse_rounds_table_row(&joined) {
            table_started = true;
            table_rows.push(row);
        } else if table_started {
            post_paras.push(joined);
        } else {
            pre_paras.push(joined);
        }
    }

    let mut html = String::new();

    for para in &pre_paras {
        html.push_str(&format!("<p>{}</p>", html_escape(para)));
    }

    if !table_rows.is_empty() {
        let mut rows = String::new();
        for row in &table_rows {
            rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_escape(&row[0]),
                html_escape(&row[1]),
                html_escape(&row[2]),
            ));
        }
        html.push_str(&format!(
            "<table class=\"penalty-table\">\
             <thead><tr><th>Players (Teams)</th><th>Swiss Rounds</th><th>Playoff</th></tr></thead>\
             <tbody>{}</tbody>\
             </table>",
            rows
        ));
    }

    for para in &post_paras {
        html.push_str(&format!("<p>{}</p>", html_escape(para)));
    }

    html
}

fn parse_rounds_table_row(line: &str) -> Option<[String; 3]> {
    let player_re = Regex::new(r"^(\d+[\d\u{2013}\-+]*(?:\s*\([^)]*\))?)\s+(.+)$").unwrap();
    let captures = player_re.captures(line)?;
    let players = captures[1].trim().to_string();
    let remainder = captures[2].trim();

    let playoff_start = [" Top ", " None "]
        .iter()
        .filter_map(|marker| remainder.find(marker).map(|index| index + 1))
        .min()?;
    let swiss = remainder[..playoff_start].trim().to_string();
    let playoff = remainder[playoff_start..].trim().to_string();
    if swiss.is_empty() || playoff.is_empty() {
        return None;
    }
    Some([players, swiss, playoff])
}

fn append_paragraph(para: &str, rules: &mut Vec<RuleDetail>, re_xref: &Regex) {
    if let Some(rule) = rules.last_mut() {
        if !rule.body.is_empty() {
            rule.body.push('\n');
        }
        rule.body.push_str(para);
        let html = html_escape(para);
        let html = bold_label(&html, "Upgrade:");
        let html = bold_label(&html, "Downgrade:");
        rule.body_html
            .push_str(&format!("<p>{}</p>", linkify_mtr(re_xref, &html)));
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns true if `title` looks like a proper MTR section title rather than
/// a numbered list item.
///
/// Real section titles (e.g. "Tournament Basics", "Communication") are:
///   - Short: ≤ 5 words
///   - Title Case: every word longer than 3 chars starts with an uppercase letter
///
/// List items (e.g. "Each player draws their starting hand...") are longer
/// prose sentences and fail both criteria.
fn looks_like_section_title(title: &str) -> bool {
    // Small words that may be lowercase in titles
    const SMALL: &[&str] = &[
        "a", "an", "the", "of", "in", "on", "at", "to", "for", "and", "or", "by", "with",
    ];

    let words: Vec<&str> = title.split_whitespace().collect();
    if words.is_empty() || words.len() > 5 {
        return false;
    }

    // Every word that isn't a small connector word must start with uppercase.
    words.iter().all(|w| {
        let alpha: String = w.chars().filter(|c| c.is_alphabetic()).collect();
        let lower = alpha.to_lowercase();
        if SMALL.contains(&lower.as_str()) {
            true
        } else {
            w.chars().next().map_or(false, |c| c.is_uppercase())
        }
    })
}

fn is_header_footer(line: &str) -> bool {
    // Repeated page header/footer patterns in MTR PDFs
    line.contains("Magic: The Gathering Tournament Rules")
        || line.starts_with("Wizards of the Coast")
        || line.starts_with("©")
        || line.starts_with("WPN ")
}

fn clean_title(s: &str) -> String {
    // Strip trailing dot leaders that pdf-extract may leave
    s.trim_end_matches('.').trim().to_string()
}

fn parent_of(number: &str) -> Option<String> {
    let pos = number.rfind('.')?;
    Some(number[..pos].to_string())
}

fn linkify_mtr(xref_re: &Regex, html: &str) -> String {
    xref_re
        .replace_all(html, |caps: &regex::Captures| {
            let num = &caps[1];
            format!(
                r##"section <a href="#R{num}" class="rule-ref">{num}</a>"##,
                num = num
            )
        })
        .into_owned()
}

fn bold_label(html: &str, label: &str) -> String {
    let pattern = format!(r"\b{}", regex::escape(label));
    let re = Regex::new(&pattern).unwrap();
    re.replace_all(html, |caps: &regex::Captures| {
        format!("<strong>{}</strong>", &caps[0])
    })
    .into_owned()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn starts_list_item(line: &str) -> bool {
    // Matches: "A. ", "B. ", ..., "Z. " or "1. ", "2. ", etc.
    if line.starts_with('•') {
        return true;
    }
    let mut chars = line.chars();
    match (chars.next(), chars.next(), chars.next()) {
        (Some(first), Some('.'), Some(' ')) => {
            first.is_ascii_alphabetic() || first.is_ascii_digit()
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_mtr(body: &str) -> String {
        // Minimal MTR skeleton: version line, then first section to get past TOC.
        format!(
            "Magic Tournament Rules\nEffective as of April 1, 2025\n\n1. Tournament Basics\n\n{body}"
        )
    }

    #[test]
    fn test_section_parsed() {
        let input =
            minimal_mtr("1.1 Definitions\n\nSome text here.\n\n2. Tournament Mechanics\n\n");
        let mtr = parse_mtr(&input);
        assert!(
            mtr.rules.iter().any(|r| r.number == "1"),
            "missing section 1"
        );
        assert!(
            mtr.rules.iter().any(|r| r.number == "1.1"),
            "missing subsection 1.1"
        );
    }

    #[test]
    fn test_version_extracted() {
        let input = minimal_mtr("");
        let mtr = parse_mtr(&input);
        assert_eq!(mtr.version, "April 1, 2025");
    }

    #[test]
    fn test_paragraph_body_accumulated() {
        let input = minimal_mtr("1.1 Definitions\n\nLine one.\nLine two.\n\n");
        let mtr = parse_mtr(&input);
        // The subsection rule itself accumulates the paragraph content.
        let rule = mtr
            .rules
            .iter()
            .find(|r| r.number == "1.1")
            .expect("missing 1.1");
        assert!(
            rule.body.contains("Line one."),
            "body missing line one: {}",
            rule.body
        );
        assert!(
            rule.body.contains("Line two."),
            "body missing line two: {}",
            rule.body
        );
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("a < b & c"), "a &lt; b &amp; c");
    }

    #[test]
    fn test_clean_title_strips_dot_leaders() {
        assert_eq!(
            clean_title("Tournament Basics.........."),
            "Tournament Basics"
        );
        assert_eq!(clean_title("No Dots"), "No Dots");
    }

    #[test]
    fn test_parent_of() {
        assert_eq!(parent_of("1.2.3"), Some("1.2".to_string()));
        assert_eq!(parent_of("1.2"), Some("1".to_string()));
        assert_eq!(parent_of("1"), None);
    }

    #[test]
    fn test_starts_list_item() {
        assert!(starts_list_item("A. Something"));
        assert!(starts_list_item("1. Something"));
        assert!(starts_list_item("• bullet"));
        assert!(!starts_list_item("No match here"));
        assert!(!starts_list_item(""));
    }

    #[test]
    fn test_is_header_footer() {
        assert!(is_header_footer(
            "Magic: The Gathering Tournament Rules 2025"
        ));
        assert!(is_header_footer("Wizards of the Coast LLC"));
        assert!(is_header_footer("© 2025 Wizards"));
        assert!(!is_header_footer("Some normal text"));
    }

    #[test]
    fn appendix_e_wrapped_rows_render_as_a_table_and_stop_at_appendix_f() {
        let input = minimal_mtr(
            "Appendix E—Recommended Number of Rounds in Swiss Tournaments\n\
             \n\
             The following number of Swiss rounds is required for Premier tournaments.\n\
             \n\
             Players (Teams)  Swiss Rounds  Playoff\n\
             \n\
             4 (Team/2HG Only)  2 Single-Elimination\n\
             Rounds (No Swiss)  None (Run Single Elimination)\n\
             \n\
             5-8  3 Single-Elimination\n\
             Rounds (No Swiss)  None (Run Single Elimination)\n\
             \n\
             9-16  4 (if Limited Format with\n\
             Booster Draft in Playoff)\n\
             5 (All Other Formats)\n\
             Top 8 (If Limited Format with\n\
             Booster Draft in Playoff)\n\
             Top 4 (All Other Formats)\n\
             \n\
             17-32  5  Top 8\n\
             \n\
             Team tournaments consider each team as a single player for this purpose.\n\
             \n\
             Appendix F—Rules Enforcement Levels of Programs\n\
             \n\
             Appendix F content must not appear in Appendix E.",
        );

        let parsed = parse_mtr(&input);
        let appendix_e = parsed
            .rules
            .iter()
            .find(|rule| rule.number == "Appendix E")
            .expect("missing Appendix E");
        assert!(appendix_e.body_html.contains("<table"));
        assert!(appendix_e
            .body_html
            .contains("<td>4 (Team/2HG Only)</td><td>2 Single-Elimination Rounds (No Swiss)</td>"));
        assert!(appendix_e.body_html.contains(
            "<td>9-16</td><td>4 (if Limited Format with Booster Draft in Playoff) 5 (All Other Formats)</td><td>Top 8"
        ));
        assert!(appendix_e.body_html.contains("Team tournaments consider"));
        assert!(!appendix_e.body_html.contains("Appendix F content"));
        validate_mtr(&parsed).expect("valid MTR should pass integrity checks");
    }

    #[test]
    fn validation_rejects_a_missing_appendix_e_table() {
        let parsed = ParsedMTR {
            version: "February 27, 2026".to_string(),
            rules: vec![RuleDetail {
                id: 1,
                number: "Appendix E".to_string(),
                title: Some("Recommended Number of Rounds".to_string()),
                body: String::new(),
                body_html: "<p>PDF extraction unexpectedly changed.</p>".to_string(),
                parent: None,
            }],
        };

        let error = validate_mtr(&parsed).expect_err("broken table must be rejected");
        assert!(error.contains("table was not parsed correctly"));
    }
}
