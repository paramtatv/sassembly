use crate::t1::ast::SymbolId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Int {
        width: u8,
        signed: bool,
    },
    Float {
        width: u8,
    },
    Pointer(Box<Ty>),
    Slice(Box<Ty>),
    /// A fixed-capacity array — ADR-0026, `अङ्कः <numeral> अन्तः T`.
    ///
    /// Distinct from [`Ty::Slice`] BY THE CAPACITY AND NOT BY THE ELEMENT: two
    /// arrays of the same element and different bounds are different types, so
    /// a `४` may not be assigned where a `८` is declared. That is the whole
    /// value of writing the bound — a slice already says "some number of these".
    Array {
        element: Box<Ty>,
        capacity: u64,
    },
    Optional(Box<Ty>),
    ErrorUnion(Box<Ty>),
    Struct(SymbolId),
    Union(SymbolId),
    Enum(SymbolId),
    Void,
    Never,
    Error, // Used for poison/error recovery
}

/// One machine word. Every field of a record occupies exactly one, which is the
/// v1 rule — see [`StructLayout`].
pub const WORD: u64 = 8;

impl Ty {
    /// How many bytes a value of this type occupies IN A RECORD'S FIELD SLOT.
    ///
    /// `W-274`'s successor unit, 2026-09-06 — provisional, pending the owner's
    /// ruling on the data model. Before this there was NO size or alignment
    /// anywhere in the crate: `Ty::Struct` carried a `SymbolId` and nothing
    /// else, and at runtime a record is `Value::Record(Rc<RefCell<HashMap<…>>>)`
    /// — name-keyed and layout-free. That works for the interpreter, which never
    /// needs an address, and it is exactly why no field access could be lowered:
    /// `आहारः rd, base, offset` has nothing to compute an offset FROM.
    ///
    /// THE RULE IS ONE WORD PER FIELD, AND IT IS UNIFORM BY MEASUREMENT RATHER
    /// THAN BY TASTE. Over the 250 fields the corpus declares:
    ///   ॱ 209 are scalars — `न६४` 88, `अ६४` 59, `अ३२` 12, `बूल` 7, `इ६४` 2 and
    ///     named scalar aliases. A word each, unpacked; `अ३२` and `बूल` are
    ///     padded rather than packed, because packing is a measured change and
    ///     this is the simplest thing that can be correct.
    ///   ॱ 39 are SLICES (`अङ्कः अन्तः अ८` 35, and four of struct element type).
    ///   ॱ 3 are `सम्भाव्य`: two of a RECORD (`सम्भाव्य पाठ`), which are fine,
    ///     and ONE of a scalar (`सम्भाव्य अ३२`, `encode.t1:1020`), which is not.
    /// So an aggregate field — record, slice, optional — is a REFERENCE, one
    /// word, which makes the rule uniform and matches `Value::Record` already
    /// being `Rc`-shared. A slice's pointer and length live in the header it
    /// refers to, not inline, so no field is two words.
    ///
    /// `Optional` of a SCALAR would need a discriminant beside the payload and
    /// has no answer here, so it is REFUSED rather than guessed — and unlike the
    /// rest of this rule it has a cost, named here because I first wrote that it
    /// had none. THE CORPUS DECLARES EXACTLY ONE: `encode.t1:1020`,
    /// `विस्तार ॱॱ सम्भाव्य अ३२`, in the same record as two `सम्भाव्य पाठ` which
    /// are fine (a record is already a reference, so null is the absent case).
    /// So that one record cannot be laid out in v1 and the refusal says which.
    ///
    /// I claimed "the corpus declares none" from a top-14 frequency table read
    /// as if it were the whole list; the structural check over every `संरचना`
    /// found three `सम्भाव्य` fields, not two, and one of them is the scalar
    /// case. The number to fix is one field, and the habit is to count from the
    /// structure rather than from a display that was truncated for reading.
    pub fn field_size(&self) -> Result<u64, String> {
        match self {
            Self::Int { .. } | Self::Float { .. } | Self::Pointer(_) => Ok(WORD),
            // Aggregates are referred to, not embedded.
            Self::Slice(_) | Self::Array { .. } | Self::Struct(_) | Self::Union(_) => Ok(WORD),
            Self::Enum(_) => Ok(WORD),
            Self::Optional(inner) => match **inner {
                Self::Struct(_) | Self::Union(_) | Self::Slice(_) | Self::Array { .. } => Ok(WORD),
                _ => Err(format!(
                    "`सम्भाव्य` of a non-record ({inner:?}) has no v1 representation: a null \
                     reference cannot stand for the absent case when the present case is not a \
                     reference. The corpus declares exactly one such field — `encode.t1:1020`, \
                     `विस्तार ॱॱ सम्भाव्य अ३२` — so that record cannot be laid out until this \
                     is decided, and it is refused rather than guessed"
                )),
            },
            // `दोषयुक्त T` carries a value OR an error, so it needs a
            // discriminant beside the payload — the same unanswered question as
            // `सम्भाव्य` of a scalar, and refused for the same reason. The
            // corpus declares no field of this type either: of its 250 declared
            // fields not one is a `दोषयुक्त`. It is a RETURN type in this
            // corpus, never a member.
            Self::ErrorUnion(_) => Err(
                "`दोषयुक्त` has no v1 field representation: value-or-error needs a discriminant \
                 beside the payload. The corpus declares no such field — it is a return type \
                 here, never a member — so this is refused rather than guessed"
                    .into(),
            ),
            Self::Void | Self::Never => Ok(0),
            Self::Error => Err("a poisoned type has no layout".into()),
        }
    }
}

/// Where each field of a record sits, in declaration order.
///
/// ॥ THE DATA MODEL, RULED BY THE OWNER 2026-09-06 ॥
///
/// Three decisions, recorded here because this is the site they govern and they
/// will outlive every choice made around them. Each is written with WHAT IT
/// COSTS, so a later reader can tell a decision from drift.
///
///   1. A RECORD IS BY REFERENCE. A record value IS an address: assignment
///      shares, and passing one costs a word whatever its size, so a call site
///      never needs to know how big a record is.
///      CHOSEN BECAUSE IT ALREADY MATCHES THE INTERPRETER, whose record is
///      `Rc<RefCell<HashMap<String, Value>>>` and therefore already shared — so
///      `ख भवति क` makes them the same record on both sides, and the two halves
///      agree on SEMANTICS from the first field lowered rather than after a
///      later reconciliation.
///      COST: no record can be copied by assignment; a caller that wants a
///      distinct record must build one.
///
///   2. ONE WORD PER FIELD, so a field's offset is `WORD × declaration index`
///      and its record's size is `WORD × field count`. No alignment rules, no
///      padding logic, no trailing-pad policy — and none is built here.
///      COST: a `बूल` field wastes seven bytes and an `अ३२` four. ACCEPTED.
///      Packing moves no semantics, so it is a later change that can be
///      measured when the waste matters rather than argued now.
///
///   3. THE INTERPRETER DOES NOT CHANGE. `Value::Record` stays a name-keyed
///      `HashMap`. The twins agree on OBSERVABLE BEHAVIOUR — the same field
///      values and the same program answers — and NOT on internal
///      representation.
///      COST, and the one most likely to be "fixed" by mistake: THE TWO HALVES
///      DESCRIBE A RECORD DIFFERENTLY ON PURPOSE. Compiled code addresses a
///      block of words; the interpreter looks a name up in a map. That
///      divergence is a DECISION, not drift. The existing twin tests compare
///      emitted octets and program results, which is why they are undisturbed
///      by it. If one ever starts caring about representation, that is a
///      finding to report — not a reason to change `nirvahana.rs`.
///
/// DECLARATION ORDER IS THE LAYOUT — the compiler reorders nothing. A record is
/// addressed BY REFERENCE, so this describes the block the reference points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructLayout {
    /// `(field name, byte offset from the record's base)`, in declaration order.
    pub fields: Vec<(String, u64)>,
    /// Total size of the block, in bytes.
    pub size: u64,
}

impl StructLayout {
    /// Compute the layout of a record from its fields IN DECLARATION ORDER.
    ///
    /// Refuses a duplicate field name rather than letting the later one win:
    /// `सदस्यान्वेषणम्` in `sanchaya.t1` looks a member up BY NAME, so two
    /// fields sharing one name would make the lookup answer arbitrarily and the
    /// offset it yields would be a coin toss.
    pub fn compute(fields: &[(String, Ty)]) -> Result<Self, String> {
        let mut out = Vec::with_capacity(fields.len());
        let mut offset = 0u64;
        for (name, ty) in fields {
            if out.iter().any(|(n, _): &(String, u64)| n == name) {
                return Err(format!(
                    "the record declares `{name}` twice; a member is looked up by NAME, so a \
                     duplicate makes the offset a coin toss"
                ));
            }
            // THE OFFSET IS THE DECLARATION INDEX, NOTHING ELSE. `field_size`
            // is called for its REFUSAL — a shape with no representation must
            // not silently get a slot — and its answer is always one word.
            ty.field_size()?;
            out.push((name.clone(), offset));
            offset += WORD;
        }
        Ok(Self {
            fields: out,
            size: offset,
        })
    }

    /// The byte offset of one field, or `None` if the record has no such member.
    pub fn offset_of(&self, name: &str) -> Option<u64> {
        self.fields.iter().find(|(n, _)| n == name).map(|(_, o)| *o)
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    fn u64ty() -> Ty {
        Ty::Int {
            width: 64,
            signed: false,
        }
    }

    #[test]
    fn fields_sit_one_word_apart_in_declaration_order() {
        let l = StructLayout::compute(&[
            ("भेद".into(), u64ty()),
            ("वामसूचकाङ्क".into(), u64ty()),
            ("दक्षिणसूचकाङ्क".into(), u64ty()),
        ])
        .expect("scalars lay out");
        assert_eq!(l.offset_of("भेद"), Some(0));
        assert_eq!(l.offset_of("वामसूचकाङ्क"), Some(8));
        assert_eq!(l.offset_of("दक्षिणसूचकाङ्क"), Some(16));
        assert_eq!(l.size, 24);
        assert_eq!(l.offset_of("नास्ति"), None, "a member the record has not");
    }

    /// The rule is uniform BECAUSE an aggregate is a reference. `अङ्कः अन्तः अ८`
    /// is 35 of the corpus's 250 fields, so if a slice were two words the "one
    /// word each" rule would be false for one field in seven.
    #[test]
    fn an_aggregate_field_is_one_word_because_it_is_a_reference() {
        let l = StructLayout::compute(&[
            ("पाठ".into(), Ty::Slice(Box::new(u64ty()))),
            ("दैर्घ्य".into(), u64ty()),
            ("भीतर".into(), Ty::Struct(crate::t1::ast::SymbolId(7))),
        ])
        .expect("aggregates lay out as references");
        assert_eq!(l.offset_of("पाठ"), Some(0));
        assert_eq!(l.offset_of("दैर्घ्य"), Some(8));
        assert_eq!(l.offset_of("भीतर"), Some(16));
        assert_eq!(l.size, 24, "no field is two words");
    }

    /// `सम्भाव्य` of a RECORD is fine — the record is already a reference, so a
    /// null stands for the absent case with no discriminant.
    #[test]
    fn an_optional_record_is_one_word_and_an_optional_scalar_is_refused() {
        let ok = StructLayout::compute(&[(
            "परिवर्तनस्रोतः".into(),
            Ty::Optional(Box::new(Ty::Struct(crate::t1::ast::SymbolId(3)))),
        )])
        .expect("optional of a record lays out");
        assert_eq!(ok.size, 8);

        // THE ONE THE CORPUS ACTUALLY DECLARES — `encode.t1:1020`,
        // `विस्तार ॱॱ सम्भाव्य अ३२`. v1 cannot lay it out and says so by name
        // rather than choosing a representation nobody ruled on.
        let err = StructLayout::compute(&[(
            "विस्तार".into(),
            Ty::Optional(Box::new(Ty::Int {
                width: 32,
                signed: false,
            })),
        )])
        .expect_err("optional of a scalar has no v1 representation");
        assert!(
            err.contains("encode.t1:1020"),
            "the refusal must name the one site it costs, so the reader knows the \
             size of the gap rather than only that there is one: {err}"
        );
    }

    /// The owner's rule 2, pinned as arithmetic rather than as prose: offset is
    /// `WORD * declaration index` and size is `WORD * field count`, whatever the
    /// declared types are. A `बूल` beside an `अ६४` gets the same eight bytes.
    #[test]
    fn the_offset_is_the_declaration_index_whatever_the_types_are() {
        let mixed = StructLayout::compute(&[
            (
                "झ".into(),
                Ty::Int {
                    width: 1,
                    signed: false,
                },
            ), // बूल
            (
                "ञ".into(),
                Ty::Int {
                    width: 32,
                    signed: false,
                },
            ), // अ३२
            ("ट".into(), u64ty()),
            ("ठ".into(), Ty::Slice(Box::new(u64ty()))),
        ])
        .expect("all four are representable");
        for (i, name) in ["झ", "ञ", "ट", "ठ"].iter().enumerate() {
            assert_eq!(
                mixed.offset_of(name),
                Some(WORD * i as u64),
                "{name} sits at WORD * its declaration index and nowhere else"
            );
        }
        assert_eq!(mixed.size, WORD * 4, "size is WORD * field count");

        // The same four in another ORDER lay out at the same offsets by index —
        // the compiler reorders nothing, so a record's shape is exactly what its
        // declaration says.
        let reordered = StructLayout::compute(&[
            ("ठ".into(), Ty::Slice(Box::new(u64ty()))),
            ("ट".into(), u64ty()),
            (
                "ञ".into(),
                Ty::Int {
                    width: 32,
                    signed: false,
                },
            ),
            (
                "झ".into(),
                Ty::Int {
                    width: 1,
                    signed: false,
                },
            ),
        ])
        .expect("representable");
        assert_eq!(reordered.offset_of("ठ"), Some(0));
        assert_eq!(reordered.offset_of("झ"), Some(24));
        assert_eq!(reordered.size, mixed.size);
    }

    #[test]
    fn an_empty_record_has_no_size() {
        let l = StructLayout::compute(&[]).expect("a record may declare nothing");
        assert_eq!(l.size, 0);
        assert_eq!(l.offset_of("क"), None);
    }

    #[test]
    fn an_error_union_field_is_refused() {
        let err = StructLayout::compute(&[("फलम्".into(), Ty::ErrorUnion(Box::new(u64ty())))])
            .expect_err("value-or-error needs a discriminant");
        assert!(err.contains("दोषयुक्त"), "{err}");
    }

    /// `सदस्यान्वेषणम्` looks a member up BY NAME, so two fields of one name
    /// would make the offset it yields arbitrary.
    #[test]
    fn a_duplicate_field_name_is_refused() {
        let err = StructLayout::compute(&[("भेद".into(), u64ty()), ("भेद".into(), u64ty())])
            .expect_err("a record may not declare one name twice");
        assert!(err.contains("भेद"), "the refusal names the field: {err}");
    }
}
