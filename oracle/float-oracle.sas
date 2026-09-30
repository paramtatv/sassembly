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

॥ कोष्ठकम् ॱदत्त ॥
॥ संरेखः ८ ॥
स्थिराःॱॱ
॥ अष्टाष्टकाः ०षोड्३ऊऊ००००००००००००० ०षोड्४००००००००००००००० ०षोड्४००८०००००००००००० ०षोड्४०१८०००००००००००० ०षोड्४०१००००००००००००० ॥
