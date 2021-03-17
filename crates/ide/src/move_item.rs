use std::iter::once;

use hir::Semantics;
use ide_db::{base_db::FileRange, RootDatabase};
use syntax::{algo, AstNode, NodeOrToken, SyntaxElement, SyntaxKind, SyntaxNode};
use text_edit::{TextEdit, TextEditBuilder};

pub enum Direction {
    Up,
    Down,
}

// Feature: Move Item
//
// Move item under cursor or selection up and down.
//
// |===
// | Editor  | Action Name
//
// | VS Code | **Rust Analyzer: Move item up**
// | VS Code | **Rust Analyzer: Move item down**
// |===
pub(crate) fn move_item(
    db: &RootDatabase,
    range: FileRange,
    direction: Direction,
) -> Option<TextEdit> {
    let sema = Semantics::new(db);
    let file = sema.parse(range.file_id);

    let item = file.syntax().covering_element(range.range);
    find_ancestors(item, direction)
}

fn find_ancestors(item: SyntaxElement, direction: Direction) -> Option<TextEdit> {
    let root = match item {
        NodeOrToken::Node(node) => node,
        NodeOrToken::Token(token) => token.parent()?,
    };

    let movable = [
        SyntaxKind::MATCH_ARM,
        SyntaxKind::PARAM,
        SyntaxKind::LET_STMT,
        SyntaxKind::EXPR_STMT,
        SyntaxKind::MATCH_EXPR,
        SyntaxKind::MACRO_CALL,
        SyntaxKind::TYPE_ALIAS,
        SyntaxKind::TRAIT,
        SyntaxKind::IMPL,
        SyntaxKind::MACRO_DEF,
        SyntaxKind::STRUCT,
        SyntaxKind::UNION,
        SyntaxKind::ENUM,
        SyntaxKind::FN,
        SyntaxKind::MODULE,
        SyntaxKind::USE,
        SyntaxKind::STATIC,
        SyntaxKind::CONST,
        SyntaxKind::MACRO_RULES,
    ];

    let ancestor = once(root.clone())
        .chain(root.ancestors())
        .find(|ancestor| movable.contains(&ancestor.kind()))?;

    move_in_direction(&ancestor, direction)
}

fn move_in_direction(node: &SyntaxNode, direction: Direction) -> Option<TextEdit> {
    let sibling = match direction {
        Direction::Up => node.prev_sibling(),
        Direction::Down => node.next_sibling(),
    }?;

    Some(replace_nodes(node, &sibling))
}

fn replace_nodes(first: &SyntaxNode, second: &SyntaxNode) -> TextEdit {
    let mut edit = TextEditBuilder::default();

    algo::diff(first, second).into_text_edit(&mut edit);
    algo::diff(second, first).into_text_edit(&mut edit);

    edit.finish()
}

#[cfg(test)]
mod tests {
    use crate::fixture;
    use expect_test::{expect, Expect};

    use crate::Direction;

    fn check(ra_fixture: &str, expect: Expect, direction: Direction) {
        let (analysis, range) = fixture::range(ra_fixture);
        let edit = analysis.move_item(range, direction).unwrap().unwrap_or_default();
        let mut file = analysis.file_text(range.file_id).unwrap().to_string();
        edit.apply(&mut file);
        expect.assert_eq(&file);
    }

    #[test]
    fn test_moves_match_arm_up() {
        check(
            r#"
fn main() {
    match true {
        true => {
            println!("Hello, world");
        },
        false =>$0$0 {
            println!("Test");
        }
    };
}
            "#,
            expect![[r#"
fn main() {
    match true {
        false => {
            println!("Test");
        },
        true => {
            println!("Hello, world");
        }
    };
}
            "#]],
            Direction::Up,
        );
    }

    #[test]
    fn test_moves_match_arm_down() {
        check(
            r#"
fn main() {
    match true {
        true =>$0$0 {
            println!("Hello, world");
        },
        false => {
            println!("Test");
        }
    };
}
            "#,
            expect![[r#"
fn main() {
    match true {
        false => {
            println!("Test");
        },
        true => {
            println!("Hello, world");
        }
    };
}
            "#]],
            Direction::Down,
        );
    }

    #[test]
    fn test_nowhere_to_move() {
        check(
            r#"
fn main() {
    match true {
        true =>$0$0 {
            println!("Hello, world");
        },
        false => {
            println!("Test");
        }
    };
}
            "#,
            expect![[r#"
fn main() {
    match true {
        true => {
            println!("Hello, world");
        },
        false => {
            println!("Test");
        }
    };
}
            "#]],
            Direction::Up,
        );
    }

    #[test]
    fn test_moves_let_stmt_up() {
        check(
            r#"
fn main() {
    let test = 123;
    let test2$0$0 = 456;
}
            "#,
            expect![[r#"
fn main() {
    let test2 = 456;
    let test = 123;
}
            "#]],
            Direction::Up,
        );
    }

    #[test]
    fn test_moves_expr_up() {
        check(
            r#"
fn main() {
    println!("Hello, world");
    println!("All I want to say is...");$0$0
}
            "#,
            expect![[r#"
fn main() {
    println!("All I want to say is...");
    println!("Hello, world");
}
            "#]],
            Direction::Up,
        );
    }

    #[test]
    fn test_nowhere_to_move_stmt() {
        check(
            r#"
fn main() {
    println!("All I want to say is...");$0$0
    println!("Hello, world");
}
            "#,
            expect![[r#"
fn main() {
    println!("All I want to say is...");
    println!("Hello, world");
}
            "#]],
            Direction::Up,
        );
    }

    #[test]
    fn test_move_item() {
        check(
            r#"
fn main() {}

fn foo() {}$0$0
            "#,
            expect![[r#"
fn foo() {}

fn main() {}
            "#]],
            Direction::Up,
        );
    }

    #[test]
    fn test_move_impl_up() {
        check(
            r#"
struct Yay;

trait Wow {}

impl Wow for Yay {}$0$0
            "#,
            expect![[r#"
struct Yay;

impl Wow for Yay {}

trait Wow {}
            "#]],
            Direction::Up,
        );
    }

    #[test]
    fn test_move_use_up() {
        check(
            r#"
use std::vec::Vec;
use std::collections::HashMap$0$0;
            "#,
            expect![[r#"
use std::collections::HashMap;
use std::vec::Vec;
            "#]],
            Direction::Up,
        );
    }

    #[test]
    fn moves_match_expr_up() {
        check(
            r#"
fn main() {
    let test = 123;

    $0match test {
        456 => {},
        _ => {}
    };$0
}
            "#,
            expect![[r#"
fn main() {
    match test {
        456 => {},
        _ => {}
    };

    let test = 123;
}
            "#]],
            Direction::Up,
        );
    }

    #[test]
    fn moves_param_up() {
        check(
            r#"
fn test(one: i32, two$0$0: u32) {}

fn main() {
    test(123, 456);
}
            "#,
            expect![[r#"
fn test(two: u32, one: i32) {}

fn main() {
    test(123, 456);
}
            "#]],
            Direction::Up,
        );
    }

    #[test]
    fn test_prioritizes_trait_items() {
        check(
            r#"
struct Test;

trait Yay {
    type One;

    type Two;

    fn inner();
}

impl Yay for Test {
    type One = i32;

    type Two = u32;

    fn inner() {$0$0
        println!("Mmmm");
    }
}
            "#,
            expect![[r#"
struct Test;

trait Yay {
    type One;

    type Two;

    fn inner();
}

impl Yay for Test {
    type One = i32;

    fn inner() {
        println!("Mmmm");
    }

    type Two = u32;
}
            "#]],
            Direction::Up,
        );
    }

    #[test]
    fn test_weird_nesting() {
        check(
            r#"
fn test() {
    mod hello {
        fn inner() {}
    }

    mod hi {$0$0
        fn inner() {}
    }
}
            "#,
            expect![[r#"
fn test() {
    mod hi {
        fn inner() {}
    }

    mod hello {
        fn inner() {}
    }
}
            "#]],
            Direction::Up,
        );
    }

    #[test]
    fn handles_empty_file() {
        check(r#"$0$0"#, expect![[r#""#]], Direction::Up);
    }
}
