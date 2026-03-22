use crate::models::rule::RuleDetail;
use regex::Regex;
use std::collections::HashSet;

pub struct ParsedMTR {
    pub version: String,
    pub rules: Vec<RuleDetail>,
}

pub fn parse_mtr(raw: &str) -> ParsedMTR {
    // Normalize line endings
    let text = raw.replace("\r\n", "\n").replace('\r', "\n");

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

    macro_rules! flush_para {
        () => {
            if !para_buf.is_empty() {
                append_paragraph(&para_buf, &mut rules, &re_xref);
                para_buf.clear();
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
            // Empty line = paragraph boundary
            flush_para!();
            continue;
        }

        if let Some(caps) = re_appendix.captures(trimmed) {
            flush_para!();
            let number = caps[1].to_string();
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

    ParsedMTR { version, rules }
}

fn append_paragraph(para: &str, rules: &mut Vec<RuleDetail>, re_xref: &Regex) {
    if let Some(rule) = rules.last_mut() {
        if !rule.body.is_empty() {
            rule.body.push('\n');
        }
        rule.body.push_str(para);
        rule.body_html.push_str(&format!(
            "<p>{}</p>",
            linkify_mtr(re_xref, &html_escape(para))
        ));
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
