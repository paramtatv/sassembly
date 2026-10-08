#!/usr/bin/env python3
"""Writes echo.t1. Every numeral is computed from the device constants in
crates/yantra/src/{lib,socket,input}.rs and printed in Devanagari digits; the two
device-call names and the wait member are read from the release source, not retyped."""
import re, sys
src = sys.argv[1]  # path to the sassembly source tree
rd = lambda p: open(f"{src}/{p}", encoding="utf-8").read()
SOCK = int(re.search(r"pub const SOCK: u64 = (0x[0-9a-fA-F_]+)", rd("crates/yantra/src/lib.rs")).group(1).replace("_", ""), 16)
EVENT_TAG = int(re.search(r"pub const EVENT_TAG: u64 = (0x[0-9a-fA-F_]+)", rd("crates/yantra/src/input.rs")).group(1).replace("_", ""), 16)
sock = rd("crates/yantra/src/socket.rs")
def c(n): return int(re.search(rf"pub const {n}: u32 = (0x[0-9a-f]+)", sock).group(1), 16)
EMPTY, END = c("EMPTY"), c("END")
NEXT, TX = SOCK, SOCK + 8
nir = rd("crates/sadhana/src/t1/nirvahana.rs")
MEMBER = re.search(r'WAIT_BUILTIN_MEMBER: &str = "([^"]+)"', nir).group(1)
MOD = re.search(r'WAIT_BUILTIN_MODULE: &str = "([^"]+)"', nir).group(1)
t = rd("crates/yantra/tests/w377_sockets.rs")
LOAD = re.search(r'LOAD_CALL: &str = "([^"]+)"', t).group(1)
STORE = re.search(r'STORE_CALL: &str = "([^"]+)"', t).group(1)
d = lambda n: "".join("०१२३४५६७८९"[int(x)] for x in str(n))
q = lambda m: f"{MOD}ॱ{m}"
RX = SOCK + 4
print(f"""मण्डलम् शृङ्खला ॥
आयातः {MOD} ।

सार्वजनिक चरः घटनासङ्केतक ॱॱ न६४ भवति {d(EVENT_TAG)} ।
सार्वजनिक चरः घटनामूल्यक ॱॱ न६४ भवति ० ।

वृत्तिः शब्दमुद्रणम् आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि
    चरः क्रमः ॱॱ न६४ भवति ० ।
    चरः अवगणना ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् ८ आदि
        चरः सरणम् ॱॱ न६४ भवति क्रमः गुणनम् ८ ।
        चरः सृतम् ॱॱ न६४ भवति मूल्यम् दक्षिणसृ सरणम् ।
        चरः अष्टकम् ॱॱ न६४ भवति सृतम् युक् २५५ ।
        अवगणना भवति अष्टकॱमुद्रणम् अष्टकम् ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    प्रत्यागमनम् ० ।
इति

सार्वजनिक वृत्तिः स्वपरीक्षास्वप्रतिबिम्बम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    चरः अष्टकम् ॱॱ न६४ भवति ० ।
    चरः योगफलम् ॱॱ न६४ भवति ० ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यावत् क्रमः समम् ० आदि
        अवगणना भवति {q(STORE)} {d(NEXT)} ० ।
        अष्टकम् भवति {q(LOAD)} {d(RX)} ।
        यदि अष्टकम् न्यूनम् {d(0x100)} आदि
            अवगणना भवति {q(STORE)} {d(TX)} अष्टकम् ।
            योगफलम् भवति योगफलम् योगः १ ।
        इति
        यदि अष्टकम् समम् {d(EMPTY)} आदि
            अवगणना भवति {q(MEMBER)} ० ।
        इति
        यदि अष्टकम् समम् {d(END)} आदि
            क्रमः भवति १ ।
        इति
    इति
    अवगणना भवति शब्दमुद्रणम् योगफलम् ।
    प्रत्यागमनम् ० ।
इति""")
