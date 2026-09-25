use rowan::{NodeOrToken, ast::AstNode};
use std::cmp::min;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Write as _;

use super::TraversalContext;
use super::Traverser;
use super::event::{Container, Event};
use crate::{
    SyntaxElement, SyntaxKind, SyntaxNode,
    ast::{FnDef, FnRef},
};

/// A wrapper for escaping sensitive characters in html.
///
/// ```rust
/// use orgize::export::HtmlEscape as Escape;
///
/// assert_eq!(format!("{}", Escape("< < <")), "&lt; &lt; &lt;");
/// assert_eq!(
///     format!("{}", Escape("<script>alert('Hello XSS')</script>")),
///     "&lt;script&gt;alert(&apos;Hello XSS&apos;)&lt;/script&gt;"
/// );
/// ```
pub struct HtmlEscape<S: AsRef<str>>(pub S);

impl<S: AsRef<str>> fmt::Display for HtmlEscape<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut pos = 0;

        let content = self.0.as_ref();
        let bytes = content.as_bytes();

        while let Some(off) = jetscii::bytes!(b'<', b'>', b'&', b'\'', b'"').find(&bytes[pos..]) {
            write!(f, "{}", &content[pos..pos + off])?;

            pos += off + 1;

            match bytes[pos - 1] {
                b'<' => write!(f, "&lt;")?,
                b'>' => write!(f, "&gt;")?,
                b'&' => write!(f, "&amp;")?,
                b'\'' => write!(f, "&apos;")?,
                b'"' => write!(f, "&quot;")?,
                _ => {}
            }
        }

        write!(f, "{}", &content[pos..])
    }
}

// Export punctuation only in prose. Escaping each unchanged run before writing the
// entity also keeps author-supplied markup inert without rewriting Org source.
fn write_prose(output: &mut String, text: &str) {
    let mut remaining = text;
    while let Some(offset) = jetscii::bytes!(b'-', b'.').find(remaining.as_bytes()) {
        let candidate = &remaining[offset..];
        let (width, entity) = if candidate.starts_with("---") {
            (3, "&mdash;")
        } else if candidate.starts_with("--") {
            (2, "&ndash;")
        } else if candidate.starts_with("...") {
            (3, "&hellip;")
        } else {
            // A lone punctuation mark is part of the next unmodified run.
            let next = offset + 1;
            let _ = write!(output, "{}", HtmlEscape(&remaining[..next]));
            remaining = &remaining[next..];
            continue;
        };
        let _ = write!(output, "{}", HtmlEscape(&remaining[..offset]));
        output.push_str(entity);
        remaining = &candidate[width..];
    }
    let _ = write!(output, "{}", HtmlEscape(remaining));
}

#[derive(Default)]
pub struct HtmlExport {
    output: String,

    prose_depth: usize,
    literal_depth: usize,
    in_descriptive_list: Vec<bool>,

    table_row: TableRow,

    footnote_namespace: Option<i64>,
    definitions: HashMap<String, SyntaxNode>,
    notes: Vec<Footnote>,
}

struct Footnote {
    label: String,
    definition: SyntaxNode,
    references: usize,
}

#[derive(Default, PartialEq, Eq)]
enum TableRow {
    #[default]
    HeaderRule,
    Header,
    BodyRule,
    Body,
}

impl HtmlExport {
    /// Give links within a Post a stable, collision-free fragment namespace.
    /// The same Post ID must be supplied on creation, update, and rebuild.
    pub fn with_footnote_namespace(post_id: i64) -> Self {
        Self {
            footnote_namespace: Some(post_id),
            ..Self::default()
        }
    }

    fn label(node: &SyntaxNode) -> Option<String> {
        let mut children = node.children_with_tokens();
        children.find(|child| child.kind() == SyntaxKind::COLON)?;
        match children.next()? {
            NodeOrToken::Token(token) if token.kind() == SyntaxKind::TEXT => {
                Some(token.text().to_owned())
            }
            _ => None,
        }
    }

    fn inline_definition(node: &SyntaxNode) -> bool {
        node.children_with_tokens()
            .filter(|child| child.kind() == SyntaxKind::COLON)
            .count()
            > 1
    }

    fn note_id(&self, number: usize) -> String {
        match self.footnote_namespace {
            Some(id) => format!("post-{id}-fn-{number}"),
            None => format!("fn-{number}"),
        }
    }

    fn reference_id(&self, number: usize, occurrence: usize) -> String {
        match self.footnote_namespace {
            Some(id) => format!("post-{id}-fnref-{number}-{occurrence}"),
            None => format!("fnref-{number}-{occurrence}"),
        }
    }

    fn render_note_body(&mut self, node: &SyntaxNode, ctx: &mut TraversalContext) {
        let mut colons = 0;
        let mut content = false;
        for child in node.children_with_tokens() {
            if !content {
                if node.kind() == SyntaxKind::FN_DEF && child.kind() == SyntaxKind::R_BRACKET {
                    content = true;
                } else if node.kind() == SyntaxKind::FN_REF && child.kind() == SyntaxKind::COLON {
                    colons += 1;
                    content = colons == 2;
                }
                continue;
            }
            if node.kind() == SyntaxKind::FN_REF && child.kind() == SyntaxKind::R_BRACKET {
                break;
            }
            self.element(child, ctx);
        }
    }

    fn render_notes(&mut self, ctx: &mut TraversalContext) {
        if self.notes.is_empty() {
            return;
        }
        self.output += "<section class=\"footnotes\"><h2>Footnotes</h2><ol>";
        let mut index = 0;
        while index < self.notes.len() {
            let number = index + 1;
            let definition = self.notes[index].definition.clone();
            let note_id = self.note_id(number);
            let _ = write!(&mut self.output, "<li id=\"{note_id}\"><p>");
            self.render_note_body(&definition, ctx);
            self.output += "</p>";
            for occurrence in 1..=self.notes[index].references {
                let reference_id = self.reference_id(number, occurrence);
                let _ = write!(&mut self.output, " <a href=\"#{reference_id}\">↩</a>");
            }
            self.output += "</li>";
            index += 1;
        }
        self.output += "</ol></section>";
    }

    pub fn push_str(&mut self, s: impl AsRef<str>) {
        self.output += s.as_ref();
    }

    pub fn finish(self) -> String {
        self.output
    }

    /// Render syntax node to html string
    ///
    /// ```rust
    /// use orgize::{Org, ast::Bold, export::HtmlExport, rowan::ast::AstNode};
    ///
    /// let org = Org::parse("* /hello/ *world*");
    /// let bold = org.first_node::<Bold>().unwrap();
    /// let mut html = HtmlExport::default();
    /// html.render(bold.syntax());
    /// assert_eq!(html.finish(), "<b>world</b>");
    /// ```
    pub fn render(&mut self, node: &SyntaxNode) {
        let mut ctx = TraversalContext::default();
        self.element(SyntaxElement::Node(node.clone()), &mut ctx);
    }
}

impl Traverser for HtmlExport {
    fn event(&mut self, event: Event, ctx: &mut TraversalContext) {
        match event {
            Event::Enter(Container::Document(document)) => {
                self.definitions.clear();
                self.notes.clear();
                for definition in document.syntax().descendants().filter_map(FnDef::cast) {
                    if let Some(label) = Self::label(definition.syntax()) {
                        self.definitions
                            .entry(label)
                            .or_insert_with(|| definition.syntax().clone());
                    }
                }
                for reference in document.syntax().descendants().filter_map(FnRef::cast) {
                    if Self::inline_definition(reference.syntax()) {
                        if let Some(label) =
                            Self::label(reference.syntax()).filter(|label| !label.is_empty())
                        {
                            self.definitions
                                .entry(label)
                                .or_insert_with(|| reference.syntax().clone());
                        }
                    }
                }
                self.output += "<main>";
            }
            Event::Leave(Container::Document(_)) => {
                self.render_notes(ctx);
                self.output += "</main>";
            }

            Event::Enter(Container::FnDef(_)) => ctx.skip(),
            Event::Enter(Container::FnRef(reference)) => {
                let node = reference.syntax();
                let inline = Self::inline_definition(node);
                let label = Self::label(node).unwrap_or_default();
                let label = if label.is_empty() && inline {
                    format!("anonymous-{}", u32::from(node.text_range().start()))
                } else {
                    label
                };
                let definition = if inline {
                    Some(node.clone())
                } else {
                    self.definitions.get(&label).cloned()
                };
                if let Some(definition) = definition {
                    let index = self
                        .notes
                        .iter()
                        .position(|note| note.label == label)
                        .unwrap_or_else(|| {
                            self.notes.push(Footnote {
                                label,
                                definition,
                                references: 0,
                            });
                            self.notes.len() - 1
                        });
                    self.notes[index].references += 1;
                    let number = index + 1;
                    let occurrence = self.notes[index].references;
                    let reference_id = self.reference_id(number, occurrence);
                    let note_id = self.note_id(number);
                    let _ = write!(
                        &mut self.output,
                        "<sup class=\"footnote-reference\" id=\"{reference_id}\"><a href=\"#{note_id}\">{number}</a></sup>"
                    );
                } else {
                    let _ = write!(&mut self.output, "{}", HtmlEscape(reference.raw()));
                }
                ctx.skip();
            }

            Event::Enter(Container::Headline(headline)) => {
                self.prose_depth += 1;
                let level = min(headline.level(), 6);
                let _ = write!(&mut self.output, "<h{level}>");
                for elem in headline.title() {
                    self.element(elem, ctx);
                }
                let _ = write!(&mut self.output, "</h{level}>");
            }
            Event::Leave(Container::Headline(_)) => self.prose_depth -= 1,

            Event::Enter(Container::Paragraph(_)) => {
                self.prose_depth += 1;
                self.output += "<p>";
            }
            Event::Leave(Container::Paragraph(_)) => {
                self.prose_depth -= 1;
                self.output += "</p>";
            }

            Event::Enter(Container::Section(_)) => self.output += "<section>",
            Event::Leave(Container::Section(_)) => self.output += "</section>",

            Event::Enter(Container::Italic(_)) => self.output += "<i>",
            Event::Leave(Container::Italic(_)) => self.output += "</i>",

            Event::Enter(Container::Bold(_)) => self.output += "<b>",
            Event::Leave(Container::Bold(_)) => self.output += "</b>",

            Event::Enter(Container::Strike(_)) => self.output += "<s>",
            Event::Leave(Container::Strike(_)) => self.output += "</s>",

            Event::Enter(Container::Underline(_)) => self.output += "<u>",
            Event::Leave(Container::Underline(_)) => self.output += "</u>",

            Event::Enter(Container::Verbatim(_) | Container::Code(_)) => {
                self.literal_depth += 1;
                self.output += "<code>";
            }
            Event::Leave(Container::Verbatim(_) | Container::Code(_)) => {
                self.literal_depth -= 1;
                self.output += "</code>";
            }

            Event::Enter(Container::SourceBlock(block)) => {
                self.literal_depth += 1;
                if let Some(language) = block.language() {
                    let _ = write!(
                        &mut self.output,
                        r#"<pre><code class="language-{}">"#,
                        HtmlEscape(&language)
                    );
                } else {
                    self.output += r#"<pre><code>"#
                }
            }
            Event::Leave(Container::SourceBlock(_)) => {
                self.literal_depth -= 1;
                self.output += "</code></pre>";
            }

            Event::Enter(Container::QuoteBlock(_)) => self.output += "<blockquote>",
            Event::Leave(Container::QuoteBlock(_)) => self.output += "</blockquote>",

            Event::Enter(Container::VerseBlock(_)) => {
                self.prose_depth += 1;
                self.output += "<p class=\"verse\">";
            }
            Event::Leave(Container::VerseBlock(_)) => {
                self.prose_depth -= 1;
                self.output += "</p>";
            }

            Event::Enter(Container::ExampleBlock(_)) => {
                self.literal_depth += 1;
                self.output += "<pre class=\"example\">";
            }
            Event::Leave(Container::ExampleBlock(_)) => {
                self.literal_depth -= 1;
                self.output += "</pre>";
            }

            Event::Enter(Container::CenterBlock(_)) => self.output += "<div class=\"center\">",
            Event::Leave(Container::CenterBlock(_)) => self.output += "</div>",

            Event::Enter(Container::CommentBlock(_) | Container::Comment(_)) => {
                self.literal_depth += 1;
                self.output += "<!--";
            }
            Event::Leave(Container::CommentBlock(_) | Container::Comment(_)) => {
                self.literal_depth -= 1;
                self.output += "-->";
            }

            Event::Enter(Container::FixedWidth(_) | Container::ExportBlock(_)) => {
                self.literal_depth += 1;
            }
            Event::Leave(Container::FixedWidth(_) | Container::ExportBlock(_)) => {
                self.literal_depth -= 1;
            }

            Event::Enter(Container::Subscript(_)) => self.output += "<sub>",
            Event::Leave(Container::Subscript(_)) => self.output += "</sub>",

            Event::Enter(Container::Superscript(_)) => self.output += "<sup>",
            Event::Leave(Container::Superscript(_)) => self.output += "</sup>",

            Event::Enter(Container::List(list)) => {
                self.output += if list.is_ordered() {
                    self.in_descriptive_list.push(false);
                    "<ol>"
                } else if list.is_descriptive() {
                    self.in_descriptive_list.push(true);
                    "<dl>"
                } else {
                    self.in_descriptive_list.push(false);
                    "<ul>"
                };
            }
            Event::Leave(Container::List(list)) => {
                self.output += if list.is_ordered() {
                    "</ol>"
                } else if let Some(true) = self.in_descriptive_list.last() {
                    "</dl>"
                } else {
                    "</ul>"
                };
                self.in_descriptive_list.pop();
            }
            Event::Enter(Container::ListItem(list_item)) => {
                self.prose_depth += 1;
                if let Some(&true) = self.in_descriptive_list.last() {
                    self.output += "<dt>";
                    for elem in list_item.tag() {
                        self.element(elem, ctx);
                    }
                    self.output += "</dt><dd>";
                } else {
                    self.output += "<li>";
                }
            }
            Event::Leave(Container::ListItem(_)) => {
                self.prose_depth -= 1;
                if let Some(&true) = self.in_descriptive_list.last() {
                    self.output += "</dd>";
                } else {
                    self.output += "</li>";
                }
            }

            Event::Enter(Container::OrgTable(table)) => {
                self.output += "<table>";
                self.table_row = if table.has_header() {
                    TableRow::HeaderRule
                } else {
                    TableRow::BodyRule
                }
            }
            Event::Leave(Container::OrgTable(_)) => {
                match self.table_row {
                    TableRow::Body => self.output += "</tbody>",
                    TableRow::Header => self.output += "</thead>",
                    _ => {}
                }
                self.output += "</table>";
            }
            Event::Enter(Container::OrgTableRow(row)) => {
                if row.is_rule() {
                    match self.table_row {
                        TableRow::Body => {
                            self.output += "</tbody>";
                            self.table_row = TableRow::BodyRule;
                        }
                        TableRow::Header => {
                            self.output += "</thead>";
                            self.table_row = TableRow::BodyRule;
                        }
                        _ => {}
                    }
                    ctx.skip();
                } else {
                    match self.table_row {
                        TableRow::HeaderRule => {
                            self.table_row = TableRow::Header;
                            self.output += "<thead>";
                        }
                        TableRow::BodyRule => {
                            self.table_row = TableRow::Body;
                            self.output += "<tbody>";
                        }
                        _ => {}
                    }
                    self.output += "<tr>";
                }
            }
            Event::Leave(Container::OrgTableRow(row)) => {
                if row.is_rule() {
                    match self.table_row {
                        TableRow::Body => {
                            self.output += "</tbody>";
                            self.table_row = TableRow::BodyRule;
                        }
                        TableRow::Header => {
                            self.output += "</thead>";
                            self.table_row = TableRow::BodyRule;
                        }
                        _ => {}
                    }
                    ctx.skip();
                } else {
                    self.output += "</tr>";
                }
            }
            Event::Enter(Container::OrgTableCell(_)) => {
                self.prose_depth += 1;
                self.output += "<td>";
            }
            Event::Leave(Container::OrgTableCell(_)) => {
                self.prose_depth -= 1;
                self.output += "</td>";
            }

            Event::Enter(Container::Link(link)) => {
                let path = link.path();
                let path = path.trim_start_matches("file:");

                if link.is_image() {
                    let _ = write!(&mut self.output, r#"<img src="{}">"#, HtmlEscape(&path));
                    return ctx.skip();
                }

                let _ = write!(&mut self.output, r#"<a href="{}">"#, HtmlEscape(&path));

                if !link.has_description() {
                    let _ = write!(&mut self.output, "{}</a>", HtmlEscape(&path));
                    ctx.skip();
                }
            }
            Event::Leave(Container::Link(_)) => self.output += "</a>",

            Event::Text(text) => {
                if self.prose_depth > 0 && self.literal_depth == 0 {
                    write_prose(&mut self.output, &text.to_string());
                } else {
                    let _ = write!(&mut self.output, "{}", HtmlEscape(text));
                }
            }

            Event::LineBreak(_) => self.output += "<br/>",

            Event::Snippet(snippet) => {
                if snippet.backend().eq_ignore_ascii_case("html") {
                    self.output += &snippet.value();
                }
            }

            Event::Rule(_) => self.output += "<hr/>",

            Event::Timestamp(timestamp) => {
                self.output += r#"<span class="timestamp-wrapper"><span class="timestamp">"#;
                for e in timestamp.syntax.children_with_tokens() {
                    match e {
                        NodeOrToken::Token(t) if t.kind() == SyntaxKind::MINUS2 => {
                            self.output += "&#x2013;";
                        }
                        NodeOrToken::Token(t) => {
                            self.output += t.text();
                        }
                        _ => {}
                    }
                }
                self.output += r#"</span></span>"#;
            }

            Event::LatexFragment(latex) => {
                let _ = write!(&mut self.output, "{}", &latex.syntax);
            }
            Event::LatexEnvironment(latex) => {
                let _ = write!(&mut self.output, "{}", &latex.syntax);
            }

            // ignores keyword
            Event::Enter(Container::Keyword(_)) => ctx.skip(),

            Event::Entity(entity) => self.output += entity.html(),

            _ => {}
        }
    }
}
