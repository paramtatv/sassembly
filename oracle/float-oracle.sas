॰ SANSOS — THE FLOATING-POINT DIFFERENTIAL ORACLE (`V-012`, semantic half)
॰
॰ `crates/yantra/tests/fp_oracle.rs` says plainly what it could not do:
॰ *"no independent authority has checked what these instructions COMPUTE — only
॰ what they decode to and that they run"*. Its decode half asks GNU binutils and
॰ its coverage half asks the machine not to halt `Unimplemented`; neither would
॰ have caught `V-001`'s one real defect, which was `fnmsub`/`fnmadd` implemented
॰ with their signs swapped.
॰
॰ THIS PROGRAM IS ITS OWN ORACLE, AND `qemu-system-riscv64` IS THE INDEPENDENT
॰ AUTHORITY. Every check below compares a float result's BIT PATTERN against a
॰ constant this file states, through `प्लवसंचारः` into an integer register and an
॰ integer branch. So the verdict never rests on a float comparison, and a wrong
॰ `fadd.d` cannot be hidden by an equally wrong `feq.d`. It halts `0x5555` only if
॰ every pattern matches, which makes a green run a claim the PROGRAM makes and not
॰ a byte comparison a script makes.
॰
॰ Run it on `yantra` and on `qemu-system-riscv64` and the two must agree. That is
॰ the differential part: this repository's machine against one it did not write.
॰
॰ THE CONSTANTS ARE IEEE-754 DOUBLES, WRITTEN AS THEIR EXACT OCTETS, because a
॰ float literal does not exist in this language — `grammar-t1.ebnf:288` admits no
॰ fractional production (`V-006`). That is a limitation this file turns into a
॰ virtue: an exact bit pattern is what a conformance check wants anyway, and no
॰ decimal-to-binary rounding sits between the source and the assertion.
॰
॰   ०षोड्३ऊऊ०००००००००००००  0x3FF0000000000000  1.0
॰   ०षोड्४०००००००००००००००  0x4000000000000000  2.0
॰   ०षोड्४००८००००००००००००  0x4008000000000000  3.0
॰   ०षोड्४०१८००००००००००००  0x4018000000000000  6.0
॰   ०षोड्४०१०००००००००००००  0x4010000000000000  4.0
॰
॰ FS MUST BE TURNED ON BEFORE THE FIRST FLOAT INSTRUCTION. At reset `mstatus.FS`
॰ is `Off` and a real machine traps every F/D instruction; `sstatus` is a window
॰ onto the same field, so setting bit १३ there is what enables the unit. An
॰ emulator that ignores FS would run this file either way, which is exactly why
॰ the check is here and not left to the emulator's goodwill.

॰ sstatus.FS ← Initial (bits १४:१३ = ०१), so the float unit answers at all
उपरिभारः क्षणिक२म् ०षोड्२न ।
नियन्त्रकविकल्पः शून्यःम् ०षोड्१००त् क्षणिक२न ।

॰ the constant block's address
स्थानसापेक्षयोगः क्षणिक६म् स्थिराःॱउपरिन ।
योगः क्षणिक६म् क्षणिक६न स्थिराःॱअधःन ।

॰ प्लवाहारः — the I-type float load, one of the forms `V-003` names
प्लवाहारःॱप६४ प्लव०म् क्षणिक६त् ०न ।
प्लवाहारःॱप६४ प्लव१म् क्षणिक६त् ८न ।
प्लवाहारःॱप६४ प्लव२म् क्षणिक६त् १६न ।
प्लवाहारःॱप६४ प्लव३म् क्षणिक६त् २४न ।
प्लवाहारःॱप६४ प्लव४म् क्षणिक६त् ३२न ।

॰ १ ॱॱ १.0 + २.0 must be exactly ३.0
प्लवयोगःॱप६४ प्लव५म् प्लव०न प्लव१न ।
प्लवसंचारःॱप६४ क्षणिक०म् प्लव५न ।
प्लवसंचारःॱप६४ क्षणिक१म् प्लव२न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् विफलःय् ।

॰ २ ॱॱ २.0 × ३.0 must be exactly ६.0
प्लवगुणनम्ॱप६४ प्लव५म् प्लव१न प्लव२न ।
प्लवसंचारःॱप६४ क्षणिक०म् प्लव५न ।
प्लवसंचारःॱप६४ क्षणिक१म् प्लव३न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् विफलःय् ।

॰ ३ ॱॱ ६.0 − ३.0 must be exactly ३.0
प्लववियोगःॱप६४ प्लव५म् प्लव३न प्लव२न ।
प्लवसंचारःॱप६४ क्षणिक०म् प्लव५न ।
प्लवसंचारःॱप६४ क्षणिक१म् प्लव२न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् विफलःय् ।

॰ ४ ॱॱ √४.0 must be exactly २.0 — a single-operand R-type with its own funct7
प्लववर्गमूलम्ॱप६४ प्लव५म् प्लव४न ।
प्लवसंचारःॱप६४ क्षणिक०म् प्लव५न ।
प्लवसंचारःॱप६४ क्षणिक१म् प्लव१न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् विफलःय् ।

॰ ५ ॱॱ प्लवसमम् must answer १ for ३.0 against itself. THE ONE CHECK WHOSE SUBJECT
॰ IS A FLOAT COMPARISON, and it is read as an INTEGER, which is what `feq.d`
॰ returns — so a broken compare shows up here as a wrong integer and not as a
॰ silently agreeable branch.
प्लवसमम्ॱप६४ क्षणिक०म् प्लव२न प्लव२न ।
योगः क्षणिक१म् शून्यःन १न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् विफलःय् ।

॰ ६ ॱॱ and it must answer ० for ३.0 against ६.0, or check ५ passes vacuously
प्लवसमम्ॱप६४ क्षणिक०म् प्लव२न प्लव३न ।
विषमलङ्घनम् क्षणिक०न शून्यःत् विफलःय् ।

॰ ─────────────────────────────────────────────────────────────────────────────
॰ THE F HALF — SINGLE PRECISION. `V-012`'s own margin named this as the gap the
॰ D checks above leave: *"NOT COVERED: six operations, D only. F (32-bit) is
॰ exercised by `V-003`'s encode corpus but not by this semantic one"*. Encoding
॰ `fadd.s` to the right word and COMPUTING a single-precision sum are two
॰ different claims, and only the second is one a program running on two machines
॰ can settle.
॰
॰ F IS NOT D WITH A NARROWER SUFFIX, which is why these are their own checks and
॰ not the ones above re-spelled. `प्लवसंचारःॱप३२` is `fmv.x.w` — it reads the LOW
॰ ३२ bits of the register. A machine that answered every `प३२` opcode by
॰ computing in double and writing the double back would pass all six checks above
॰ and fail check ७ at once, because the low ३२ bits of ३.0 as a double are all
॰ zero and the single-precision pattern is ०x40400000.
॰
॰   ०षोड्३ऊ८०००००  0x3F800000  1.0f
॰   ०षोड्४०००००००  0x40000000  2.0f
॰   ०षोड्४०४०००००  0x40400000  3.0f
॰   ०षोड्४०इ०००००  0x40C00000  6.0f
॰   ०षोड्४०८०००००  0x40800000  4.0f
॰
॰ `fmv.x.w` SIGN-EXTENDS. Every pattern above has bit ३१ clear, so the integer
॰ compared is the pattern itself and no widening sits between result and
॰ assertion. A FAILURE HERE HALTS WITH STATUS २, NOT १ — the D half, the F half
॰ and the conversions have three distinct statuses, so a red names which of the
॰ three it is instead of leaving the reader to re-run and bisect.

॰ प्लवाहारःॱप३२ — the I-type FLW, the other load form `V-003` names
प्लवाहारःॱप३२ प्लव१०म् क्षणिक६त् ७२न ।
प्लवाहारःॱप३२ प्लव११म् क्षणिक६त् ७६न ।
प्लवाहारःॱप३२ प्लव१२म् क्षणिक६त् ८०न ।
प्लवाहारःॱप३२ प्लव१३म् क्षणिक६त् ८४न ।
प्लवाहारःॱप३२ प्लव१४म् क्षणिक६त् ८८न ।

॰ ७ ॱॱ १.0f + २.0f must be exactly ३.0f
प्लवयोगःॱप३२ प्लव१५म् प्लव१०न प्लव११न ।
प्लवसंचारःॱप३२ क्षणिक०म् प्लव१५न ।
प्लवसंचारःॱप३२ क्षणिक१म् प्लव१२न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् लघुविफलःय् ।

॰ ८ ॱॱ २.0f × ३.0f must be exactly ६.0f
प्लवगुणनम्ॱप३२ प्लव१५म् प्लव११न प्लव१२न ।
प्लवसंचारःॱप३२ क्षणिक०म् प्लव१५न ।
प्लवसंचारःॱप३२ क्षणिक१म् प्लव१३न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् लघुविफलःय् ।

॰ ९ ॱॱ ६.0f − ३.0f must be exactly ३.0f
प्लववियोगःॱप३२ प्लव१५म् प्लव१३न प्लव१२न ।
प्लवसंचारःॱप३२ क्षणिक०म् प्लव१५न ।
प्लवसंचारःॱप३२ क्षणिक१म् प्लव१२न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् लघुविफलःय् ।

॰ १० ॱॱ √४.0f must be exactly २.0f — the single-operand form with its own funct7
प्लववर्गमूलम्ॱप३२ प्लव१५म् प्लव१४न ।
प्लवसंचारःॱप३२ क्षणिक०म् प्लव१५न ।
प्लवसंचारःॱप३२ क्षणिक१म् प्लव११न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् लघुविफलःय् ।

॰ ११ ॱॱ प्लवसमम्ॱप३२ must answer १ for ३.0f against itself
प्लवसमम्ॱप३२ क्षणिक०म् प्लव१२न प्लव१२न ।
योगः क्षणिक१म् शून्यःन १न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् लघुविफलःय् ।

॰ १२ ॱॱ and ० for ३.0f against ६.0f, or check ११ passes vacuously
प्लवसमम्ॱप३२ क्षणिक०म् प्लव१२न प्लव१३न ।
विषमलङ्घनम् क्षणिक०न शून्यःत् लघुविफलःय् ।

॰ ─────────────────────────────────────────────────────────────────────────────
॰ THE TWO FORMATS AGAINST EACH OTHER. Both halves above are self-contained: each
॰ loads its own constants and compares within one width, so a machine could get
॰ both right and still have `fcvt` wired to the wrong format — nothing yet reads
॰ a value one width wrote with the other width's instruction. These two checks do,
॰ and they are the only ones here whose subject is a CONVERSION. Status ३.

॰ १३ ॱॱ fcvt.s.d of the double ३.0 is the single ३.0 — narrowing, read as ३२ bits
प्लवरूपान्तरम्ॱप३२ॱप६४ प्लव१५म् प्लव२न ।
प्लवसंचारःॱप३२ क्षणिक०म् प्लव१५न ।
प्लवसंचारःॱप३२ क्षणिक१म् प्लव१२न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् रूपविफलःय् ।

॰ १४ ॱॱ fcvt.d.s of the single ३.0 is the double ३.0 — widening, read as ६४ bits.
॰ ३.0 is exact in both formats, so this is an identity the conversion must hold
॰ and not a rounding question; a rounding one would need a value F cannot name.
प्लवरूपान्तरम्ॱप६४ॱप३२ प्लव५म् प्लव१२न ।
प्लवसंचारःॱप६४ क्षणिक०म् प्लव५न ।
प्लवसंचारःॱप६४ क्षणिक१म् प्लव२न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् रूपविफलःय् ।

॰ ─────────────────────────────────────────────────────────────────────────────
॰ THE ROUNDING HALF. `V-012`'s margin named what the eighteen above still cannot
॰ reach: *"no check here reads `fflags`, and no rounding mode other than RNE is
॰ exercised — every constant here is exact in both formats, so nothing in this
॰ program can round at all"*. THAT IS TWO CLAIMS AND ONLY ONE OF THEM IS A
॰ MACHINE PROPERTY THIS PROGRAM CAN ASK ABOUT.
॰
॰ `fflags` AND `frm` CANNOT BE READ HERE, AND THE REASON IS MEASURED, NOT
॰ ASSUMED. `yantra`'s `csr_read` (`crates/yantra/src/lib.rs:1738`) implements
॰ `0x100`, `0x104`, `0x105`, `0x140`-`0x144`, `0x180`, `0xc01` and four vector
॰ numbers, and refuses everything else with `None` — which is an illegal
॰ instruction. `0x001` (`fflags`), `0x002` (`frm`) and `0x003` (`fcsr`) are not
॰ among them, so a `csrrs` of `fflags` traps before it answers, and `csrrwi` of
॰ `frm` cannot select a mode. A check written that way would be red against a
॰ machine this cycle may not change, and it would be red for a REGISTER FILE
॰ reason while claiming to be about arithmetic. The flag and mode half is
॰ therefore carried, named in the row, and NOT faked.
॰
॰ WHAT IS LEFT IS THE PART THAT NEEDS NO CSR AT ALL — THE ROUNDING DECISION IS
॰ IN THE RESULT. Every constant in the eighteen above is exact in both formats,
॰ which is exactly why none of them round; these five are chosen so the correct
॰ answer is one a machine can only produce by rounding the way RNE says.
॰
॰ १५ and १६ divide १ by ३, whose quotient is not representable in either format,
॰ so the last bit of the answer IS the rounding decision. १७ and १८ convert १.५
॰ and २.५ to integers: ties-to-even answers २ and २, truncation answers १ and २,
॰ ties-away answers २ and ३ — so the PAIR pins the mode and neither value alone
॰ does. १९ converts २^३१, which does not fit a signed ३२-bit integer, and the
॰ spec's answer is saturation to ०x7FFFFFFF rather than a wrap or a trap.
॰
॰ `fcvt.w.d` here is the `DYN` form — `encodings-riscv64.tsv` fixes bits १४:१२ at
॰ ७ under mask ०xfff0707f, so the mode comes from `fcsr.frm`, which is ० at
॰ reset on both machines and ० is RNE. A FAILURE HERE HALTS WITH STATUS ४.

प्लवाहारःॱप६४ प्लव७म् क्षणिक६त् ४०न ।
प्लवाहारःॱप६४ प्लव८म् क्षणिक६त् ४८न ।
प्लवाहारःॱप६४ प्लव९म् क्षणिक६त् ५६न ।
प्लवाहारःॱप६४ प्लव६म् क्षणिक६त् ६४न ।
प्लवाहारःॱप३२ प्लव१६म् क्षणिक६त् ९२न ।

॰ १५ ॱॱ १.0 ÷ ३.0 must be ०x3FD5555555555555 — THE FIRST CHECK HERE WHOSE EXACT
॰ RESULT IS INEXACT. A machine that truncated the quotient answers ...५५५४.
प्लवभागःॱप६४ प्लव५म् प्लव०न प्लव२न ।
प्लवसंचारःॱप६४ क्षणिक०म् प्लव५न ।
प्लवसंचारःॱप६४ क्षणिक१म् प्लव६न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् सन्निकर्षविफलःय् ।

॰ १६ ॱॱ १.0f ÷ ३.0f must be ०x3EAAAAAB, where truncation gives ०x3EAAAAAA. The
॰ single-precision rounding decision is its own, not the double one narrowed.
प्लवभागःॱप३२ प्लव१५म् प्लव१०न प्लव१२न ।
प्लवसंचारःॱप३२ क्षणिक०म् प्लव१५न ।
प्लवसंचारःॱप३२ क्षणिक१म् प्लव१६न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् सन्निकर्षविफलःय् ।

॰ १७ ॱॱ fcvt.w.d of १.५ must be २ — the tie rounds UP here, to the even side
प्लवरूपान्तरम्ॱअ३२ॱप६४ क्षणिक०म् प्लव७न ।
योगः क्षणिक१म् शून्यःन २न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् सन्निकर्षविफलःय् ।

॰ १८ ॱॱ and fcvt.w.d of २.५ must ALSO be २ — the tie rounds DOWN here, to the
॰ same even side. Without this one, १७ alone cannot tell ties-to-even from
॰ ties-away; without १७, १८ alone cannot tell it from truncation.
प्लवरूपान्तरम्ॱअ३२ॱप६४ क्षणिक०म् प्लव८न ।
योगः क्षणिक१म् शून्यःन २न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् सन्निकर्षविफलःय् ।

॰ १९ ॱॱ fcvt.w.d of २^३१ does not fit a signed ३२-bit integer, and the answer is
॰ ०x7FFFFFFF — saturation, not a wrap and not a trap. Built as (१ << ३१) − १
॰ because `lui` would sign-extend bit ३१ across the upper half on RV64 and the
॰ comparison would then be against ०xFFFFFFFF7FFFFFFF.
प्लवरूपान्तरम्ॱअ३२ॱप६४ क्षणिक०म् प्लव९न ।
योगः क्षणिक१म् शून्यःन १न ।
वामसरणम् क्षणिक१म् क्षणिक१न ३१न ।
योगः क्षणिक३म् शून्यःन १न ।
वियोगः क्षणिक१म् क्षणिक१न क्षणिक३न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् सन्निकर्षविफलःय् ।

॰ ─────────────────────────────────────────────────────────────────────────────
॰ THE SPECIAL-VALUE HALF — INFINITY, NaN AND SIGNED ZERO. Status ५.
॰
॰ THE NINETEEN ABOVE ALL COMPUTE ON FINITE, NORMAL NUMBERS, so every one of them
॰ could pass on a machine whose float unit has no idea what an infinity is. That
॰ is not a hypothetical: the IEEE special cases are where an implementation most
॰ often differs from the spec AND FROM ITS HOST, because a host's own hardware
॰ answers some of them differently than RISC-V requires. This half needs no
॰ `fflags` for the same reason the rounding half did not — THE SPECIAL-CASE
॰ DECISION IS IN THE RESULT PATTERN, not in a flag.
॰
॰ २० ॱॱ १.0 ÷ ०.0 = +∞ — division by zero is a RESULT, not a trap
प्लवाहारःॱप६४ प्लव१७म् क्षणिक६त् ९६न ।
प्लवाहारःॱप६४ प्लव१८म् क्षणिक६त् १०४न ।
प्लवाहारःॱप६४ प्लव१९म् क्षणिक६त् ११२न ।
प्लवाहारःॱप६४ प्लव२०म् क्षणिक६त् १२०न ।
प्लवाहारःॱप६४ प्लव२१म् क्षणिक६त् १२८न ।
प्लवाहारःॱप३२ प्लव२२म् क्षणिक६त् १३६न ।
प्लवाहारःॱप३२ प्लव२३म् क्षणिक६त् १४०न ।
प्लवाहारःॱप३२ प्लव२४म् क्षणिक६त् १४४न ।

प्लवभागःॱप६४ प्लव५म् प्लव०न प्लव१७न ।
प्लवसंचारःॱप६४ क्षणिक०म् प्लव५न ।
प्लवसंचारःॱप६४ क्षणिक१म् प्लव१९न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् विशेषविफलःय् ।

॰ २१ ॱॱ ०.0 ÷ ०.0 = THE CANONICAL QUIET NaN, ०x7FF8000000000000, and no other.
॰ RISC-V admits exactly one NaN as a generated result; a machine that handed back
॰ its HOST's default NaN would answer ०xFFF8000000000000 on an x86 divide and
॰ ०x7FF8000000000000 on an aarch64 one, so this check is also the one that says
॰ whether the float unit is canonicalising or passing the host's answer through.
प्लवभागःॱप६४ प्लव५म् प्लव१७न प्लव१७न ।
प्लवसंचारःॱप६४ क्षणिक०म् प्लव५न ।
प्लवसंचारःॱप६४ क्षणिक१म् प्लव२०न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् विशेषविफलःय् ।

॰ २२ ॱॱ √(−१.0) = the canonical NaN too — the SAME pattern from a different
॰ invalid operation, which is the claim २१ alone cannot make
प्लववर्गमूलम्ॱप६४ प्लव५म् प्लव१८न ।
प्लवसंचारःॱप६४ क्षणिक०म् प्लव५न ।
प्लवसंचारःॱप६४ क्षणिक१म् प्लव२०न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् विशेषविफलःय् ।

॰ २३ ॱॱ प्लवसमम् of the NaN against ITSELF must answer ० — NaN is not equal to
॰ anything including itself, and `feq.d` is the QUIET compare, so this must
॰ answer rather than trap. Check ५ asserted १ for a value against itself; this
॰ is the one value for which that identity is false.
प्लवसमम्ॱप६४ क्षणिक०म् प्लव२०न प्लव२०न ।
विषमलङ्घनम् क्षणिक०न शून्यःत् विशेषविफलःय् ।

॰ २४ ॱॱ −१.0 × ०.0 = −०.0, ०x8000000000000000. The SIGN of a zero is a real bit
॰ and the product's sign is the XOR of the operands' signs; a machine that
॰ normalised negative zero to positive answers ० and fails here alone.
प्लवगुणनम्ॱप६४ प्लव५म् प्लव१८न प्लव१७न ।
प्लवसंचारःॱप६४ क्षणिक०म् प्लव५न ।
प्लवसंचारःॱप६४ क्षणिक१म् प्लव२१न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् विशेषविफलःय् ।

॰ २५ ॱॱ fcvt.w.d of the NaN = ०x7FFFFFFF — the MAXIMUM signed value. This is the
॰ RISC-V answer and it is not the usual one: x86's `cvtsd2si` answers the
॰ INDEFINITE value ०x80000000 and ARM's `fcvtzs` answers ०. So a float unit that
॰ let its host decide fails this check while passing every other one here.
॰ Built as (१ << ३१) − १ for the same reason check १९ is: `lui` would sign-extend.
प्लवरूपान्तरम्ॱअ३२ॱप६४ क्षणिक०म् प्लव२०न ।
योगः क्षणिक१म् शून्यःन १न ।
वामसरणम् क्षणिक१म् क्षणिक१न ३१न ।
योगः क्षणिक३म् शून्यःन १न ।
वियोगः क्षणिक१म् क्षणिक१न क्षणिक३न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् विशेषविफलःय् ।

॰ २६ ॱॱ १.0f ÷ ०.0f = +∞ in F, ०x7F800000 — the single-precision infinity has its
॰ own exponent field and is not the double's pattern narrowed
प्लवभागःॱप३२ प्लव१५म् प्लव१०न प्लव२२न ।
प्लवसंचारःॱप३२ क्षणिक०म् प्लव१५न ।
प्लवसंचारःॱप३२ क्षणिक१म् प्लव२३न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् विशेषविफलःय् ।

॰ २७ ॱॱ ०.0f ÷ ०.0f = ०x7FC00000, the canonical NaN in F. Bit ३१ is clear in both
॰ patterns २६ and २७ compare, so `fmv.x.w`'s sign extension adds nothing and the
॰ integer compared is the pattern itself.
प्लवभागःॱप३२ प्लव१५म् प्लव२२न प्लव२२न ।
प्लवसंचारःॱप३२ क्षणिक०म् प्लव१५न ।
प्लवसंचारःॱप३२ क्षणिक१म् प्लव२४न ।
विषमलङ्घनम् क्षणिक०न क्षणिक१त् विशेषविफलःय् ।

॰ सफलता — ०x5555 समापक को
उपरिभारः क्षणिक४म् ०षोड्१००न ।
उपरिभारः क्षणिक५म् ०षोड्५न ।
योगः क्षणिक५म् क्षणिक५न ०षोड्५५५न ।
निधानम्ॱअ३२ क्षणिक४य् ०न क्षणिक५न ।

चक्रःॱॱ
लङ्घनम् शून्यःम् चक्रःय् ।

विफलःॱॱ
उपरिभारः क्षणिक४म् ०षोड्१००न ।
उपरिभारः क्षणिक५म् ०षोड्१३न ।
योगः क्षणिक५म् क्षणिक५न ०षोड्३३३न ।
निधानम्ॱअ३२ क्षणिक४य् ०न क्षणिक५न ।
लङ्घनम् शून्यःम् चक्रःय् ।

॰ STATUS २ — the F half. ०x3333 | (२ << १६).
लघुविफलःॱॱ
उपरिभारः क्षणिक४म् ०षोड्१००न ।
उपरिभारः क्षणिक५म् ०षोड्२३न ।
योगः क्षणिक५म् क्षणिक५न ०षोड्३३३न ।
निधानम्ॱअ३२ क्षणिक४य् ०न क्षणिक५न ।
लङ्घनम् शून्यःम् चक्रःय् ।

॰ STATUS ३ — the conversions. ०x3333 | (३ << १६).
रूपविफलःॱॱ
उपरिभारः क्षणिक४म् ०षोड्१००न ।
उपरिभारः क्षणिक५म् ०षोड्३३न ।
योगः क्षणिक५म् क्षणिक५न ०षोड्३३३न ।
निधानम्ॱअ३२ क्षणिक४य् ०न क्षणिक५न ।
लङ्घनम् शून्यःम् चक्रःय् ।

॰ STATUS ४ — the rounding half. ०x3333 | (४ << १६).
सन्निकर्षविफलःॱॱ
उपरिभारः क्षणिक४म् ०षोड्१००न ।
उपरिभारः क्षणिक५म् ०षोड्४३न ।
योगः क्षणिक५म् क्षणिक५न ०षोड्३३३न ।
निधानम्ॱअ३२ क्षणिक४य् ०न क्षणिक५न ।
लङ्घनम् शून्यःम् चक्रःय् ।

॰ STATUS ५ — the special-value half. ०x3333 | (५ << १६).
विशेषविफलःॱॱ
उपरिभारः क्षणिक४म् ०षोड्१००न ।
उपरिभारः क्षणिक५म् ०षोड्५३न ।
योगः क्षणिक५म् क्षणिक५न ०षोड्३३३न ।
निधानम्ॱअ३२ क्षणिक४य् ०न क्षणिक५न ।
लङ्घनम् शून्यःम् चक्रःय् ।

॥ कोष्ठकम् ॱदत्त ॥
॥ संरेखः ८ ॥
स्थिराःॱॱ
॰ +० through +३२ are the five the D and F halves compare against; +४० through
॰ +६४ are the four the rounding half needs — १.५ and २.५ for the tie, २^३१ for
॰ the conversion that does not fit, and १/३ as the quotient RNE must produce.
॥ अष्टाष्टकाः ०षोड्३ऊऊ००००००००००००० ०षोड्४००००००००००००००० ०षोड्४००८०००००००००००० ०षोड्४०१८०००००००००००० ०षोड्४०१००००००००००००० ०षोड्३ऊऊ८०००००००००००० ०षोड्४००४०००००००००००० ०षोड्४१उ००००००००००००० ०षोड्३ऊई५५५५५५५५५५५५५ ॥
॰ the singles sit at +७२ through +९२ — the doubles above are ७२ octets and ७२
॰ is a multiple of ४, so `चतुरष्टकाः` needs no further `संरेखः`.
॥ चतुरष्टकाः ०षोड्३ऊ८०००००  ०षोड्४०००००००  ०षोड्४०४०००००  ०षोड्४०इ०००००  ०षोड्४०८०००००  ०षोड्३उअअअअअआ ॥
॰ +९६ through +१२८ are the five THE SPECIAL-VALUE HALF needs. None of them is
॰ a finite number that arithmetic on the nine above can reach: ०.0 is the
॰ divisor that makes an infinity, −१.0 the operand that makes `fsqrt` invalid,
॰ and +∞, the canonical quiet NaN and −०.0 are the three patterns the answers
॰ must BE. The NaN is ०x7FF8000000000000 and not any other NaN, because
॰ RISC-V names ONE canonical NaN and a generated NaN must be it.
॥ संरेखः ८ ॥
॥ अष्टाष्टकाः ०षोड्०००००००००००००००० ०षोड्आऊऊ००००००००००००० ०षोड्७ऊऊ००००००००००००० ०षोड्७ऊऊ८०००००००००००० ०षोड्८००००००००००००००० ॥
॰ and +१३६ through +१४४ are the single-precision three. ०x7FC00000 is the
॰ canonical NaN in F and is NOT ०x7FF8000000000000 narrowed — a machine that
॰ produced one by truncating the other answers ०x7FFC0000 and fails २७.
॥ चतुरष्टकाः ०षोड्००००००००  ०षोड्७ऊ८०००००  ०षोड्७ऊइ००००० ॥
