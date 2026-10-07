use orgize::Org;

#[test]
fn special_strings_in_prose_and_nested_inline_markup() {
    let source = "A long---pause, short--pause, and then... *bold---pause* /italic--pause/ <tag>";
    let org = Org::parse(source);
    assert_eq!(org.to_org(), source);
    assert_eq!(
        org.to_html(),
        "<main><section><p>A long&mdash;pause, short&ndash;pause, and then&hellip; <b>bold&mdash;pause</b> <i>italic&ndash;pause</i> &lt;tag&gt;</p></section></main>"
    );
}

#[test]
fn special_strings_in_headlines() {
    let source = "* Head---tail -- middle ... *bold--word*";
    let org = Org::parse(source);
    assert_eq!(org.to_org(), source);
    assert_eq!(
        org.to_html(),
        "<main><h1>Head&mdash;tail &ndash; middle &hellip; <b>bold&ndash;word</b></h1></main>"
    );
}

#[test]
fn standalone_post_title_uses_prose_export_without_rewriting_source() {
    // Jaunder renders a Post title as a standalone Org document, not a headline.
    let source = "Three---two--one... *bold--word*";
    let org = Org::parse(source);
    assert_eq!(org.to_org(), source);
    assert_eq!(
        org.to_html(),
        "<main><section><p>Three&mdash;two&ndash;one&hellip; <b>bold&ndash;word</b></p></section></main>"
    );
}

#[test]
fn literal_contexts_and_link_destinations_remain_source_exact() {
    let source = "~code---...~ =verbatim--...= [[https://example.org/a---b?q=x--y][label---text]]\n\n#+begin_src text\ncode--- -- ...\n#+end_src";
    let org = Org::parse(source);
    assert_eq!(org.to_org(), source);
    let html = org.to_html();
    assert!(html.contains("<code>code---...</code>"), "{html}");
    assert!(html.contains("<code>verbatim--...</code>"), "{html}");
    assert!(
        html.contains("href=\"https://example.org/a---b?q=x--y\""),
        "{html}"
    );
    assert!(html.contains(">label&mdash;text</a>"), "{html}");
    assert!(
        html.contains("<code class=\"language-text\">code--- -- ..."),
        "{html}"
    );
}

#[test]
fn nested_list_literal_blocks_do_not_export_special_strings() {
    let source = "- prose---text\n  #+begin_comment\n  comment--- -- ...\n  #+end_comment\n  #+begin_example\n  example--- -- ...\n  #+end_example\n  : fixed--- -- ...\n  #+begin_export html\n  export--- -- ...\n  #+end_export";
    let org = Org::parse(source);
    assert_eq!(org.to_org(), source);
    let html = org.to_html();
    assert!(html.contains("prose&mdash;text"), "{html}");
    for literal in [
        "comment--- -- ...",
        "example--- -- ...",
        "fixed--- -- ...",
        "export--- -- ...",
    ] {
        assert!(html.contains(literal), "missing literal {literal}: {html}");
    }
}
