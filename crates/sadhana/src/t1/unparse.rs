//! The unparser — the inverse of `parse.rs` over the same AST. `W-215`,
//! research/23 §2.8 statistic 29, research/22 §4.4 Rule X4: every forward
//! pass has a stated inverse over the same arena. Twin of
//! `crates/sadhana-t1/src/unparse.t1` (`मुद्रण`), which is the one that
//! prints the corpus; this side prints exactly what `ast.rs` holds and no
//! more.
//!
//! # What this AST cannot spell, said by name
//!
//! `ast.rs` keeps a routine's name, parameters, return type and body, a
//! module's and an import's name, and a device's name, address and layout.
//! It does NOT keep a struct's fields — `Declaration::TypeDecl { name,
//! is_struct }` — and `parse.rs` reads a routine's parameters as an empty
//! list. So a `TypeDecl` is REFUSED here rather than printed as a struct with
//! no fields, which would re-parse "equal" only because both sides lost the
//! same thing; the refusal names the node. The `.t1` twin keeps fields since
//! this row and prints them.

use crate::t1::ast::*;
use std::fmt::Write as _;

/// What the unparser refuses to print. The first refusal is the one
/// reported; nothing after it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unprintable {
    /// A struct or enum declaration — `ast.rs` drops its fields.
    TypeDecl { name: String },
}

impl std::fmt::Display for Unprintable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unprintable::TypeDecl { name } => write!(
                f,
                "type declaration `{name}` cannot be printed: ast.rs keeps no fields for it"
            ),
        }
    }
}

impl std::error::Error for Unprintable {}

/// Print a whole program back to source, or refuse it by name.
pub fn unparse(program: &Program) -> Result<String, Unprintable> {
    let mut out = String::new();
    for decl in &program.declarations {
        unparse_declaration(decl, &mut out)?;
    }
    Ok(out)
}

fn unparse_type(ty: &Type, out: &mut String) {
    match ty {
        Type::Primitive(name) => out.push_str(name),
        Type::Slice(inner) => {
            out.push_str("अङ्कः अन्तः ");
            unparse_type(inner, out);
        }
        Type::Array { element, capacity } => {
            let _ = write!(out, "अङ्कः {capacity} अन्तः ");
            unparse_type(element, out);
        }
        Type::Pointer(inner) => {
            out.push_str("स्थानम् ");
            unparse_type(inner, out);
        }
        Type::Optional(inner) => {
            out.push_str("सम्भाव्य ");
            unparse_type(inner, out);
        }
        Type::ErrorUnion(inner) => {
            out.push_str("दोषयुक्त ");
            unparse_type(inner, out);
        }
    }
}

fn unparse_declaration(decl: &Declaration, out: &mut String) -> Result<(), Unprintable> {
    match decl {
        Declaration::Module { name } => {
            let _ = writeln!(out, "मण्डलम् {name} ॥");
        }
        Declaration::Import { name } => {
            let _ = writeln!(out, "आयातः {name} ।");
        }
        Declaration::Function {
            name,
            params,
            return_type,
            body,
        } => {
            let _ = write!(out, "वृत्तिः {name}");
            for (i, (p, ty)) in params.iter().enumerate() {
                out.push_str(if i == 0 { " आदाय " } else { " ऽ " });
                let _ = write!(out, "{p} ॱॱ ");
                unparse_type(ty, out);
            }
            if let Some(ty) = return_type {
                out.push_str(" ददाति ");
                unparse_type(ty, out);
            }
            match body {
                Some(b) => {
                    out.push(' ');
                    unparse_statement(b, out);
                }
                None => out.push('\n'),
            }
        }
        Declaration::TypeDecl { name, .. } => {
            return Err(Unprintable::TypeDecl { name: name.clone() });
        }
        Declaration::Device {
            name,
            address,
            layout,
        } => {
            let _ = write!(out, "यन्त्रम् {name} {address} ");
            unparse_type(layout, out);
            out.push('\n');
        }
    }
    Ok(())
}

fn unparse_statement(stmt: &Statement, out: &mut String) {
    match stmt {
        Statement::Expression(expr) => {
            unparse_expression(expr, out);
            out.push_str(" ।\n");
        }
        Statement::Block(stmts) => {
            out.push_str("आदि\n");
            for s in stmts {
                unparse_statement(s, out);
            }
            out.push_str("इति\n");
        }
    }
}

fn unparse_expression(expr: &Expression, out: &mut String) {
    match expr {
        Expression::Identifier(name) | Expression::Numeral(name) => out.push_str(name),
        Expression::StringLiteral(text) => {
            let _ = write!(out, "उक्तम् {text} इति");
        }
        Expression::Group(inner) => {
            out.push_str("आरभ्य ");
            unparse_expression(inner, out);
            out.push_str(" समाप्तम्");
        }
        Expression::Index(inner) => {
            out.push_str("अङ्कः ");
            unparse_expression(inner, out);
            out.push_str(" अन्तः");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::lex;
    use crate::t1::parse::Parser;

    fn parse(src: &str) -> Program {
        let tokens = lex(src).expect("the fixture lexes");
        Parser::new(&tokens)
            .parse_program()
            .expect("the fixture parses")
    }

    /// PARSE → PRINT → PARSE is the identity on what `ast.rs` holds:
    /// a module, an import, a routine whose body has an expression
    /// statement, a group and an index. The tree is compared, not the
    /// text — layout is not part of the AST.
    ///
    /// NO STRING IN THE FIXTURE, and that is a finding: `parse.rs` reads
    /// `Expression::StringLiteral` off a `Kind::Str` token, and the Rust
    /// lexer this chain uses (`crate::lex`) has no `उक्तम् … इति` form —
    /// measured: "Unexpected token in expression: उक्तम्". The printer
    /// writes the T1 spelling, which the T1 twin reads; on this side a
    /// printed string does not round-trip because the LEXER cannot read
    /// it, not because the printer misspelt it.
    #[test]
    fn a_printed_program_re_parses_to_an_equal_tree() {
        let src = "मण्डलम् क ॥\nआयातः ख ।\nवृत्तिः ग आदि\n    ३ ।\n    आरभ्य ४ समाप्तम् ।\n    अङ्कः ५ अन्तः ।\nइति\n";
        let first = parse(src);
        let printed = unparse(&first).expect("every node here is printable");
        let second = parse(&printed);
        assert_eq!(
            first, second,
            "the reprint must re-parse to the same tree; printed:\n{printed}"
        );
        // and again: printing is a fixed point once, so it is one twice.
        assert_eq!(unparse(&second).expect("printable"), printed);
    }

    /// THE REFUSED CASE: a node this AST cannot spell is refused by name,
    /// not printed as something else. `TypeDecl` keeps no fields, so a
    /// struct printed from it would be a lie that re-parses "equal".
    #[test]
    fn a_type_declaration_is_refused_by_name_not_printed_empty() {
        let program = Program {
            declarations: vec![
                Declaration::Module { name: "क".into() },
                Declaration::TypeDecl {
                    name: "आज्ञा".into(),
                    is_struct: true,
                },
            ],
        };
        let fault = unparse(&program).expect_err("a TypeDecl is refused");
        assert_eq!(
            fault,
            Unprintable::TypeDecl {
                name: "आज्ञा".into()
            }
        );
    }
}
