use crate::models::rule::RuleDetail;
use regex::Regex;

pub struct ParsedJAR {
    pub version: String,
    pub rules: Vec<RuleDetail>,
}

const SECTION_NAMES: &[(&str, &str)] = &[
    ("Common Issues", "1"),
    ("General Unwanted Behaviors", "2"),
    ("Serious Problems", "3"),
    ("Resources", "4"),
];

pub fn parse_jar(raw: &str) -> ParsedJAR {
    let text = raw.replace("\r\n", "\n").replace('\r', "\n");
    let re_version = Regex::new(r"(?i)updated\s+([A-Za-z]+\s+\d+,?\s+\d{4})").unwrap();
    let re_only_digits = Regex::new(r"^\d+$").unwrap();

    let mut version = String::from("unknown");
    let mut rules: Vec<RuleDetail> = Vec::new();
    let mut sort_order: i64 = 0;
    let mut para_buf = String::new();
    let mut in_common_issues = false;
    let mut subsection_counter: u32 = 0;

    // Introduction section collects pre-section preamble text
    sort_order += 1;
    rules.push(RuleDetail {
        id: sort_order,
        number: "0".to_string(),
        title: Some("Introduction".to_string()),
        body: String::new(),
        body_html: String::new(),
        parent: None,
    });

    macro_rules! flush_para {
        () => {
            if !para_buf.is_empty() {
                append_paragraph(&para_buf, &mut rules);
                para_buf.clear();
            }
        };
    }

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            flush_para!();
            continue;
        }

        if re_only_digits.is_match(trimmed) {
            continue;
        }

        if is_header_footer(trimmed) {
            continue;
        }

        // Extract version from "Updated September 25, 2020"
        if version == "unknown" {
            if let Some(caps) = re_version.captures(trimmed) {
                version = caps[1].to_string();
                continue;
            }
        }

        // Check for a known section header (case-insensitive exact match)
        if let Some(&(title, number)) = SECTION_NAMES
            .iter()
            .find(|(t, _)| t.eq_ignore_ascii_case(trimmed))
        {
            flush_para!();
            in_common_issues = number == "1";
            subsection_counter = 0;
            sort_order += 1;
            rules.push(RuleDetail {
                id: sort_order,
                number: number.to_string(),
                title: Some(title.to_string()),
                body: title.to_string(),
                body_html: String::new(),
                parent: None,
            });
            continue;
        }

        // Within Common Issues, "A player..." lines at the start of a paragraph are sub-issue headers
        if in_common_issues && trimmed.starts_with("A player") && para_buf.is_empty() {
            subsection_counter += 1;
            let sub_number = format!("1.{}", subsection_counter);
            let title = trimmed.trim_end_matches('.').to_string();
            sort_order += 1;
            rules.push(RuleDetail {
                id: sort_order,
                number: sub_number.clone(),
                title: Some(title.clone()),
                body: title.clone(),
                body_html: format!("<p><strong>{}</strong></p>", html_escape(&title)),
                parent: Some("1".to_string()),
            });
            continue;
        }

        // Bullet points start a new paragraph
        if trimmed.starts_with('•') || trimmed.starts_with('-') {
            flush_para!();
        } else if !para_buf.is_empty() {
            para_buf.push(' ');
        }
        para_buf.push_str(trimmed);
    }

    flush_para!();
    ParsedJAR { version, rules }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn append_paragraph(para: &str, rules: &mut Vec<RuleDetail>) {
    if let Some(rule) = rules.last_mut() {
        if !rule.body.is_empty() {
            rule.body.push('\n');
        }
        rule.body.push_str(para);
        let html = if para.starts_with('•') || para.starts_with('-') {
            format!("<p class=\"bullet\">{}</p>", html_escape(para))
        } else {
            format!("<p>{}</p>", html_escape(para))
        };
        rule.body_html.push_str(&html);
    }
}

fn is_header_footer(line: &str) -> bool {
    line.contains("Judging at Regular")
        || line.starts_with("Wizards of the Coast")
        || line.starts_with("©")
        || line.starts_with("WPN ")
        || line.starts_with("Magic: The Gathering")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_jar(body: &str) -> String {
        format!(
            "Judging at Regular Rules Enforcement Level\nUpdated September 25, 2020\n\n{body}"
        )
    }

    #[test]
    fn test_version_extracted() {
        let input = minimal_jar("Common Issues\n\n");
        let jar = parse_jar(&input);
        assert_eq!(jar.version, "September 25, 2020");
    }

    #[test]
    fn test_sections_parsed() {
        let input = minimal_jar(
            "Common Issues\n\nGeneral Unwanted Behaviors\n\nSerious Problems\n\nResources\n\n",
        );
        let jar = parse_jar(&input);
        assert!(jar.rules.iter().any(|r| r.number == "1"), "missing section 1");
        assert!(jar.rules.iter().any(|r| r.number == "2"), "missing section 2");
        assert!(jar.rules.iter().any(|r| r.number == "3"), "missing section 3");
        assert!(jar.rules.iter().any(|r| r.number == "4"), "missing section 4");
    }

    #[test]
    fn test_common_issue_subsection() {
        let input = minimal_jar(
            "Common Issues\n\nA player forgets a triggered ability\n\nThe trigger was not remembered.\n\n",
        );
        let jar = parse_jar(&input);
        assert!(
            jar.rules.iter().any(|r| r.number == "1.1"),
            "missing subsection 1.1"
        );
        let sub = jar.rules.iter().find(|r| r.number == "1.1").unwrap();
        assert_eq!(sub.parent, Some("1".to_string()));
    }

    #[test]
    fn test_intro_section_exists() {
        let input = minimal_jar("Intro text here.\n\nCommon Issues\n\n");
        let jar = parse_jar(&input);
        let intro = jar.rules.iter().find(|r| r.number == "0").unwrap();
        assert!(intro.body.contains("Intro text here."));
    }

    #[test]
    fn test_html_escape() {
        let input = minimal_jar("Common Issues\n\nA player has a card with a < b & c\n\n");
        let jar = parse_jar(&input);
        let sub = jar.rules.iter().find(|r| r.number == "1.1").unwrap();
        assert!(sub.body_html.contains("&lt;"));
        assert!(sub.body_html.contains("&amp;"));
    }
}
