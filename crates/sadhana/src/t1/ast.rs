//! T1 AST structures

/// `W-274`, 2026-09-05: `pub struct NodeId(pub usize)` WAS DELETED FROM HERE.
///
/// It was declared, derived `Hash` so it could key a map, AND NOTHING IN THE
/// TREE EVER CONSTRUCTED ONE. Its only use was `Resolver::resolved_symbols`,
/// deleted in the same row; after that it was a type mentioned by nothing but
/// margins describing its absence.
///
/// WHY NO LINT NAMED IT — this is the row's whole subject. `dead_code` DOES NOT
/// FIRE ON A `pub` ITEM OF A LIBRARY CRATE: it is public API and therefore
/// reachable by definition. `t1/mod.rs`'s directory-wide allow looked like the
/// cause and was not. Measured for `W-274`: with that allow removed entirely,
/// twenty-one warnings appear across the T1 directory and neither `NodeId` nor
/// `resolved_symbols` is among them. Deletion is the only thing that ends a
/// dead `pub` item, because nothing will ever report it.
///
/// WHAT IT MEANT, kept because it was a placeholder and not a mistake: AST
/// nodes carry no identity, so nothing can be keyed by node. `resolve.rs`'s
/// `Identifier` arm still says "We'd store this if we had NodeIds in the AST".
/// If nodes ever carry one, this type comes back WITH a constructor and a
/// writer, which is the state it never reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
/// Types for the T1 grammar.
pub enum Type {
    /// Primitive types like `अ३२`, `बूल`, etc.
    Primitive(String),
    /// Slice: `अङ्कः अन्तः [Type]`
    Slice(Box<Type>),
    /// Fixed-capacity array: `अङ्कः [numeral] अन्तः [Type]` — ADR-0026.
    ///
    /// The SAME index pair as [`Type::Slice`] with the bound written between
    /// the marks. A slice is this type with its bound left out, which is why
    /// this is a field on the existing constructor's spelling rather than a
    /// second one: `spec/grammar-t1.ebnf` coins no word for it.
    ///
    /// `capacity` is an element count and comes from a `numeral` TOKEN, so it
    /// is known at parse time and no allocator is implied.
    Array { element: Box<Type>, capacity: u64 },
    /// Pointer: `स्थानम् [Type]`
    Pointer(Box<Type>),
    /// Optional: `सम्भाव्य [Type]`
    Optional(Box<Type>),
    /// Error Union: `दोषयुक्त [Type]`
    ErrorUnion(Box<Type>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    /// Identifier or keyword (like `सत्यम्`)
    Identifier(String),
    /// Literal number
    Numeral(String),
    /// String literal (`उक्तम् ... इति`)
    StringLiteral(String),
    /// Group: `आरभ्य [Expression] समाप्तम्`
    Group(Box<Expression>),
    /// Index: `अङ्कः [Expression] अन्तः`
    Index(Box<Expression>),
    // Operators and complex expressions deferred (B-080 bounds).
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    /// Expression statement
    Expression(Expression),
    /// Block: `आदि [Statements] इति`
    Block(Vec<Statement>),
    // Variable, return, if, etc. deferred for now or implemented as needed.
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Declaration {
    /// Module declaration (`मण्डलम् <name> ॥`), ADR-0026.
    ///
    /// Nineteen `.t1` sources opened with this before the parser had a case for
    /// it, and it parsed only because `parse_program`'s fallback advances past
    /// anything it does not know. `crates/sadhana/src/t1/mandala.rs` is where
    /// the name is resolved against the rest of the compilation set.
    Module { name: String },
    /// Import (`आयातः <name> ।`), ADR-0026. The keyword was frozen at the
    /// freeze; what it named had no production until the module declaration did.
    Import { name: String },
    /// Function declaration (वृत्तिः)
    Function {
        name: String,
        params: Vec<(String, Type)>,
        return_type: Option<Type>,
        body: Option<Statement>,
    },
    /// Type declaration (प्रकारः)
    TypeDecl { name: String, is_struct: bool },
    /// Device declaration (यन्त्रम्)
    Device {
        name: String,
        address: u64,
        layout: Type,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub declarations: Vec<Declaration>,
}
