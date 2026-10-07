use nom::{
    IResult, InputTake,
    bytes::complete::{tag, take_while1},
    sequence::tuple,
};

use super::{
    SyntaxKind,
    combinator::{
        GreenElement, NodeBuilder, blank_lines, colon_token, l_bracket_token, line_ends_iter,
        r_bracket_token,
    },
    input::Input,
    keyword::affiliated_keyword_nodes,
    object::standard_object_nodes,
};

#[cfg_attr(
  feature = "tracing",
  tracing::instrument(level = "debug", skip(input), fields(input = input.s))
)]
pub fn fn_def_node(input: Input) -> IResult<Input, GreenElement, ()> {
    let (remaining, (affiliated_keywords, l_bracket, fn_, colon, label, r_bracket)) = tuple((
        affiliated_keyword_nodes,
        l_bracket_token,
        tag("fn"),
        colon_token,
        take_while1(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        r_bracket_token,
    ))(input)?;
    // Continue a definition until a blank line or the next definition. Parse
    // its contents with the ordinary Org object parser, not as opaque text.
    let mut end = 0;
    for idx in line_ends_iter(remaining.as_str()) {
        let line = &remaining.as_str()[end..idx];
        if end > 0 && (line.trim().is_empty() || line.starts_with("[fn:")) {
            break;
        }
        end = idx;
    }
    let (remaining, contents) = remaining.take_split(end);
    let (remaining, post_blank) = blank_lines(remaining)?;
    let mut b = NodeBuilder::new();
    b.children.extend(affiliated_keywords);
    b.push(l_bracket);
    b.text(fn_);
    b.push(colon);
    b.text(label);
    b.push(r_bracket);
    b.children.extend(standard_object_nodes(contents));
    b.children.extend(post_blank);
    Ok((remaining, b.finish(SyntaxKind::FN_DEF)))
}

#[test]
fn parse() {
    use crate::ParseConfig;
    use crate::{ast::FnDef, tests::to_ast};

    let to_fn_def = to_ast::<FnDef>(fn_def_node);

    insta::assert_debug_snapshot!(
         to_fn_def("[fn:1] https://orgmode.org").syntax,
         @r###"
    FN_DEF@0..26
      L_BRACKET@0..1 "["
      TEXT@1..3 "fn"
      COLON@3..4 ":"
      TEXT@4..5 "1"
      R_BRACKET@5..6 "]"
      TEXT@6..26 " https://orgmode.org"
    "###
    );

    insta::assert_debug_snapshot!(
         to_fn_def("[fn:word_1] https://orgmode.org").syntax,
         @r###"
    FN_DEF@0..31
      L_BRACKET@0..1 "["
      TEXT@1..3 "fn"
      COLON@3..4 ":"
      TEXT@4..10 "word_1"
      R_BRACKET@10..11 "]"
      TEXT@11..31 " https://orgmode.org"
    "###
    );

    insta::assert_debug_snapshot!(
         to_fn_def("[fn:WORD-1] https://orgmode.org").syntax,
         @r###"
    FN_DEF@0..31
      L_BRACKET@0..1 "["
      TEXT@1..3 "fn"
      COLON@3..4 ":"
      TEXT@4..10 "WORD-1"
      R_BRACKET@10..11 "]"
      TEXT@11..31 " https://orgmode.org"
    "###
    );

    insta::assert_debug_snapshot!(
         to_fn_def("[fn:WORD]").syntax,
         @r###"
    FN_DEF@0..9
      L_BRACKET@0..1 "["
      TEXT@1..3 "fn"
      COLON@3..4 ":"
      TEXT@4..8 "WORD"
      R_BRACKET@8..9 "]"
    "###
    );

    insta::assert_debug_snapshot!(
         to_fn_def("[fn:1] In particular, the parser requires stars at column 0 to be\n").syntax,
         @r###"
    FN_DEF@0..66
      L_BRACKET@0..1 "["
      TEXT@1..3 "fn"
      COLON@3..4 ":"
      TEXT@4..5 "1"
      R_BRACKET@5..6 "]"
      TEXT@6..66 " In particular, the p ..."
    "###
    );

    let config = &ParseConfig::default();

    assert!(fn_def_node(("[fn:] https://orgmode.org", config).into()).is_err());
    assert!(fn_def_node(("[fn:wor d] https://orgmode.org", config).into()).is_err());
    assert!(fn_def_node(("[fn:WORD https://orgmode.org", config).into()).is_err());

    insta::assert_debug_snapshot!(
         to_fn_def("#+ATTR_poi: 1\n[fn:WORD-1] https://orgmode.org").syntax,
         @r###"
    FN_DEF@0..45
      AFFILIATED_KEYWORD@0..14
        HASH_PLUS@0..2 "#+"
        TEXT@2..10 "ATTR_poi"
        COLON@10..11 ":"
        TEXT@11..13 " 1"
        NEW_LINE@13..14 "\n"
      L_BRACKET@14..15 "["
      TEXT@15..17 "fn"
      COLON@17..18 ":"
      TEXT@18..24 "WORD-1"
      R_BRACKET@24..25 "]"
      TEXT@25..45 " https://orgmode.org"
    "###
    );
}
