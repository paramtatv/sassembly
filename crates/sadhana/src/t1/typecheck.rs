use crate::t1::ast::*;
use crate::t1::resolve::Resolver;
use crate::t1::types::Ty;

#[derive(Debug, Clone)]
pub struct TypeError {
    pub reason: String,
}

/// ॥ TWO SYMBOL-KEYED MAPS WERE DELETED HERE, AND THE MARGIN IS THE POINT ॥
///
/// `W-272`, 2026-09-05. This struct carried `types: HashMap<SymbolId, Ty>` and
/// `node_types: HashMap<NodeId, Ty>`. The row reported the first as "storage
/// keyed by a numbering that RESTARTS per program, with no reset path and no
/// writer", and offered two closings: clear it in `typecheck_program`, or
/// delete it. IT IS DELETED, and neither of the row's reasons is why.
///
/// THE PREMISE IS WRONG FOR THIS TWIN, and that is the finding. It is right for
/// the `.t1` one, where `W-223` part 2 closed exactly this: `artha.t1`'s
/// `संज्ञाप्रकारकोश`, `संज्ञाभेदकोश` and `संज्ञाघोषणाकोश` are GLOBAL, keyed by
/// `SymbolId+१`, and `निर्णायकारम्भः` sets `अग्रिमसंज्ञा` back to ० — so a second
/// program's symbol १ indexed the slot the first program's symbol १ wrote, and
/// the clearing had to be added at the reset that restarts the numbering. NONE
/// OF THAT HOLDS IN RUST:
///
///   (a) `SymbolId` is minted in one place, `Resolver::new_symbol`, off
///       `Resolver::next_symbol` — which is monotonic for the resolver's whole
///       life and is never reset. `resolve_program` called twice keeps counting,
///       and it declares into the SAME global scope: a name reused across two
///       programs is refused as a duplicate rather than renumbered. Measured in
///       `one_resolver_does_not_restart_its_numbering_for_a_second_program`.
///   (b) `TypeChecker<'a>` holds `&'a Resolver` — a SHARED borrow, and `Resolver`
///       has no interior mutability — so no `&mut Resolver` method can run for
///       this checker's whole life. THE BORROW IS THE CLEAR: a map keyed by that
///       resolver's numbering cannot outlive the numbering, because the numbering
///       cannot even ADVANCE while the map exists. Two programs through one
///       checker share one numbering and cannot collide; a second numbering
///       means a second `Resolver`, which forces a second `TypeChecker`.
///   (c) `NodeId` was CONSTRUCTED NOWHERE IN THE TREE — `ast.rs` declared it and
///       `resolve.rs` says "We'd store this if we had NodeIds in the AST". A map
///       keyed by it could not hold an entry if someone tried to fill it.
///       DELETED 2026-09-05 (`W-274`): once `Resolver::resolved_symbols` went,
///       the type was named by nothing but margins. The argument here does not
///       depend on it surviving — it was always about a key nobody could make.
///
/// So `types` was not a leak awaiting a writer; it was dead storage whose stated
/// invariant this type already enforces structurally. The intent it served is
/// still written down, at `typecheck_expression`'s identifier arm ("This is a
/// stub for variables"), and whoever implements that adds the field back — with
/// (b) as the reason it needs no reset, and `निर्णायकारम्भः`'s margin as the
/// reason the `.t1` twin's does.
///
/// `resolver` IS ITSELF UNREAD TODAY and is kept deliberately: it is the borrow
/// (b) rests on and the constructor's only argument, and dropping it would take
/// the invariant with it.
///
/// WHY NO LINT EVER NAMED THESE: `t1/mod.rs` opens
/// `#![allow(dead_code, missing_docs, unused_variables, clippy::all)]` over the
/// whole T1 directory. Three fields of a three-field struct were unread and the
/// build was clean.
///
/// `Resolver::resolved_symbols` WAS the same shape and IS NOW DELETED (`W-274`,
/// 2026-09-05) — the sentence here used to say it was "still `pub`, and left
/// alone here", which is superseded. And the reason it was invisible was NOT
/// the allow, which is what this margin implied by putting the two side by
/// side: `dead_code` cannot fire on a `pub` field of a `pub` struct at all,
/// allow or no allow. Measured — with that allow removed, twenty-one warnings
/// appear across the T1 directory and none of them is that field. The allow
/// hides the THREE PRIVATE dead items named above; the `pub` field was hidden
/// by being `pub`. Two causes that look like one.
pub struct TypeChecker<'a> {
    /// UNREAD, AND LOAD-BEARING ANYWAY — see (b) above. This shared borrow is
    /// what makes the deleted maps safe to have deleted: while it exists no
    /// `&mut Resolver` method can run, so the numbering it keys cannot advance.
    /// Dropping the field would take the invariant with it. `W-274` names the
    /// allow here rather than leaving the directory-wide one to cover it.
    #[allow(dead_code)]
    resolver: &'a Resolver,
}

impl<'a> TypeChecker<'a> {
    pub fn new(resolver: &'a Resolver) -> Self {
        Self { resolver }
    }

    pub fn typecheck_program(&mut self, program: &Program) -> Result<(), TypeError> {
        // First pass: register types
        // (In a real compiler we'd resolve struct contents)

        // Second pass: typecheck functions
        for decl in &program.declarations {
            if let Declaration::Function {
                name,
                params: _,
                return_type,
                body,
            } = decl
            {
                // Determine return type
                let ret_ty = if let Some(t) = return_type {
                    self.eval_ast_type(t)?
                } else {
                    Ty::Void
                };

                // Typecheck body
                if let Some(b) = body {
                    let body_ty = self.typecheck_statement(b)?;
                    if body_ty != ret_ty && body_ty != Ty::Never {
                        return Err(TypeError {
                            reason: format!(
                                "Type mismatch in {}: expected {:?}, got {:?}",
                                name, ret_ty, body_ty
                            ),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn eval_ast_type(&self, ast_type: &Type) -> Result<Ty, TypeError> {
        match ast_type {
            Type::Primitive(name) => {
                if name == "अ३२" {
                    Ok(Ty::Int {
                        width: 32,
                        signed: true,
                    })
                } else if name == "बूल" {
                    Ok(Ty::Int {
                        width: 1,
                        signed: false,
                    }) // Representing bool as u1
                } else {
                    Ok(Ty::Error)
                }
            }
            Type::Pointer(inner) => Ok(Ty::Pointer(Box::new(self.eval_ast_type(inner)?))),
            Type::Slice(inner) => Ok(Ty::Slice(Box::new(self.eval_ast_type(inner)?))),
            // ADR-0026. THE ZERO IS REFUSED HERE AND NOT IN THE PARSER: `०` is
            // a well-formed numeral, so the parser has nothing to complain
            // about, but an array that can hold nothing is a store with no
            // room and every write into it is out of bounds. `vastu.t1`
            // initialises its four arenas with `भवति ०`, so a `०` landing in
            // the capacity position instead is the exact typo this catches.
            Type::Array { element, capacity } => {
                if *capacity == 0 {
                    return Err(TypeError {
                        reason: "a fixed-capacity array of ० holds nothing; \
                                 write the bound the store needs"
                            .into(),
                    });
                }
                Ok(Ty::Array {
                    element: Box::new(self.eval_ast_type(element)?),
                    capacity: *capacity,
                })
            }
            Type::Optional(inner) => Ok(Ty::Optional(Box::new(self.eval_ast_type(inner)?))),
            Type::ErrorUnion(inner) => Ok(Ty::ErrorUnion(Box::new(self.eval_ast_type(inner)?))),
        }
    }

    fn typecheck_statement(&mut self, stmt: &Statement) -> Result<Ty, TypeError> {
        match stmt {
            Statement::Expression(expr) => self.typecheck_expression(expr),
            Statement::Block(stmts) => {
                let mut last_ty = Ty::Void;
                for s in stmts {
                    last_ty = self.typecheck_statement(s)?;
                }
                Ok(last_ty) // In T1, blocks might return a value or void
            }
        }
    }

    fn typecheck_expression(&mut self, expr: &Expression) -> Result<Ty, TypeError> {
        match expr {
            Expression::Identifier(_name) => {
                // If it's a known symbol, fetch its type. For now, assume it's valid if resolved.
                // The resolver checked it exists, but we haven't stored var types yet.
                // This is a stub for variables.
                Ok(Ty::Int {
                    width: 32,
                    signed: true,
                })
            }
            Expression::Numeral(_) => Ok(Ty::Int {
                width: 32,
                signed: true,
            }),
            Expression::StringLiteral(_) => Ok(Ty::Slice(Box::new(Ty::Int {
                width: 8,
                signed: false,
            }))),
            Expression::Group(inner) => self.typecheck_expression(inner),
            Expression::Index(_) => {
                // Needs to know base type. Stub.
                Ok(Ty::Error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typecheck_basic() {
        let resolver = Resolver::new();
        let mut tc = TypeChecker::new(&resolver);

        let program = Program {
            declarations: vec![Declaration::Function {
                name: "main".into(),
                params: vec![],
                return_type: None, // Implicit Void
                body: Some(Statement::Block(vec![
                    Statement::Expression(Expression::Numeral("42".into())), // Int, can be ignored in block if it's trailing, wait, block returns Int if it's the last stmt. We expect Void from fn!
                ])),
            }],
        };

        // We expect an error because the block returns Int, but function expects Void.
        let result = tc.typecheck_program(&program);
        assert!(
            result.is_err(),
            "Expected type error because block returns Int but fn returns Void"
        );
    }

    #[test]
    fn test_typecheck_match() {
        let resolver = Resolver::new();
        let mut tc = TypeChecker::new(&resolver);

        let program = Program {
            declarations: vec![Declaration::Function {
                name: "main".into(),
                params: vec![],
                return_type: Some(Type::Primitive("अ३२".into())), // Expects Int
                body: Some(Statement::Block(vec![Statement::Expression(
                    Expression::Numeral("42".into()),
                )])),
            }],
        };

        let result = tc.typecheck_program(&program);
        assert!(result.is_ok(), "Failed to typecheck matching return type");
    }

    // ── `W-272`: two programs, one checker ───────────────────────────────

    /// A routine whose declared return is `अ३२` and whose body's last statement
    /// is a numeral — the two agree, so this typechecks.
    fn agrees(name: &str) -> Program {
        Program {
            declarations: vec![Declaration::Function {
                name: name.into(),
                params: vec![],
                return_type: Some(Type::Primitive("अ३२".into())),
                body: Some(Statement::Block(vec![Statement::Expression(
                    Expression::Numeral("42".into()),
                )])),
            }],
        }
    }

    /// The same body against an implicit `Void` return — refused, and the
    /// refusal NAMES the routine, so a carried answer would be visible.
    fn disagrees(name: &str) -> Program {
        Program {
            declarations: vec![Declaration::Function {
                name: name.into(),
                params: vec![],
                return_type: None,
                body: Some(Statement::Block(vec![Statement::Expression(
                    Expression::Numeral("42".into()),
                )])),
            }],
        }
    }

    fn verdict(r: Result<(), TypeError>) -> Result<(), String> {
        r.map_err(|e| e.reason)
    }

    /// `W-272`'s acceptance, in the only terms this twin can state it: A SECOND
    /// PROGRAM THROUGH ONE `TypeChecker` SEES NOTHING OF THE FIRST.
    ///
    /// The row asked for a test proving a second program sees none of the
    /// first's entries in `types`. There are no entries and there is no
    /// `types` — see the struct's margin — so what is provable, and what
    /// matters to a caller, is that the ANSWER for the second program is the
    /// answer it gets alone. Driven in BOTH orders, because a checker that
    /// carried an accept and a checker that carried a refusal fail differently
    /// and one order would catch only one of them.
    #[test]
    fn a_second_program_through_one_checker_is_judged_on_its_own() {
        let resolver = Resolver::new();

        // The reference: each program through a checker of its own.
        let good_alone = verdict(TypeChecker::new(&resolver).typecheck_program(&agrees("क")));
        let bad_alone = verdict(TypeChecker::new(&resolver).typecheck_program(&disagrees("ख")));
        assert_eq!(good_alone, Ok(()), "`अ३२` against a numeral body agrees");
        assert!(
            bad_alone
                .as_ref()
                .is_err_and(|r| r.contains("ख") && r.contains("Void")),
            "the refusal must name the routine and the expected kind, or this \
             test cannot tell a carried answer from a fresh one: {bad_alone:?}"
        );

        // Refusal first, then the agreeing program.
        let mut after_refusal = TypeChecker::new(&resolver);
        assert_eq!(
            verdict(after_refusal.typecheck_program(&disagrees("ख"))),
            bad_alone
        );
        assert_eq!(
            verdict(after_refusal.typecheck_program(&agrees("क"))),
            good_alone,
            "the accepting program must still be accepted after a refusal"
        );

        // And the other way round.
        let mut after_accept = TypeChecker::new(&resolver);
        assert_eq!(
            verdict(after_accept.typecheck_program(&agrees("क"))),
            good_alone
        );
        assert_eq!(
            verdict(after_accept.typecheck_program(&disagrees("ख"))),
            bad_alone,
            "the refusal must still name `ख`, not the routine before it"
        );
    }

    /// `W-272`'s PREMISE, MEASURED — "keyed by a numbering that RESTARTS per
    /// program". IT DOES NOT RESTART, and this is the observable form of that.
    ///
    /// `Resolver::next_symbol` is private, so the counter is not readable; the
    /// GLOBAL SCOPE it is spent through is observable, because `declare` refuses
    /// a name already in it. A second program that reuses the first's name is
    /// refused as a duplicate — which is only possible if neither the scope nor
    /// the counter behind it was reset between the two. A distinct name still
    /// resolves, so the refusal is about the numbering persisting and not about
    /// `resolve_program` refusing every second call.
    #[test]
    fn one_resolver_does_not_restart_its_numbering_for_a_second_program() {
        let mut resolver = Resolver::new();
        resolver
            .resolve_program(&agrees("क"))
            .expect("the first program resolves");
        resolver
            .resolve_program(&agrees("ख"))
            .expect("a second program under a distinct name resolves");
        let again = resolver.resolve_program(&agrees("क"));
        assert!(
            again.is_err_and(|e| e.reason.contains("Duplicate")),
            "a name from the FIRST program must still be taken. If this ever \
             passes, the resolver began restarting per program and every \
             `SymbolId`-keyed map reachable from a `TypeChecker` needs the \
             clearing `artha.t1`'s `निर्णायकारम्भः` carries"
        );
    }
}
