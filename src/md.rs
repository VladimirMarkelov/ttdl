//! Markdown rendering for task subject text.
//!
//! Provides terminal-aware rendering of a CommonMark subset:
//! bold, italic, inline code, and links — with optional OSC 8 hyperlinks
//! and syntax highlighting of `+projects`, `@contexts`, and `tag:value` fields.

use std::io;
use std::sync::OnceLock;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use regex::Regex;
use termcolor::{ColorSpec, HyperlinkSpec, WriteColor};

use crate::fmt;

fn bare_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"https?://\S+").unwrap())
}

/// Strips common trailing punctuation characters from a bare URL match.
///
/// Greedy `\S+` can absorb trailing `.`, `,`, `)`, etc. that belong to the
/// surrounding prose.  This helper peels them off so only the URL is linked.
/// Returns `(url, trailing)` where `trailing` is the stripped suffix.
fn trim_url_punctuation(url: &str) -> (&str, &str) {
    let trim_chars: &[char] = &['.', ',', ')', ']', '!', '?', ';', ':'];
    let trimmed = url.trim_end_matches(trim_chars);
    let trailing = &url[trimmed.len()..];
    (trimmed, trailing)
}

/// Returns the visible text that a terminal would display after markdown rendering.
/// Strips all formatting — used for width calculation and wrapping.
pub fn visible_text(input: &str) -> String {
    let parser = Parser::new_ext(input, Options::empty());
    let mut out = String::new();
    for event in parser {
        match event {
            Event::Text(t) => out.push_str(&t),
            Event::Code(t) => out.push_str(&t),
            Event::SoftBreak | Event::HardBreak => out.push(' '),
            _ => {}
        }
    }
    out
}

/// Merges bold/italic/underline from the markdown style stack into a base ColorSpec.
fn merge_md_style(base: &ColorSpec, bold: bool, italic: bool, underline: bool) -> ColorSpec {
    let mut spc = base.clone();
    if bold {
        spc.set_bold(true);
    }
    if italic {
        spc.set_italic(true);
    }
    if underline {
        spc.set_underline(true);
    }
    spc
}

/// Writes `text` with syntax highlighting (projects, contexts, tags, hashtags)
/// while preserving bold/italic/underline from the markdown stack.
fn write_text_with_syntax<W: WriteColor>(
    stdout: &mut W,
    text: &str,
    base: &ColorSpec,
    bold: bool,
    italic: bool,
    underline: bool,
    c: &fmt::Conf,
) -> io::Result<()> {
    let merged = merge_md_style(base, bold, italic, underline);
    if !c.syntax {
        write_text_with_urls(stdout, text, &merged, c)?;
        return Ok(());
    }
    let words = fmt::parse_subj(text);
    for word in words.iter() {
        let word_color = if fmt::is_project(word) {
            merge_md_style(&c.colors.project, bold, italic, underline)
        } else if fmt::is_hashtag(word) {
            merge_md_style(&c.colors.hashtag, bold, italic, underline)
        } else if fmt::is_context(word) {
            merge_md_style(&c.colors.context, bold, italic, underline)
        } else if fmt::is_tag(word) {
            merge_md_style(&c.colors.tag, bold, italic, underline)
        } else {
            merged.clone()
        };
        write_text_with_urls(stdout, word, &word_color, c)?;
    }
    Ok(())
}

/// Writes text, wrapping bare URLs in OSC 8 hyperlinks when running in a terminal.
/// Trailing punctuation is stripped from matched URLs and printed as plain text.
fn write_text_with_urls<W: WriteColor>(stdout: &mut W, text: &str, color: &ColorSpec, c: &fmt::Conf) -> io::Result<()> {
    if !c.atty || c.color_term == fmt::TermColorType::None {
        stdout.set_color(color)?;
        write!(stdout, "{text}")?;
        return Ok(());
    }

    let mut last = 0;
    for m in bare_url_re().find_iter(text) {
        if m.start() > last {
            stdout.set_color(color)?;
            write!(stdout, "{}", &text[last..m.start()])?;
        }
        let (url, trailing) = trim_url_punctuation(m.as_str());
        let mut url_color = color.clone();
        url_color.set_underline(true);
        stdout.set_hyperlink(&HyperlinkSpec::open(url.as_bytes()))?;
        stdout.set_color(&url_color)?;
        write!(stdout, "{url}")?;
        stdout.set_hyperlink(&HyperlinkSpec::close())?;
        if !trailing.is_empty() {
            stdout.set_color(color)?;
            write!(stdout, "{trailing}")?;
        }
        last = m.end();
    }
    if last < text.len() {
        stdout.set_color(color)?;
        write!(stdout, "{}", &text[last..])?;
    }
    Ok(())
}

/// Renders markdown-formatted text to the terminal with ANSI styling.
///
/// Supports bold, italic, inline code, links, and bare URLs.
/// When `c.syntax` is true, also highlights +projects, @contexts, #hashtags, and tags.
pub fn print_markdown<W: WriteColor>(stdout: &mut W, text: &str, base: &ColorSpec, c: &fmt::Conf) -> io::Result<()> {
    let parser = Parser::new_ext(text, Options::empty());
    let mut bold = false;
    let mut italic = false;
    let mut in_link = false;

    for event in parser {
        match event {
            Event::Start(Tag::Emphasis) => italic = true,
            Event::Start(Tag::Strong) => bold = true,
            Event::Start(Tag::Link { dest_url, .. }) => {
                in_link = true;
                if c.atty && c.color_term != fmt::TermColorType::None {
                    stdout.set_hyperlink(&HyperlinkSpec::open(dest_url.as_bytes()))?;
                }
            }
            Event::End(TagEnd::Emphasis) => italic = false,
            Event::End(TagEnd::Strong) => bold = false,
            Event::End(TagEnd::Link) => {
                if c.atty && c.color_term != fmt::TermColorType::None {
                    stdout.set_hyperlink(&HyperlinkSpec::close())?;
                }
                in_link = false;
            }
            Event::Code(t) => {
                let code_color = merge_md_style(&c.colors.code, bold, italic, in_link);
                stdout.set_color(&code_color)?;
                write!(stdout, "{t}")?;
            }
            Event::Text(t) => {
                if in_link {
                    // Inside a link — `in_link` maps to underline in merge_md_style
                    let merged = merge_md_style(base, bold, italic, true);
                    stdout.set_color(&merged)?;
                    write!(stdout, "{t}")?;
                } else {
                    write_text_with_syntax(stdout, &t, base, bold, italic, false, c)?;
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                write!(stdout, " ")?;
            }
            _ => {} // Ignore block-level events
        }
    }

    Ok(())
}

/// Renders a range of markdown text by visible character count.
///
/// Skips the first `skip` visible characters, then renders up to `limit`
/// visible characters. Used for wrapped/cut lines: line 1 uses skip=0,
/// line 2 uses skip=len(line1), etc.
///
/// Note: when `skip` or `limit` bisects a bare URL, the halves are rendered
/// as plain text rather than hyperlinks.  This is a known limitation of the
/// character-offset slicing approach; the output is visually correct.
pub fn print_markdown_range<W: WriteColor>(
    stdout: &mut W,
    text: &str,
    skip: usize,
    limit: usize,
    base: &ColorSpec,
    c: &fmt::Conf,
) -> io::Result<()> {
    let parser = Parser::new_ext(text, Options::empty());
    let mut bold = false;
    let mut italic = false;
    let mut in_link = false;
    let mut link_url_str: Option<String> = None;
    let mut link_hyperlink_opened = false;
    let mut visible_pos = 0usize;
    let end = skip + limit;

    for event in parser {
        if visible_pos >= end {
            break;
        }
        match event {
            Event::Start(Tag::Emphasis) => italic = true,
            Event::Start(Tag::Strong) => bold = true,
            Event::Start(Tag::Link { dest_url, .. }) => {
                in_link = true;
                link_url_str = Some(dest_url.to_string());
                link_hyperlink_opened = false;
                // Open hyperlink eagerly when already in the visible range
                if visible_pos >= skip && c.atty && c.color_term != fmt::TermColorType::None {
                    stdout.set_hyperlink(&HyperlinkSpec::open(dest_url.as_bytes()))?;
                    link_hyperlink_opened = true;
                }
            }
            Event::End(TagEnd::Emphasis) => italic = false,
            Event::End(TagEnd::Strong) => bold = false,
            Event::End(TagEnd::Link) => {
                if link_hyperlink_opened && c.atty && c.color_term != fmt::TermColorType::None {
                    stdout.set_hyperlink(&HyperlinkSpec::close())?;
                }
                in_link = false;
                link_url_str = None;
                link_hyperlink_opened = false;
            }
            Event::Code(t) => {
                for ch in t.chars() {
                    if visible_pos >= end {
                        break;
                    }
                    if visible_pos >= skip {
                        let code_color = merge_md_style(&c.colors.code, bold, italic, in_link);
                        stdout.set_color(&code_color)?;
                        write!(stdout, "{ch}")?;
                    }
                    visible_pos += 1;
                }
            }
            Event::Text(t) => {
                // Compute the slice of text that falls in our range
                let text_start = visible_pos;
                let text_chars: Vec<char> = t.chars().collect();
                let text_end = text_start + text_chars.len();

                if text_end <= skip || text_start >= end {
                    // Entirely outside our range
                    visible_pos += text_chars.len();
                    continue;
                }

                let range_start = skip.saturating_sub(text_start);
                let range_end = if text_end > end { end - text_start } else { text_chars.len() };
                let slice: String = text_chars[range_start..range_end].iter().collect();

                if in_link {
                    // Lazy-open hyperlink when the link straddles the skip boundary
                    if !link_hyperlink_opened
                        && c.atty
                        && c.color_term != fmt::TermColorType::None
                        && let Some(ref url) = link_url_str
                    {
                        stdout.set_hyperlink(&HyperlinkSpec::open(url.as_bytes()))?;
                        link_hyperlink_opened = true;
                    }
                    let merged = merge_md_style(base, bold, italic, true);
                    stdout.set_color(&merged)?;
                    write!(stdout, "{slice}")?;
                } else {
                    write_text_with_syntax(stdout, &slice, base, bold, italic, false, c)?;
                }
                visible_pos += text_chars.len();
            }
            Event::SoftBreak | Event::HardBreak => {
                if visible_pos >= skip && visible_pos < end {
                    write!(stdout, " ")?;
                }
                visible_pos += 1;
            }
            _ => {}
        }
    }

    // Close any open hyperlink if we broke out of the loop mid-link
    if in_link && link_hyperlink_opened && c.atty && c.color_term != fmt::TermColorType::None {
        stdout.set_hyperlink(&HyperlinkSpec::close())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use termcolor::{Buffer, ColorSpec};

    // ── visible_text ──────────────────────────────────────────────────────────

    #[test]
    fn visible_text_bold() {
        assert_eq!(visible_text("**bold**"), "bold");
    }

    #[test]
    fn visible_text_italic() {
        assert_eq!(visible_text("*italic*"), "italic");
    }

    #[test]
    fn visible_text_code() {
        assert_eq!(visible_text("`code`"), "code");
    }

    #[test]
    fn visible_text_link() {
        assert_eq!(visible_text("[label](https://example.com)"), "label");
    }

    #[test]
    fn visible_text_plain() {
        assert_eq!(visible_text("plain text"), "plain text");
    }

    #[test]
    fn visible_text_unmatched_star() {
        assert_eq!(visible_text("a * b"), "a * b");
    }

    #[test]
    fn visible_text_underscore_tag() {
        // CommonMark flanking rules: underscores within words don't trigger emphasis
        assert_eq!(visible_text("due_date:2024-01-01"), "due_date:2024-01-01");
    }

    #[test]
    fn visible_text_bold_italic() {
        assert_eq!(visible_text("***both***"), "both");
    }

    #[test]
    fn visible_text_mixed() {
        assert_eq!(visible_text("**bold** and *italic* and `code`"), "bold and italic and code");
    }

    #[test]
    fn visible_text_empty() {
        assert_eq!(visible_text(""), "");
    }

    #[test]
    fn visible_text_nested_formatting() {
        assert_eq!(visible_text("**bold *and italic***"), "bold and italic");
    }

    #[test]
    fn visible_text_soft_break() {
        // pulldown-cmark emits SoftBreak between lines; visible_text renders it as a space
        assert_eq!(visible_text("line one\nline two"), "line one line two");
    }

    #[test]
    fn visible_text_multiple_links() {
        assert_eq!(visible_text("[first](https://a.com) and [second](https://b.com)"), "first and second");
    }

    #[test]
    fn visible_text_bare_url() {
        // Bare URLs are plain text to the parser; they pass through unchanged
        assert_eq!(visible_text("see https://example.com for details"), "see https://example.com for details");
    }

    // ── merge_md_style ────────────────────────────────────────────────────────

    #[test]
    fn merge_md_style_bold() {
        let base = ColorSpec::new();
        let result = merge_md_style(&base, true, false, false);
        assert!(result.bold());
        assert!(!result.italic());
        assert!(!result.underline());
    }

    #[test]
    fn merge_md_style_italic() {
        let base = ColorSpec::new();
        let result = merge_md_style(&base, false, true, false);
        assert!(!result.bold());
        assert!(result.italic());
    }

    #[test]
    fn merge_md_style_underline() {
        let base = ColorSpec::new();
        let result = merge_md_style(&base, false, false, true);
        assert!(result.underline());
    }

    #[test]
    fn merge_md_style_all_flags() {
        let base = ColorSpec::new();
        let result = merge_md_style(&base, true, true, true);
        assert!(result.bold());
        assert!(result.italic());
        assert!(result.underline());
    }

    #[test]
    fn merge_md_style_preserves_base_color() {
        use termcolor::Color;
        let mut base = ColorSpec::new();
        base.set_fg(Some(Color::Red));
        let result = merge_md_style(&base, true, false, false);
        assert_eq!(result.fg(), Some(&Color::Red));
        assert!(result.bold());
    }

    // ── trim_url_punctuation ──────────────────────────────────────────────────

    #[test]
    fn trim_url_trailing_period() {
        let (url, trailing) = trim_url_punctuation("https://example.com.");
        assert_eq!(url, "https://example.com");
        assert_eq!(trailing, ".");
    }

    #[test]
    fn trim_url_trailing_paren_comma() {
        let (url, trailing) = trim_url_punctuation("https://example.com),");
        assert_eq!(url, "https://example.com");
        assert_eq!(trailing, "),");
    }

    #[test]
    fn trim_url_no_trailing_punctuation() {
        let (url, trailing) = trim_url_punctuation("https://example.com/path");
        assert_eq!(url, "https://example.com/path");
        assert_eq!(trailing, "");
    }

    #[test]
    fn trim_url_query_string() {
        let (url, trailing) = trim_url_punctuation("https://example.com/path?q=1");
        assert_eq!(url, "https://example.com/path?q=1");
        assert_eq!(trailing, "");
    }

    // ── print_markdown output tests ───────────────────────────────────────────

    fn make_conf() -> fmt::Conf {
        fmt::Conf { atty: false, ..Default::default() }
    }

    fn render_markdown(text: &str) -> String {
        let mut buf = Buffer::no_color();
        let base = ColorSpec::new();
        let c = make_conf();
        print_markdown(&mut buf, text, &base, &c).unwrap();
        String::from_utf8(buf.into_inner()).unwrap()
    }

    #[test]
    fn print_markdown_plain_text() {
        assert_eq!(render_markdown("hello world"), "hello world");
    }

    #[test]
    fn print_markdown_bold() {
        // Bold markers are consumed; visible text is the inner content
        assert_eq!(render_markdown("**bold**"), "bold");
    }

    #[test]
    fn print_markdown_italic() {
        assert_eq!(render_markdown("*italic*"), "italic");
    }

    #[test]
    fn print_markdown_code() {
        assert_eq!(render_markdown("`code`"), "code");
    }

    #[test]
    fn print_markdown_link() {
        // Link label is the visible text; dest_url is only in hyperlink escape
        assert_eq!(render_markdown("[label](https://example.com)"), "label");
    }

    #[test]
    fn print_markdown_mixed() {
        assert_eq!(render_markdown("**bold** and *italic*"), "bold and italic");
    }

    // ── print_markdown_range output tests ────────────────────────────────────

    fn render_range(text: &str, skip: usize, limit: usize) -> String {
        let mut buf = Buffer::no_color();
        let base = ColorSpec::new();
        let c = make_conf();
        print_markdown_range(&mut buf, text, skip, limit, &base, &c).unwrap();
        String::from_utf8(buf.into_inner()).unwrap()
    }

    #[test]
    fn print_markdown_range_full() {
        let text = "hello world";
        let full = render_markdown(text);
        let range = render_range(text, 0, full.chars().count());
        assert_eq!(full, range);
    }

    #[test]
    fn print_markdown_range_skip() {
        // Skip first 6 chars of "hello world" → "world"
        let result = render_range("hello world", 6, 5);
        assert_eq!(result, "world");
    }

    #[test]
    fn print_markdown_range_cut_through_formatted() {
        // "**bold** text" → visible "bold text"; take first 4 chars → "bold"
        let result = render_range("**bold** text", 0, 4);
        assert_eq!(result, "bold");
    }

    #[test]
    fn print_markdown_range_second_line() {
        // "bold text" visible; skip 4 → " text"
        let result = render_range("**bold** text", 4, 5);
        assert_eq!(result, " text");
    }
}
