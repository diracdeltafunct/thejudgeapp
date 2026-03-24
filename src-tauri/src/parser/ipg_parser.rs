use crate::models::rule::RuleDetail;
use regex::Regex;
use std::collections::HashSet;

pub struct ParsedIPG {
    pub version: String,
    pub rules: Vec<RuleDetail>,
}

const IPG_SUBHEADERS: &[&str] = &[
    "Definition",
    "Philosophy",
    "Penalty",
    "Penalties",
    "Additional Remedy",
    "Upgrade",
    "Downgrade",
];

const PENALTY_KEYWORDS: &[&str] = &[
    "Disqualification",
    "Match Loss",
    "Game Loss",
    "Warning",
    "None",
];

// Lines that are just the PDF table's column headers — skip them
const PDF_HEADER_LINES: &[&str] = &["Infraction", "Penalty", "Infraction Penalty"];

pub fn parse_ipg(raw: &str) -> ParsedIPG {
    let text = raw.replace("\r\n", "\n").replace('\r', "\n");
    let re_section = Regex::new(r"^(\d+)\.\s+(.+)$").unwrap();
    let re_subsection = Regex::new(r"^(\d+\.\d+(?:\.\d+)*)\.?\s+(.+)$").unwrap();
    let re_only_digits = Regex::new(r"^\d+$").unwrap();
    let re_version =
        Regex::new(r"(?i)effective\s+(?:as\s+of\s+)?([A-Za-z]+\s+\d+,?\s+\d{4})").unwrap();
    let re_appendix = Regex::new(r"(?i)^(Appendix\s+[A-Z])\s*\u{2014}\s*(.+)$").unwrap();
    let re_xref = Regex::new(r"\bsection\s+(\d+(?:\.\d+)*)").unwrap();
    let re_xref_mtr = Regex::new(r"\bsection\s+(\d+(?:\.\d+)*)\s+of\s+the\s+Magic\s+Tournament\s+Rules").unwrap();

    let mut version = String::from("unknown");
    let mut rules: Vec<RuleDetail> = Vec::new();
    let mut sort_order: i64 = 0;
    let mut past_toc = false;
    let mut last_section_num: u32 = 0;
    let mut seen_subsections: HashSet<String> = HashSet::new();

    // Buffer for accumulating lines of a paragraph before flushing.
    let mut para_buf = String::new();

    // Appendix A gets special treatment: collect raw lines, build a table at the end.
    let mut in_appendix_a = false;
    let mut appendix_a_lines: Vec<String> = Vec::new();

    macro_rules! flush_para {
        () => {
            if !para_buf.is_empty() {
                append_paragraph(&para_buf, &mut rules, &re_xref, &re_xref_mtr);
                para_buf.clear();
            }
        };
    }

    macro_rules! finalize_appendix_a {
        () => {
            if in_appendix_a {
                if let Some(rule) = rules.iter_mut().find(|r| r.number == "Appendix A") {
                    rule.body_html = build_penalty_table_html(&appendix_a_lines);
                }
                appendix_a_lines.clear();
            }
        };
    }

    for line in text.lines() {
        let trimmed = line.trim();
        if re_only_digits.is_match(trimmed) {
            continue;
        }
        if is_header_footer(trimmed) {
            continue;
        }
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
            if !in_appendix_a {
                flush_para!();
            }
            continue;
        }

        // New appendix heading — finalize Appendix A if we were in it, then start the new one.
        if let Some(caps) = re_appendix.captures(trimmed) {
            flush_para!();
            finalize_appendix_a!();
            let letter = caps[1]
                .trim()
                .chars()
                .last()
                .unwrap_or('A')
                .to_ascii_uppercase();
            let number = format!("Appendix {}", letter);
            let raw_title = caps[2].trim();
            let title = clean_title(&title_case(raw_title));
            sort_order += 1;
            rules.push(RuleDetail {
                id: sort_order,
                number: number.clone(),
                title: Some(title.clone()),
                body: title,
                body_html: String::new(),
                parent: None,
            });
            if letter == 'A' {
                in_appendix_a = true;
            }
            continue;
        }

        // When inside Appendix A, collect every non-empty line individually.
        if in_appendix_a {
            appendix_a_lines.push(trimmed.to_string());
            continue;
        }

        // IPG sub-headers (Definition, Philosophy, etc.)
        let lower = trimmed.to_lowercase();
        if IPG_SUBHEADERS.iter().any(|h| h.to_lowercase() == lower) {
            flush_para!();
            if let Some(rule) = rules.last_mut() {
                rule.body.push_str(trimmed);
                rule.body_html
                    .push_str(&format!("<strong>{}</strong>", html_escape(trimmed)));
            }
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
    finalize_appendix_a!();

    ParsedIPG { version, rules }
}

// ── Appendix A table builder ─────────────────────────────────────────────────

fn build_penalty_table_html(lines: &[String]) -> String {
    let mut rows = String::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Skip the PDF's original column headers
        if PDF_HEADER_LINES
            .iter()
            .any(|h| h.to_lowercase() == line.to_lowercase())
        {
            i += 1;
            continue;
        }

        // If the next line is a bare penalty keyword the PDF split the row across
        // two lines — merge them so we get "Infraction Warning" as one string.
        let next_is_bare_penalty =
            i + 1 < lines.len() && PENALTY_KEYWORDS.iter().any(|&k| lines[i + 1].trim() == k);

        let test_line: String = if next_is_bare_penalty {
            format!("{} {}", line, lines[i + 1].trim())
        } else {
            line.to_string()
        };

        // Find the rightmost penalty keyword that is preceded by a space.
        let mut best: Option<(usize, &str)> = None;
        for &kw in PENALTY_KEYWORDS {
            if let Some(pos) = test_line.rfind(kw) {
                if pos > 0 && test_line.as_bytes()[pos - 1] == b' ' {
                    if best.map_or(true, |(p, _)| pos > p) {
                        best = Some((pos, kw));
                    }
                }
            }
        }

        if let Some((pos, kw)) = best {
            let infraction = test_line[..pos].trim_end_matches('/').trim();
            let penalty_text = test_line[pos..].trim();
            if !infraction.is_empty() {
                rows.push_str(&format!(
                    "<tr><td>{}</td><td class=\"penalty-cell {}\">{}</td></tr>",
                    html_escape(infraction),
                    penalty_css_class(kw),
                    html_escape(penalty_text),
                ));
                if next_is_bare_penalty {
                    i += 1;
                }
                i += 1;
                continue;
            }
        }

        // Not a penalty row — render as a category header spanning both columns.
        rows.push_str(&format!(
            "<tr class=\"penalty-category\"><td colspan=\"2\">{}</td></tr>",
            html_escape(line),
        ));
        i += 1;
    }

    format!(
        "<table class=\"penalty-table\">\
         <thead><tr><th>Infraction</th><th>Penalty</th></tr></thead>\
         <tbody>{}</tbody>\
         </table>",
        rows
    )
}

fn penalty_css_class(kw: &str) -> &'static str {
    match kw {
        "Disqualification" => "penalty-dq",
        "Match Loss" => "penalty-match-loss",
        "Game Loss" => "penalty-game-loss",
        "Warning" => "penalty-warning",
        "None" => "penalty-warning", // hack to make this format automatically
        _ => "",
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn append_paragraph(para: &str, rules: &mut Vec<RuleDetail>, re_xref: &Regex, re_xref_mtr: &Regex) {
    if let Some(rule) = rules.last_mut() {
        if !rule.body.is_empty() {
            rule.body.push('\n');
        }
        rule.body.push_str(para);
        rule.body_html.push_str(&format!(
            "<p>{}</p>",
            linkify_ipg(re_xref, re_xref_mtr, &html_escape(para))
        ));
    }
}

fn looks_like_section_title(title: &str) -> bool {
    const SMALL: &[&str] = &[
        "a", "an", "the", "of", "in", "on", "at", "to", "for", "and", "or", "by", "with",
    ];
    let words: Vec<&str> = title.split_whitespace().collect();
    if words.is_empty() || words.len() > 5 {
        return false;
    }
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
    line.contains("Magic: The Gathering Infraction Procedure Guide")
        || line.contains("Infraction Procedure Guide")
            && (line.starts_with("Wizards") || line.starts_with("©") || line.starts_with("WPN "))
        || line.starts_with("Wizards of the Coast")
        || line.starts_with("©")
        || line.starts_with("WPN ")
}

fn clean_title(s: &str) -> String {
    s.trim_end_matches('.').trim().to_string()
}

fn parent_of(number: &str) -> Option<String> {
    let pos = number.rfind('.')?;
    Some(number[..pos].to_string())
}

fn linkify_ipg(xref_re: &Regex, xref_mtr_re: &Regex, html: &str) -> String {
    // First, replace cross-document MTR references (must run before the generic pass)
    let after_mtr = xref_mtr_re
        .replace_all(html, |caps: &regex::Captures| {
            let num = &caps[1];
            format!(
                r##"section <a href="#R{num}" class="rule-ref" data-doc="mtr">{num}</a> of the Magic Tournament Rules"##,
                num = num
            )
        });
    // Then replace remaining same-document section references
    xref_re
        .replace_all(&after_mtr, |caps: &regex::Captures| {
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

fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn starts_list_item(line: &str) -> bool {
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
