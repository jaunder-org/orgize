//! test utils

use nom::IResult;
use rowan::{SyntaxNode, ast::AstNode};

use crate::{
    ParseConfig,
    syntax::{combinator::GreenElement, input::Input},
};

pub fn to_ast<N: AstNode>(
    parser: impl Fn(Input) -> IResult<Input, GreenElement, ()>,
) -> impl Fn(&str) -> N {
    move |s: &str| {
        let config = ParseConfig::default();
        let element = parser((s, &config).into()).unwrap().1;
        let node = element.into_node().unwrap();
        let node = SyntaxNode::<N::Language>::new_root(node);
        AstNode::cast(node).unwrap()
    }
}
