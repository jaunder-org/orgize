use orgize::{Org, export::HtmlExport};

fn render(source: &str, post_id: i64) -> String {
    let org = Org::parse(source);
    let mut export = HtmlExport::with_footnote_namespace(post_id);
    org.traverse(&mut export);
    export.finish()
}

#[test]
fn named_references_render_notes_with_post_scoped_links_and_backlinks() {
    let source = "first[fn:1], again[fn:1], second[fn:2].\n\n[fn:1] linked [[https://example.org][source]]\ncontinued /here/.\n\n[fn:2] another note";
    let html = render(source, 42);
    assert!(html.contains("id=\"post-42-fnref-1-1\""), "{html}");
    assert!(html.contains("id=\"post-42-fnref-1-2\""), "{html}");
    assert!(html.contains("href=\"#post-42-fn-1\""), "{html}");
    assert!(html.contains("id=\"post-42-fn-1\""), "{html}");
    assert!(html.contains("href=\"#post-42-fnref-1-1\""), "{html}");
    assert!(html.contains("href=\"#post-42-fnref-1-2\""), "{html}");
    assert!(html.contains("href=\"#post-42-fn-2\""), "{html}");
    assert!(
        html.contains("<a href=\"https://example.org\">source</a>"),
        "{html}"
    );
    assert!(html.contains("continued <i>here</i>"), "{html}");
    assert!(!html.contains("[fn:1]"), "{html}");
}

#[test]
fn numbered_by_first_use_even_when_definitions_appear_in_reverse_order() {
    let source = "later[fn:second] earlier[fn:first].\n\n[fn:first] first definition\n\n[fn:second] second definition";
    let html = render(source, 10);
    assert!(html.contains("href=\"#post-10-fn-1\">1</a>"), "{html}");
    assert!(html.contains("href=\"#post-10-fn-2\">2</a>"), "{html}");
    assert!(
        html.find("second definition").unwrap() < html.find("first definition").unwrap(),
        "{html}"
    );
}

#[test]
fn inline_notes_and_named_inline_reuse_render_once_each() {
    let source =
        "anon[fn::some /emphasis/] named[fn:key:with [[https://example.org][link]]] again[fn:key]";
    let html = render(source, 11);
    assert!(html.contains("some <i>emphasis</i>"), "{html}");
    assert!(
        html.contains("with <a href=\"https://example.org\">link</a>"),
        "{html}"
    );
    assert_eq!(html.matches("id=\"post-11-fn-2\"").count(), 1, "{html}");
    assert!(html.contains("id=\"post-11-fnref-2-2\""), "{html}");
}

#[test]
fn unresolved_and_unused_notes_do_not_make_dangling_links_or_drop_source() {
    let source = "unknown[fn:missing].\n\n[fn:unused] not referenced";
    let html = render(source, 12);
    assert!(html.contains("unknown[fn:missing]"), "{html}");
    assert!(!html.contains("not referenced"), "{html}");
    assert!(!html.contains("class=\"footnotes\""), "{html}");
    assert_eq!(Org::parse(source).to_org(), source);
}

#[test]
fn literal_contexts_do_not_authorize_footnotes() {
    let source = "#+begin_src text\n[fn:source]\n#+end_src\n\n=[fn:code]= and a real[fn:real].\n\n[fn:real] definition";
    let html = render(source, 13);
    assert!(html.contains("[fn:source]"), "{html}");
    assert!(html.contains("<code>[fn:code]</code>"), "{html}");
    assert!(html.contains("href=\"#post-13-fn-1\""), "{html}");
    assert!(!html.contains("fn-2"), "{html}");
}

#[test]
fn post_identity_prevents_cross_post_fragment_collisions() {
    let source = "reference[fn:1]\n\n[fn:1] note";
    let one = render(source, 21);
    let two = render(source, 22);
    assert!(one.contains("id=\"post-21-fn-1\""));
    assert!(two.contains("id=\"post-22-fn-1\""));
    assert!(!two.contains("post-21-fn-1"));
}

#[test]
fn production_footnote_shape_keeps_wrapped_org_link_and_second_note() {
    let source = "different behavior[fn:1]\n\nchange it back[fn:2]\n\n[fn:1] I am amused to find that [[https://example.org][no less than Mark Crispin, developer\nof the IMAP protocol]] agrees with me.\n\n[fn:2] This is, ultimately, one of the reasons I enjoy emacs---more\nthan any other editor.";
    let html = render(source, 869);
    assert!(html.contains("id=\"post-869-fn-1\""), "{html}");
    assert!(html.contains("id=\"post-869-fn-2\""), "{html}");
    assert!(
        html.contains("developer\nof the IMAP protocol</a>"),
        "{html}"
    );
    assert!(html.contains("than any other editor"), "{html}");
}
