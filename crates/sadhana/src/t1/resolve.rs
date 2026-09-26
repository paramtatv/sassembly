use crate::t1::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ResolveError {
    pub reason: String,
}

/// `W-274`, 2026-09-05: `pub resolved_symbols: HashMap<NodeId, SymbolId>` WAS
/// DELETED FROM HERE, and the design it stood for is kept because the field was
/// not a mistake — it was a placeholder for a capability the tree does not have.
///
/// WHAT IT MEANT: `resolve_expression`'s `Identifier` arm still says "We'd store
/// this if we had NodeIds in the AST" (below). `NodeId` exists — `ast.rs`
/// declares `pub struct NodeId(pub usize)` — but NOTHING IN THE TREE
/// CONSTRUCTS ONE, so a map keyed by it could never acquire an entry. It was
/// declared, never written and never read: `grep '\.resolved_symbols'` over
/// every crate returned nothing at all.
///
/// WHY NO LINT NAMED IT, which is the part worth keeping: `t1/mod.rs`'s blanket
/// `#![allow(dead_code, …)]` is NOT the reason, though it looks like it.
/// `dead_code` DOES NOT FIRE ON A `pub` FIELD OF A `pub` STRUCT in a library
/// crate — the field is public API and therefore reachable by definition.
/// Measured for `W-274`: with that allow removed entirely, twenty-one warnings
/// appear across the whole T1 directory and NOT ONE of them is this field. So
/// narrowing the allow would have left it exactly as invisible as it was, and
/// deletion is what ends it.
///
/// If AST nodes ever carry `NodeId`s, the map comes back with the arm below —
/// but it comes back WRITTEN, and its absence until then is the honest state.
pub struct Resolver {
    next_symbol: usize,
    scopes: Vec<HashMap<String, SymbolId>>,
}

/// `W-274`: `new()` takes no arguments, so `Default` is the same
/// constructor under the name the language expects. Written rather than
/// allowed, because `clippy::new_without_default` is asking for an
/// interface and not for silence.
impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            next_symbol: 0,
            scopes: vec![HashMap::new()], // Global scope
        }
    }

    fn new_symbol(&mut self) -> SymbolId {
        let id = SymbolId(self.next_symbol);
        self.next_symbol += 1;
        id
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &str) -> Result<SymbolId, ResolveError> {
        if self.scopes.last().unwrap().contains_key(name) {
            return Err(ResolveError {
                reason: format!("Duplicate symbol '{}'", name),
            });
        }
        let sym = self.new_symbol();
        let current_scope = self.scopes.last_mut().unwrap();
        current_scope.insert(name.to_string(), sym);
        Ok(sym)
    }

    fn resolve_name(&mut self, name: &str) -> Result<SymbolId, ResolveError> {
        for scope in self.scopes.iter().rev() {
            if let Some(&sym) = scope.get(name) {
                return Ok(sym);
            }
        }
        Err(ResolveError {
            reason: format!("Undefined symbol '{}'", name),
        })
    }

    pub fn resolve_program(&mut self, program: &Program) -> Result<(), ResolveError> {
        // First pass: declare all top-level functions and types
        for decl in &program.declarations {
            match decl {
                Declaration::Function { name, .. } => {
                    self.declare(name)?;
                }
                Declaration::TypeDecl { name, .. } => {
                    self.declare(name)?;
                }
                Declaration::Device { name, .. } => {
                    self.declare(name)?;
                }
                // ADR-0026. A module's own name and every imported one are
                // bound in the global scope, which is what lets the head of a
                // qualified name resolve at all. Cross-unit resolution — which
                // module a name reaches and whether it was imported — is
                // `crate::t1::mandala`, because it needs the whole compilation
                // set and this pass sees one `Program`.
                Declaration::Module { name } | Declaration::Import { name } => {
                    self.declare(name)?;
                }
            }
        }

        // Second pass: resolve bodies
        for decl in &program.declarations {
            match decl {
                Declaration::Function {
                    params,
                    return_type: _,
                    body,
                    ..
                } => {
                    self.enter_scope();
                    for (param_name, _param_type) in params {
                        self.declare(param_name)?;
                    }
                    if let Some(b) = body {
                        self.resolve_statement(b)?;
                    }
                    self.exit_scope();
                }
                Declaration::TypeDecl { .. } => {}
                Declaration::Device { .. } => {}
                Declaration::Module { .. } | Declaration::Import { .. } => {}
            }
        }
        Ok(())
    }

    fn resolve_statement(&mut self, stmt: &Statement) -> Result<(), ResolveError> {
        match stmt {
            Statement::Expression(expr) => self.resolve_expression(expr)?,
            Statement::Block(stmts) => {
                self.enter_scope();
                for s in stmts {
                    self.resolve_statement(s)?;
                }
                self.exit_scope();
            }
        }
        Ok(())
    }

    fn resolve_expression(&mut self, expr: &Expression) -> Result<(), ResolveError> {
        match expr {
            Expression::Identifier(name) => {
                let _sym = self.resolve_name(name)?;
                // We'd store this if we had NodeIds in the AST.
                // For B-081, we just need to ensure the pass exists and validates scoping.
            }
            Expression::Numeral(_) | Expression::StringLiteral(_) => {}
            Expression::Group(inner) => self.resolve_expression(inner)?,
            Expression::Index(inner) => self.resolve_expression(inner)?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_basic_scopes() {
        let mut resolver = Resolver::new();
        // A simple AST with a function "main"
        let program = Program {
            declarations: vec![Declaration::Function {
                name: "main".into(),
                params: vec![("arg1".into(), Type::Primitive("अ३२".into()))],
                return_type: None,
                body: Some(Statement::Block(vec![Statement::Expression(
                    Expression::Identifier("arg1".into()),
                )])),
            }],
        };

        let result = resolver.resolve_program(&program);
        assert!(
            result.is_ok(),
            "Failed to resolve basic program: {:?}",
            result
        );
    }

    #[test]
    fn test_duplicate_symbol() {
        let mut resolver = Resolver::new();
        let program = Program {
            declarations: vec![
                Declaration::Function {
                    name: "main".into(),
                    params: vec![],
                    return_type: None,
                    body: None,
                },
                Declaration::Function {
                    name: "main".into(), // Duplicate!
                    params: vec![],
                    return_type: None,
                    body: None,
                },
            ],
        };

        let result = resolver.resolve_program(&program);
        assert!(result.is_err());
        assert!(result.unwrap_err().reason.contains("Duplicate"));
    }

    #[test]
    fn test_undefined_symbol() {
        let mut resolver = Resolver::new();
        let program = Program {
            declarations: vec![Declaration::Function {
                name: "main".into(),
                params: vec![],
                return_type: None,
                body: Some(Statement::Block(vec![Statement::Expression(
                    Expression::Identifier("undeclared".into()),
                )])),
            }],
        };

        let result = resolver.resolve_program(&program);
        assert!(result.is_err());
        assert!(result.unwrap_err().reason.contains("Undefined"));
    }
}
