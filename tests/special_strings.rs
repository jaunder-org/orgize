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
