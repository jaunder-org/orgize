use orgize::Org;

#[test]
fn verse_preserves_layout_inline_objects_and_source() {
    let source = "#+begin_verse\nFirst *bold* line.\n  Second [[https://example.org][linked]] line.\n    Third line.\n\nLast line.\n#+end_verse";
    let org = Org::parse(source);
    assert_eq!(org.to_org(), source);
    let html = org.to_html();
    assert!(
        html.contains("<p class=\"verse\">First <b>bold</b> line.<br/>"),
        "{html}"
    );
    assert!(
        html.contains("&nbsp;&nbsp;Second <a href=\"https://example.org\">linked</a> line.<br/>"),
        "{html}"
    );
    assert!(
        html.contains("&nbsp;&nbsp;&nbsp;&nbsp;Third line.<br/><br/>Last line.<br/></p>"),
        "{html}"
    );
    assert!(!html.contains("<pre"), "{html}");
}

#[test]
fn verse_removes_common_indent_but_keeps_tabs_and_blank_lines() {
    let source = "#+begin_verse\r\n\r\n  *first*\r\n\t/second/\r\n  \r\n  last\r\n#+end_verse";
    let org = Org::parse(source);
    assert_eq!(org.to_org(), source);
    assert_eq!(
        org.to_html(),
        "<main><section><p class=\"verse\"><br/><b>first</b><br/>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<i>second</i><br/><br/>last<br/></p></section></main>"
    );
}

#[test]
fn verse_parses_cross_line_objects_without_rewriting_literals() {
    let source = "#+begin_verse\n*first\nsecond* and [[https://example.org/a--b][linked\nlabel]]\n~literal\ncode -- ...~ and =verbatim -- ...=\nprose -- ... & < >\n#+end_verse";
    let org = Org::parse(source);
    assert_eq!(org.to_org(), source);
    let html = org.to_html();
    assert!(html.contains("<b>first<br/>second</b>"), "{html}");
    assert!(
        html.contains("<a href=\"https://example.org/a--b\">linked<br/>label</a>"),
        "{html}"
    );
    assert!(
        html.contains("<code>literal\ncode -- ...</code> and <code>verbatim -- ...</code>"),
        "{html}"
    );
    assert!(
        html.contains("prose &ndash; &hellip; &amp; &lt; &gt;<br/>"),
        "{html}"
    );
}

#[test]
fn verse_footnotes_have_document_owned_identity() {
    let source = "Before[fn::outside before].\n\n#+begin_verse\nFirst[fn::inside one].\nSecond[fn::inside two].\n#+end_verse\n\nAfter[fn::outside after].";
    let org = Org::parse(source);
    assert_eq!(org.to_org(), source);
    let html = org.to_html();
    for (number, body) in [
        (1, "outside before"),
        (2, "inside one"),
        (3, "inside two"),
        (4, "outside after"),
    ] {
        assert_eq!(
            html.matches(&format!("id=\"fn-{number}\"")).count(),
            1,
            "{html}"
        );
        assert_eq!(
            html.matches(&format!("id=\"fnref-{number}-1\"")).count(),
            1,
            "{html}"
        );
        assert!(html.contains(body), "{html}");
    }
}

#[test]
fn verse_does_not_parse_block_syntax_or_shortcodes() {
    let source = "#+begin_verse\n- a list-looking line\n#+begin_quote\n| a | table |\n{{< youtube dQw4w9WgXcQ >}}\n#+end_quote\n#+end_verse";
    let org = Org::parse(source);
    assert_eq!(org.to_org(), source);
    let html = org.to_html();
    assert!(
        html.contains("- a list-looking line<br/>#+begin<sub>quote</sub><br/>| a | table |<br/>"),
        "{html}"
    );
    assert!(
        html.contains("{{&lt; youtube dQw4w9WgXcQ &gt;}}<br/>"),
        "{html}"
    );
    assert!(!html.contains("<iframe"), "{html}");
    assert!(!html.contains("<ul>"), "{html}");
    assert!(!html.contains("<blockquote>"), "{html}");
    assert!(!html.contains("<table>"), "{html}");
}

#[test]
fn verse_unescapes_comma_quoted_lines_without_rewriting_source() {
    let source = "#+begin_verse\n,* a headline-looking line\n,#+end_verse\n*bold* and a,#+midline comma\n#+end_verse";
    let org = Org::parse(source);
    assert_eq!(org.to_org(), source);
    let html = org.to_html();
    assert!(
        html.contains("* a headline-looking line<br/>#+end<sub>verse</sub><br/>"),
        "{html}"
    );
    assert!(
        html.contains("<b>bold</b> and a,#+midline comma<br/>"),
        "{html}"
    );
}

#[test]
fn verse_context_reaches_nested_inline_objects() {
    for marker in ['*', '/', '+', '_'] {
        let source = format!(
            "#+begin_verse\n{marker}{{{{< youtube id >}}}}{marker}\n{marker}first\n,#+plain{marker}\n#+end_verse"
        );
        let org = Org::parse(&source);
        assert_eq!(org.to_org(), source);
        let html = org.to_html();
        assert!(html.contains("{{&lt; youtube id &gt;}}"), "{html}");
        assert!(html.contains("first<br/>#+plain"), "{html}");
        assert!(!html.contains(",#+plain"), "{html}");
    }
    let source =
        "#+begin_verse\n[[https://example.org][first\n,#+plain]]\n*,#+midline*\n#+end_verse";
    let org = Org::parse(source);
    assert_eq!(org.to_org(), source);
    let html = org.to_html();
    assert!(html.contains("first<br/>#+plain</a>"), "{html}");
    assert!(html.contains("<b>,#+midline</b>"), "{html}");
}

#[test]
fn verse_and_quote_accept_mixed_case_delimiters() {
    for block in ["verse", "quote"] {
        for (begin, end) in [
            (block.to_uppercase(), block.to_owned()),
            (block.to_owned(), block.to_uppercase()),
        ] {
            let source = format!("#+BEGIN_{begin}\n*bold*\n#+end_{end}");
            let org = Org::parse(&source);
            assert_eq!(org.to_org(), source);
            let html = org.to_html();
            assert!(html.contains("<b>bold</b>"), "{html}");
            assert!(!html.contains("#+"), "{html}");
            assert!(
                html.contains(if block == "verse" {
                    "<p class=\"verse\">"
                } else {
                    "<blockquote>"
                }),
                "{html}"
            );
        }
    }
}

#[test]
fn quote_keeps_multiple_paragraphs_and_inline_objects() {
    let source = "#+begin_quote\nFirst *bold* paragraph.\n\nSecond [[https://example.org][linked]] paragraph.\n#+end_quote";
    let org = Org::parse(source);
    assert_eq!(org.to_org(), source);
    assert_eq!(
        org.to_html(),
        "<main><section><blockquote><p>First <b>bold</b> paragraph.\n</p><p>Second <a href=\"https://example.org\">linked</a> paragraph.\n</p></blockquote></section></main>"
    );
}

#[test]
fn verse_layout_does_not_leak_to_surrounding_prose_or_next_verse() {
    let source = "Before\nprose.\n\n#+begin_verse\n  one\n    two\n#+end_verse\n\nAfter\nprose.\n\n#+begin_verse\nthree\n#+end_verse";
    let html = Org::parse(source).to_html();
    assert!(html.contains("<p>Before\nprose.\n</p>"), "{html}");
    assert!(html.contains("<p>After\nprose.\n</p>"), "{html}");
    assert!(html.contains("<p class=\"verse\">three<br/></p>"), "{html}");
}
