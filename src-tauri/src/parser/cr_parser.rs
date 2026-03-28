use crate::models::rule::{GlossaryEntry, RuleDetail};
use regex::Regex;

pub struct ParsedCR {
    pub version: String,
    pub toc: Vec<TocEntry>,
    pub rules: Vec<RuleDetail>,
    pub glossary: Vec<GlossaryEntry>,
}

pub struct TocEntry {
    pub number: String,
    pub title: String,
}

#[derive(PartialEq)]
enum State {
    Preamble,
    Toc,
    PostToc, // after "Credits" in TOC, waiting for rules to begin
    Rules,
    Glossary,
    Credits,
}

pub fn parse_cr(raw: &str) -> ParsedCR {
    // Strip UTF-8 BOM and normalize CRLF
    let text = raw.trim_start_matches('\u{FEFF}').replace("\r\n", "\n");

    let re_rule = Regex::new(r"^(\d{3}\.\d+)\.\s+(.+)$").unwrap();
    let re_subrule = Regex::new(r"^(\d{3}\.\d+[a-z])\s+(.+)$").unwrap();
    let re_subsection = Regex::new(r"^(\d{3})\.\s+(.+)$").unwrap();
    let re_section = Regex::new(r"^(\d)\.\s+(.+)$").unwrap();
    let re_xref = Regex::new(r"\brules?\s+(\d{3}(?:\.\d+[a-z]?)?)").unwrap();
    let re_section_ref = Regex::new(r"\bsection\s+(\d)\b").unwrap();
    let re_parent = Regex::new(r"^(\d{3}\.\d+)").unwrap();
    let re_version = Regex::new(r"effective as of (\w+ \d+, \d{4})").unwrap();

    let mut state = State::Preamble;
    let mut version = String::from("unknown");
    let mut toc: Vec<TocEntry> = Vec::new();
    let mut rules: Vec<RuleDetail> = Vec::new();
    let mut glossary: Vec<GlossaryEntry> = Vec::new();

    // Glossary accumulation
    let mut gloss_term: Option<String> = None;
    let mut gloss_def = String::new();

    let mut sort_order: i64 = 0;

    for line in text.lines() {
        let trimmed = line.trim();

        if state == State::Preamble {
            if let Some(caps) = re_version.captures(trimmed) {
                version = caps[1].to_string();
            }
            if trimmed == "Contents" {
                state = State::Toc;
            }
            continue;
        }

        if state == State::Toc {
            if trimmed == "Credits" {
                state = State::PostToc;
            } else if !trimmed.is_empty() {
                if let Some(caps) = re_subsection.captures(trimmed) {
                    toc.push(TocEntry { number: caps[1].to_string(), title: caps[2].to_string() });
                } else if let Some(caps) = re_section.captures(trimmed) {
                    toc.push(TocEntry { number: caps[1].to_string(), title: caps[2].to_string() });
                }
            }
            continue;
        }

        if state == State::PostToc {
            if re_section.is_match(trimmed) {
                state = State::Rules;
                // fall through to process this line as the first rule
            } else {
                continue;
            }
        }

        if state == State::Rules {
            if trimmed == "Glossary" {
                state = State::Glossary;
                continue;
            }
            if trimmed.is_empty() {
                continue;
            }

            sort_order += 1;

            if let Some(caps) = re_rule.captures(trimmed) {
                let number = caps[1].to_string();
                let body = caps[2].to_string();
                let body_html = linkify(&re_xref, &re_section_ref, &html_escape(&body));
                let parent = number.split('.').next().map(|s| s.to_string());
                rules.push(RuleDetail { id: sort_order, number, title: None, body, body_html, parent });
            } else if let Some(caps) = re_subrule.captures(trimmed) {
                let number = caps[1].to_string();
                let body = caps[2].to_string();
                let body_html = linkify(&re_xref, &re_section_ref, &html_escape(&body));
                let parent = re_parent.captures(&number).map(|c| c[1].to_string());
                rules.push(RuleDetail { id: sort_order, number, title: None, body, body_html, parent });
            } else if let Some(caps) = re_subsection.captures(trimmed) {
                let number = caps[1].to_string();
                let title = caps[2].to_string();
                let body_html = html_escape(&title);
                rules.push(RuleDetail { id: sort_order, number, title: Some(title.clone()), body: title, body_html, parent: None });
            } else if let Some(caps) = re_section.captures(trimmed) {
                let number = caps[1].to_string();
                let title = caps[2].to_string();
                let body_html = html_escape(&title);
                rules.push(RuleDetail { id: sort_order, number, title: Some(title.clone()), body: title, body_html, parent: None });
            } else if let Some(last) = rules.last_mut() {
                // Continuation line — append to previous rule
                let escaped = html_escape(trimmed);
                let linked = linkify(&re_xref, &re_section_ref, &escaped);
                last.body.push(' ');
                last.body.push_str(trimmed);
                last.body_html.push(' ');
                last.body_html.push_str(&linked);
            }

            continue;
        }

        if state == State::Glossary {
            if trimmed == "Credits" {
                flush_glossary(&mut gloss_term, &mut gloss_def, &mut glossary);
                state = State::Credits;
                continue;
            }
            if trimmed.is_empty() {
                flush_glossary(&mut gloss_term, &mut gloss_def, &mut glossary);
                continue;
            }
            // Non-indented line = new term; indented = definition continuation
            if !line.starts_with(' ') && !line.starts_with('\t') {
                flush_glossary(&mut gloss_term, &mut gloss_def, &mut glossary);
                gloss_term = Some(trimmed.to_string());
            } else if gloss_term.is_some() {
                if !gloss_def.is_empty() { gloss_def.push(' '); }
                gloss_def.push_str(trimmed);
            }
            continue;
        }
        // State::Credits — done
    }

    ParsedCR { version, toc, rules, glossary }
}

fn flush_glossary(term: &mut Option<String>, def: &mut String, glossary: &mut Vec<GlossaryEntry>) {
    if let Some(t) = term.take() {
        glossary.push(GlossaryEntry { term: t, definition: def.trim().to_string() });
        def.clear();
    }
}

fn linkify(xref_re: &Regex, section_re: &Regex, html: &str) -> String {
    let s = xref_re.replace_all(html, |caps: &regex::Captures| {
        let num = &caps[1];
        format!(r##"rules <a href="#R{num}" class="rule-ref">{num}</a>"##, num = num)
    });
    let s = section_re.replace_all(&s, |caps: &regex::Captures| {
        let num = &caps[1];
        format!(r##"section <a href="#R{num}" class="rule-ref">{num}</a>"##, num = num)
    });
    s.into_owned()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rule_and_subrule() {
        let input = "Contents\n1. Game Concepts\nCredits\n\n1. Game Concepts\n\n100. General\n\n100.1. These are the rules.\n\n100.1a A subrule here.\n\nGlossary\n\nCredits\n";
        let cr = parse_cr(input);
        assert!(cr.rules.iter().any(|r| r.number == "100.1"), "missing 100.1");
        assert!(cr.rules.iter().any(|r| r.number == "100.1a"), "missing 100.1a");
    }

    #[test]
    fn test_linkify_rule_ref() {
        let xref_re = Regex::new(r"\brules?\s+(\d{3}(?:\.\d+[a-z]?)?)").unwrap();
        let section_re = Regex::new(r"\bsection\s+(\d)\b").unwrap();
        let result = linkify(&xref_re, &section_re, "See rules 704.5k for details.");
        assert!(result.contains(r##"href="#R704.5k""##));
    }

    #[test]
    fn test_glossary_parsed() {
        let input = "Contents\n1. Game Concepts\nCredits\n\n1. Game Concepts\n\nGlossary\n\nAbility\n  Text on an object.\n\nCredits\n";
        let cr = parse_cr(input);
        assert!(cr.glossary.iter().any(|g| g.term == "Ability"), "missing Ability");
    }

    #[test]
    fn test_version_extracted() {
        let input = "Magic: The Gathering Comprehensive Rules\nThese rules are effective as of January 1, 2025\n\nContents\nCredits\n\nGlossary\n\nCredits\n";
        let cr = parse_cr(input);
        assert_eq!(cr.version, "January 1, 2025");
    }

    #[test]
    fn test_toc_entries_parsed() {
        let input = "Contents\n1. Game Concepts\n100. General\n2. Parts of a Card\nCredits\n\n1. Game Concepts\n\nGlossary\n\nCredits\n";
        let cr = parse_cr(input);
        assert!(cr.toc.iter().any(|e| e.number == "1" && e.title == "Game Concepts"));
        assert!(cr.toc.iter().any(|e| e.number == "100" && e.title == "General"));
    }

    #[test]
    fn test_continuation_line_appended() {
        let input = "Contents\n1. Game Concepts\nCredits\n\n1. Game Concepts\n\n100. General\n\n100.1. First sentence.\nSecond sentence.\n\nGlossary\n\nCredits\n";
        let cr = parse_cr(input);
        let rule = cr.rules.iter().find(|r| r.number == "100.1").expect("missing 100.1");
        assert!(rule.body.contains("Second sentence."), "continuation not appended: {}", rule.body);
    }

    #[test]
    fn test_section_ref_linkified() {
        let xref_re = Regex::new(r"\brules?\s+(\d{3}(?:\.\d+[a-z]?)?)").unwrap();
        let section_re = Regex::new(r"\bsection\s+(\d)\b").unwrap();
        let result = linkify(&xref_re, &section_re, "See section 2 for details.");
        assert!(result.contains(r##"href="#R2""##), "section ref not linked: {result}");
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("a < b & c > d"), "a &lt; b &amp; c &gt; d");
        assert_eq!(html_escape("no special chars"), "no special chars");
    }

    #[test]
    fn test_bom_stripped() {
        let input = "\u{FEFF}Contents\n1. Game Concepts\nCredits\n\n1. Game Concepts\n\nGlossary\n\nCredits\n";
        let cr = parse_cr(input);
        assert!(cr.toc.iter().any(|e| e.number == "1"));
    }

    #[test]
    fn test_glossary_multiline_definition() {
        let input = "Contents\n1. Game Concepts\nCredits\n\n1. Game Concepts\n\nGlossary\n\nAbsorb\n  Line one.\n  Line two.\n\nCredits\n";
        let cr = parse_cr(input);
        let entry = cr.glossary.iter().find(|g| g.term == "Absorb").expect("missing Absorb");
        assert!(entry.definition.contains("Line one."), "def missing line one: {}", entry.definition);
        assert!(entry.definition.contains("Line two."), "def missing line two: {}", entry.definition);
    }
}
