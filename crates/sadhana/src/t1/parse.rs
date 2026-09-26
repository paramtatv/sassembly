use crate::lex::{Kind, Token};
use crate::t1::ast::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub aksara: usize,
    pub reason: String,
}

pub struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&'a Token> {
        let t = self.peek();
        self.pos += 1;
        t
    }

    fn match_word(&mut self, text: &str) -> bool {
        if let Some(t) = self.peek()
            && (t.kind == Kind::Word || matches!(t.kind, Kind::Operand { .. }))
            && t.text == text
        {
            self.advance();
            return true;
        }
        false
    }

    /// Whether the three tokens at the cursor are `keyword , identifier , close`
    /// — the whole shape of ADR-0026's `module_decl` and `import`.
    ///
    /// **The lookahead is what keeps this off a string's insides.** These two
    /// productions are the first in `parse_program` to REQUIRE a terminator, and
    /// `crates/sadhana-t1/tests/t1_sources.rs` runs the parser over a **T0**
    /// token stream, where `उक्तम् … इति` is not a token but a run of ordinary
    /// words. `crates/sadhana-t1/src/parse.t1:280` writes
    /// `आरभ्य उक्तम् आयातः इति समाप्तम्` — a string holding the import keyword —
    /// and committing on the keyword alone made the parser demand a `।` and
    /// refuse the file. A sequence that is not the production is not the
    /// declaration, so it falls through to the same clause it fell through to
    /// before this ADR.
    fn declaration_shape(&self, keyword: &str, close: &Kind) -> bool {
        let is_name = |t: &Token| matches!(t.kind, Kind::Word | Kind::Operand { .. });
        self.tokens
            .get(self.pos)
            .is_some_and(|t| is_name(t) && t.text == keyword)
            && self.tokens.get(self.pos + 1).is_some_and(is_name)
            && self
                .tokens
                .get(self.pos + 2)
                .is_some_and(|t| t.kind == *close)
    }

    /// The name a declaration introduces: the next token, which must be one.
    fn declared_name(&mut self, what: &str) -> Result<String, ParseError> {
        let t = self.advance().ok_or(ParseError {
            line: 0,
            aksara: 0,
            reason: format!("Expected {what} name"),
        })?;
        if matches!(t.kind, Kind::Word | Kind::Operand { .. }) {
            Ok(t.text.clone())
        } else {
            Err(ParseError {
                line: t.line,
                aksara: t.aksara,
                reason: format!("Expected {what} name, found {}", t.text),
            })
        }
    }

    /// Consume a required terminator, naming what it closes if it is missing.
    fn expect_kind(&mut self, kind: Kind, sign: &str, what: &str) -> Result<(), ParseError> {
        match self.peek() {
            Some(t) if t.kind == kind => {
                self.advance();
                Ok(())
            }
            Some(t) => Err(ParseError {
                line: t.line,
                aksara: t.aksara,
                reason: format!("Expected `{sign}` to close the {what}, found {}", t.text),
            }),
            None => Err(ParseError {
                line: 0,
                aksara: 0,
                reason: format!("Expected `{sign}` to close the {what}"),
            }),
        }
    }

    /// The bound of a fixed-capacity array, if one is written — ADR-0026.
    ///
    /// `अङ्कः अन्तः T` is a slice and `अङ्कः ८ अन्तः T` is an array of eight,
    /// so the ONLY thing that separates them is whether a `numeral` token sits
    /// between the index marks. It is read here rather than in `parse_type` so
    /// that the slice arm keeps its shape and the array is an addition to it.
    ///
    /// A numeral that is not a count — negative, or past `u64` — is refused
    /// with the numeral in the message rather than silently clamped, because a
    /// capacity that is not the number written is a buffer of the wrong size.
    fn parse_array_bound(&mut self) -> Result<Option<u64>, ParseError> {
        let Some(t) = self.peek() else {
            return Ok(None);
        };
        if t.kind != Kind::Numeral {
            return Ok(None);
        }
        let (line, aksara, text) = (t.line, t.aksara, t.text.clone());
        self.advance();
        match sanskrit_text::numeral::value(&text) {
            Ok(n) => Ok(Some(n)),
            Err(e) => Err(ParseError {
                line,
                aksara,
                reason: format!("`{text}` is not a capacity: {e:?}"),
            }),
        }
    }

    /// Read one type.
    ///
    /// `pub` since ADR-0026 so a test outside this crate can reach it.
    /// `parse_program` still does not call it — `:97` below defers parameters —
    /// so the ONLY exercise this has is a test that calls it directly, and
    /// `crates/sadhana/tests/t1_fixed_capacity_array.rs` says so out loud
    /// rather than letting a reader assume the `.t1` sources check it.
    pub fn parse_type(&mut self) -> Result<Type, ParseError> {
        if self.match_word("अङ्कः") {
            // ADR-0026: an optional bound between the index marks. `None` here
            // is the slice this arm has always parsed, so nothing that parsed
            // before parses differently now.
            let capacity = self.parse_array_bound()?;
            if !self.match_word("अन्तः") {
                return Err(ParseError {
                    line: self.peek().map(|t| t.line).unwrap_or(0),
                    aksara: self.peek().map(|t| t.aksara).unwrap_or(0),
                    reason: "Expected 'अन्तः' after 'अङ्कः' in slice type".into(),
                });
            }
            let inner = self.parse_type()?;
            return Ok(match capacity {
                Some(capacity) => Type::Array {
                    element: Box::new(inner),
                    capacity,
                },
                None => Type::Slice(Box::new(inner)),
            });
        } else if self.match_word("स्थानम्") {
            let inner = self.parse_type()?;
            return Ok(Type::Pointer(Box::new(inner)));
        } else if self.match_word("सम्भाव्य") {
            let inner = self.parse_type()?;
            return Ok(Type::Optional(Box::new(inner)));
        } else if self.match_word("दोषयुक्त") {
            let inner = self.parse_type()?;
            return Ok(Type::ErrorUnion(Box::new(inner)));
        }

        let t = self.advance().ok_or(ParseError {
            line: 0,
            aksara: 0,
            reason: "Expected type".into(),
        })?;

        if t.kind == Kind::Word {
            Ok(Type::Primitive(t.text.clone()))
        } else {
            Err(ParseError {
                line: t.line,
                aksara: t.aksara,
                reason: format!("Expected type name, found {}", t.text),
            })
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut declarations = Vec::new();
        while self.peek().is_some() {
            // A module header is read BEFORE the routine arm, and the
            // three-token lookahead in `declaration_shape` is what keeps it off
            // a string's insides: these two productions are the first in
            // `parse_program` to REQUIRE a terminator, and `t1_sources.rs` runs
            // this parser over a **T0** token stream where `उक्तम् … इति` is not
            // one token but a run of ordinary words. Without the lookahead an
            // `आयातः` inside a T0 string literal would parse as an import.
            if self.declaration_shape(crate::t1::mandala::MODULE, &Kind::DoubleDanda) {
                self.advance();
                let name = self.declared_name("module")?;
                self.expect_kind(Kind::DoubleDanda, "॥", "module declaration")?;
                declarations.push(Declaration::Module { name });
            } else if self.declaration_shape(crate::t1::mandala::IMPORT, &Kind::Danda) {
                self.advance();
                let name = self.declared_name("imported module")?;
                self.expect_kind(Kind::Danda, "।", "import")?;
                declarations.push(Declaration::Import { name });
            } else if self.match_word("वृत्तिः") {
                let name = self
                    .advance()
                    .ok_or(ParseError {
                        line: 0,
                        aksara: 0,
                        reason: "Expected function name".into(),
                    })?
                    .text
                    .clone();

                // Parse params (deferred for now, assume empty)
                let params = Vec::new();
                let return_type = None;

                // Block starts with आदि
                let body = if self.match_word("आदि") {
                    let mut stmts = Vec::new();
                    while self.peek().is_some() && !self.match_word("इति") {
                        let stmt = self.parse_statement()?;
                        stmts.push(stmt);
                    }
                    Some(Statement::Block(stmts))
                } else {
                    None
                };

                declarations.push(Declaration::Function {
                    name,
                    params,
                    return_type,
                    body,
                });
            } else if self.match_word("प्रकारः") {
                let name = self
                    .advance()
                    .ok_or(ParseError {
                        line: 0,
                        aksara: 0,
                        reason: "Expected type name".into(),
                    })?
                    .text
                    .clone();

                let is_struct = self.match_word("संरचना");
                declarations.push(Declaration::TypeDecl { name, is_struct });
            } else {
                // Ignore other things for now or error
                self.advance();
            }
        }
        Ok(Program { declarations })
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        if self.match_word("आदि") {
            let mut stmts = Vec::new();
            while self.peek().is_some() && !self.match_word("इति") {
                let stmt = self.parse_statement()?;
                stmts.push(stmt);
            }
            Ok(Statement::Block(stmts))
        } else {
            let expr = self.parse_expression()?;
            if let Some(t) = self.peek()
                && t.kind == Kind::Danda
            {
                self.advance();
            }
            Ok(Statement::Expression(expr))
        }
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        if self.match_word("आरभ्य") {
            let expr = self.parse_expression()?;
            if !self.match_word("समाप्तम्") {
                return Err(ParseError {
                    line: self.peek().map(|t| t.line).unwrap_or(0),
                    aksara: self.peek().map(|t| t.aksara).unwrap_or(0),
                    reason: "Expected 'समाप्तम्'".into(),
                });
            }
            Ok(Expression::Group(Box::new(expr)))
        } else if self.match_word("अङ्कः") {
            let expr = self.parse_expression()?;
            if !self.match_word("अन्तः") {
                return Err(ParseError {
                    line: self.peek().map(|t| t.line).unwrap_or(0),
                    aksara: self.peek().map(|t| t.aksara).unwrap_or(0),
                    reason: "Expected 'अन्तः'".into(),
                });
            }
            Ok(Expression::Index(Box::new(expr)))
        } else if let Some(Kind::Str { value }) = self.peek().map(|t| &t.kind) {
            // ADR-0017: the literal arrived whole. This used to walk words to
            // the first `इति`, joining them with one space — which dropped
            // ADR-0011's doubling, so `उक्तम् सः इति इति अवदत् इति` read as
            // `सः` and left `इति अवदत् इति` to be parsed as a block close, an
            // identifier and another block close. It parsed, and meant
            // something else.
            let value = value.clone();
            self.advance();
            Ok(Expression::StringLiteral(value))
        } else {
            let t = self.advance().ok_or(ParseError {
                line: 0,
                aksara: 0,
                reason: "Expected expression".into(),
            })?;

            if t.kind == Kind::Numeral {
                Ok(Expression::Numeral(t.text.clone()))
            } else if t.kind == Kind::Word {
                Ok(Expression::Identifier(t.text.clone()))
            } else {
                Err(ParseError {
                    line: t.line,
                    aksara: t.aksara,
                    reason: format!("Unexpected token in expression: {}", t.text),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::lex_t1 as lex;

    #[test]
    fn test_parse_group() {
        let tokens = lex("आरभ्य क समाप्तम् ।").unwrap();
        let mut parser = Parser::new(&tokens);
        let expr = parser.parse_expression().unwrap();
        assert_eq!(
            expr,
            Expression::Group(Box::new(Expression::Identifier("क".into())))
        );
    }

    #[test]
    fn test_parse_index() {
        let tokens = lex("अङ्कः १ अन्तः ।").unwrap();
        let mut parser = Parser::new(&tokens);
        let expr = parser.parse_expression().unwrap();
        assert_eq!(
            expr,
            Expression::Index(Box::new(Expression::Numeral("१".into())))
        );
    }

    #[test]
    fn test_parse_string() {
        let tokens = lex("उक्तम् नमस्कारः संसार इति ।").unwrap();
        let mut parser = Parser::new(&tokens);
        let expr = parser.parse_expression().unwrap();
        assert_eq!(expr, Expression::StringLiteral("नमस्कारः संसार".into()));
    }
}
