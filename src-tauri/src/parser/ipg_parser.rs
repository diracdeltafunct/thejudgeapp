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

pub fn parse_ipg(raw: &str) -> ParsedIPG {
    let text = raw.replace("\r\n", "\n").replace('\r', "\n");
    let re_section = Regex::new(r"^(\d+)\.\s+(.+)$").unwrap();
    let re_subsection = Regex::new(r"^(\d+\.\d+(?:\.\d+)*)\.?\s+(.+)$").unwrap();
    let re_only_digits = Regex::new(r"^\d+$").unwrap();
    let re_version =
        Regex::new(r"(?i)effective\s+(?:as\s+of\s+)?([A-Za-z]+\s+\d+,?\s+\d{4})").unwrap();
    let re_xref = Regex::new(r"\bsection\s+(\d+(?:\.\d+)*)").unwrap();

    let mut version = String::from("unknown");
    let mut rules: Vec<RuleDetail> = Vec::new();
    let mut sort_order: i64 = 0;
    let mut past_toc = false;
    let mut last_section_num: u32 = 0;
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
        if re_only_digits.is_match(trimmed) {
            continue;
        }
        if is_header_footer(trimmed) {
            continue;
        }
        if let Some(caps) = re_version.captures(trimmed) {
            version = caps[1].to_string();
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

        // IPG sub-headers (Definition, Philosophy, etc.) flush the current para
        // and are emitted immediately as their own block.
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

    ParsedIPG { version, rules }
}

fn append_paragraph(para: &str, rules: &mut Vec<RuleDetail>, re_xref: &Regex) {
    if let Some(rule) = rules.last_mut() {
        if !rule.body.is_empty() {
            rule.body.push('\n');
        }
        rule.body.push_str(para);
        rule.body_html.push_str(&format!(
            "<p>{}</p>",
            linkify_ipg(re_xref, &html_escape(para))
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

fn linkify_ipg(xref_re: &Regex, html: &str) -> String {
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
