# ADR-0048 — The network names: modules, members and refusals 0x35f to 0x367

**Status:** ACCEPTED, 2026-10-08 — the owner ruled "names: primaries; ADR-0047 accepted".
**Writes code:** the lexicon rows, the Rust refusal messages in `crates/yantra/src/netdev.rs`, and a `.t1` library under `spec/net/`. **Changes a frozen production:** no. `crates/sadhana-t1/src/*.t1` is untouched, the compiler image stays 922,146 octets.
**Builds on:** ADR-0047 (the device moves frames and nothing else), ADR-0040 (the wait intrinsic and the MMIO window), ADR-0043 (call-form members are not keywords).

## Decision

**D1. The primary column of the Phase 1 naming proposal is ruled, whole.** The alternatives are not
adopted. The family prefix is `सञ्चार` (sam + car, communication): `जाल` was refused as the prefix
because it is already a T0 directive. A public member is written `सञ्चारॱ<member>`, a call-form member
declared nowhere and never a keyword (ADR-0043); the layer modules are the plain words. Words the
lexicon already carried are kept, not re-added: `सम्बन्धः`, `मार्गः`, `नामनिर्देशः`.

| Role | Name | IAST | Sense |
|---|---|---|---|
| Family/module prefix (network) | सञ्चार | sañcāra | communication, the module (jāla is a T0 directive) |
| Frame send (raw) | सञ्चारॱसम्बन्धपिण्डप्रेषणम् | sañcāraॱsambandhapiṇḍapreṣaṇam | send a link frame |
| Frame receive (raw) | सञ्चारॱसम्बन्धपिण्डग्रहणम् | sañcāraॱsambandhapiṇḍagrahaṇam | receive a link frame |
| MAC address | सञ्चारॱसम्बन्धनिर्देशः | sañcāraॱsambandhanirdeśaḥ | link-layer address |
| Link up (state query) | सञ्चारॱसम्बन्धावस्था | sañcāraॱsambandhāvasthā | link state |
| Layer: virtio-net driver | सञ्चारपङ्क्तिः | sañcārapaṅktiḥ | network queue driver |
| Layer: Ethernet | सम्बन्धः | sambandhaḥ | link layer, Ethernet |
| Layer: ARP | स्थाननिर्देशः | sthānanirdeśaḥ | address directory, ARP |
| Layer: IPv4 | मार्गः | mārgaḥ | route, IP layer |
| Layer: ICMP | मार्गसन्देशः | mārgasandeśaḥ | route message, ICMP |
| ICMP ping call | सञ्चारॱमार्गप्रतिध्वनिः | sañcāraॱmārgapratidhvaniḥ | route echo, ping |
| UDP send | सञ्चारॱसन्देशप्रेषणम् | sañcāraॱsandeśapreṣaṇam | send a datagram |
| UDP receive | सञ्चारॱसन्देशग्रहणम् | sañcāraॱsandeśagrahaṇam | receive a datagram |
| Layer: UDP | निःसन्धिपिण्डः | niḥsandhipiṇḍaḥ | connectionless packet, UDP |
| DNS resolve | सञ्चारॱनामनिर्देशनिर्णयः | sañcāraॱnāmanirdeśanirṇayaḥ | settle a DNS name |
| Layer: DNS | नामनिर्देशः | nāmanirdeśaḥ | DNS (existing lexicon word) |

Phase 1 builds the first five members, the driver and the layers Ethernet, ARP, IPv4 and ICMP. UDP and
DNS (`निःसन्धिपिण्डः`, `सञ्चारॱसन्देशप्रेषणम्`, `सञ्चारॱसन्देशग्रहणम्`, `सञ्चारॱनामनिर्देशनिर्णयः`,
`नामनिर्देशः`) are named here; milestone 3 (below) builds them.

**D2. The refusals are 0x35f to 0x367, in FAIL form.** A refusal word is `code << 16 | 0x3333`, as
`शून्यविभाजननिषेधः` (0x35e) is. 0x35e is the highest code in use before these; 0x391, 0x395 and 0x3c6 are
SHA constants and are avoided.

| Code | Refusal word | Name | IAST | Sense |
|---|---|---|---|---|
| 0x35f | 0x35f3333 | सञ्चाराभावः | sañcārābhāvaḥ | network not enabled (no --net): network absent |
| 0x360 | 0x3603333 | पिण्डातिदीर्घनिषेधः | piṇḍātidīrghaniṣedhaḥ | frame too large: oversize frame refused |
| 0x361 | 0x3613333 | पिण्डसङ्ख्यातिक्रमः | piṇḍasaṅkhyātikramaḥ | frame-cap exceeded: frame count cap passed |
| 0x362 | 0x3623333 | परीक्षायोगभेदः | parīkṣāyogabhedaḥ | checksum mismatch: checksum does not match |
| 0x363 | 0x3633333 | पिण्डावैधरूपम् | piṇḍāvaidharūpam | malformed packet: packet is malformed |
| 0x364 | 0x3643333 | स्थाननिर्देशकालातीतम् | sthānanirdeśakālātītam | ARP timeout: ARP timed out |
| 0x365 | 0x3653333 | नामानुपलब्धिः | nāmānupalabdhiḥ | name not found: name not obtainable |
| 0x366 | 0x3663333 | अवैधसन्धिः | avaidhasandhiḥ | invalid handle: invalid connection handle |
| 0x367 | 0x3673333 | लक्ष्यनिषेधः | lakṣyaniṣedhaḥ | host/port not allowed: target refused |

**D3. Where each is raised.** The device (Rust) halts with the name and code for 0x35f (slot 2 touched
with no `--net-*` flag), 0x360 (a frame over `--net-max-frame`), 0x361 (either per-run frame cap) and 0x363
(a frame shorter than an Ethernet header, or a transmit buffer shorter than the 10-octet
`virtio_net_hdr`). `RULED_REFUSALS` in `netdev.rs` lists all nine pairs. The `.t1` stack raises 0x362
(IPv4 header or ICMP checksum), 0x363 (a malformed Ethernet, ARP, IPv4 or ICMP packet) and 0x364 (ARP
gave up), and returns the refusal word as its status; the other codes are reserved for UDP, DNS and the
connection handle. A library routine answers 0 or a refusal word, so the interpreter and the native
image agree on it with no change to `NATIVE_REFUSALS`, which keys only on refusals the compiler's own
lowering raises.

**D4. Five device refusals stay English placeholders.** The ruling named nothing for a source-MAC refusal
under `--net-mac`, `--net-timeout` expiry, a closed peer, an unavailable backend or a log tag mismatch.
`NET_SRC_MAC_REFUSED`, `NET_TIMEOUT`, `NET_PEER_CLOSED`, `NET_BACKEND_UNAVAILABLE` and
`NET_LOG_TAG_MISMATCH` keep their `TODO(owner)`. The runt frame and the missing header are mapped to the
malformed-packet name (0x363) and the two frame-count caps to 0x361; the owner may split them.

## What would show this wrong

- A refusal the stack must raise that none of the nine fits: a new code is a new ruling.
- A reader who takes `सम्बन्धः` for the module and `सञ्चारॱसम्बन्धपिण्डप्रेषणम्` for a call on it: the
  prefix names the family, `सम्बन्धः` the layer, and they are different modules.

## What was tested

Linux only (a Linux x86_64 host), release builds. `crates/yantra/tests/net_names.rs` (every gate): the nine
names and codes agree in the lexicon, this ADR, the device's messages and the `.t1` sources, and the
library is not in the compiler's corpus. `crates/yantra/tests/net_t1.rs` (ignored; `--include-ignored`
after building `t1_image` and `yantra-run`):

- the self-test, 24 checks over the checksum (RFC 1071's vector, the well-known 45 00 00 73 header),
  header build and parse, a bad IPv4 checksum, a bad ICMP checksum and five malformed packets, builds with
  the `t1_image` differential gate saying AGREED over 140 printed octets, and its ARP and echo frames equal
  frames the test builds from the RFCs;
- two `yantra-run`s over `--net-peer=PATH`, the responder answering ARP and the echo, the pinger getting
  `P 0`; both logs replay with no flag to the same output, instruction count and frames sent;
- the same pair under `YANTRA_VIRTIO_DEFER=1000`, and two one-read mutants of the driver (the transmit
  poll, the receive poll): each passes the synchronous device and is refused by name on the deferred one;
- from replay logs crafted out of the recorded one: a bad ICMP checksum and a bad IPv4 checksum give
  `परीक्षायोगभेदः`, a total length past the frame gives `पिण्डावैधरूपम्`, four waits that bring nothing give
  `स्थाननिर्देशकालातीतम्`; the finisher value is the refusal word;
- the driver with no `--net-*` flag halts with `सञ्चाराभावः (0x35f)`.

`tools/check-net-t1.sh` runs the same program on QEMU 10.1 `virt` with `-netdev user` and a
`virtio-net-device`: the guest (10.0.2.15) resolves 10.0.2.2 by ARP and pings it, `Q 0`, and QEMU's own
capture shows ARP request and reply, ICMP echo request and reply. Not tested: a live ARP timeout (a live
wait with a buffer posted blocks until a frame or `NET_TIMEOUT`, so the timeout is shown from a log whose
waits bring nothing); macOS and Windows.

## Review amendments, 2026-10-08

- **The driver has its own region, 0xAA00_0000**, clear of `spec/darshaka.t1`'s (0xA800_0000 up to the
  end of its frame), and zeroes both used rings and the transmit available ring at set-up. A GPU draw and
  then a ping in one run passes (`net-gpu.t1`); the same on the old shared region, rings not zeroed,
  misreads the GPU's used entries and fails.
- **A send after a refused one is refused** (cause 8) until the device has used the first buffer; the
  buffer is not overwritten under the device. A completion that names another descriptor is cause 9.
- **Completion order is not assumed**: a used entry is matched by its descriptor id, and a received buffer
  is returned by writing its id into the available slot (a read-modify-write of the u16).
- **No fence.** `.t1` has no fence primitive, and adding one is a compiler change, so stores are only in
  program order before the notify store; enough on yantra and QEMU TCG, and a `fence w,w` is owed on weaker
  hardware.
- `अन्तिमकारणम्` is reset on entry to a send and a receive. The 16-bit indices wrap across 65540 sends and
  65540 receives (`net_t1.rs`, roles `w` and `r`).

## Second naming ruling, 2026-10-08: "refusals: primaries"

Codes are per cause, not per phase: one timeout code (0x369) serves every phase, and the ARP timeout (0x364) stays its own. The six placeholders D4 left English are named, and the ping that gets no reply is now a refusal (0x36d), not the plain status 1 (the library raises it after 8 frames, and `net_t1.rs` shows it from a replay log). Phase 2's three words are reserved after them.

| Code | Refusal word | Name | IAST | Sense |
|---|---|---|---|---|
| 0x368 | 0x3683333 | उद्गमसम्बन्धनिषेधः | udgamasambandhaniṣedhaḥ | Source MAC refused: source link address refused |
| 0x369 | 0x3693333 | सञ्चारकालातीतम् | sañcārakālātītam | Timeout (shared by all phases): network timed out |
| 0x36a | 0x36a3333 | प्रतिपक्षविच्छिन्नम् | pratipakṣavicchinnam | Peer closed: counterpart disconnected |
| 0x36b | 0x36b3333 | आधारानुपलब्धम् | ādhārānupalabdham | Backend unavailable: host backend unavailable |
| 0x36c | 0x36c3333 | अभिलेखसङ्केतभेदः | abhilekhasaṅketabhedaḥ | Log tag mismatch: record-log tag differs |
| 0x36d | 0x36d3333 | प्रतिध्वन्यभावः | pratidhvanyabhāvaḥ | Ping got no reply: no echo returned |
| 0x36e | 0x36e3333 | सन्धिप्रत्याख्यानम् | sandhipratyākhyānam | RESERVED, Phase 2: connection refused: connection rejected |
| 0x36f | 0x36f3333 | प्रमाणपत्रनिषेधः | pramāṇapatraniṣedhaḥ | RESERVED, Phase 2: TLS verification failed: certificate refused |
| 0x370 | 0x3703333 | भारातिक्रमनिषेधः | bhārātikramaniṣedhaḥ | RESERVED, Phase 2: record too large: payload limit passed |

D4 is superseded: `NET_SRC_MAC_REFUSED`, `NET_TIMEOUT`, `NET_PEER_CLOSED`, `NET_BACKEND_UNAVAILABLE` and `NET_LOG_TAG_MISMATCH` lead with their ruled name and code, no `TODO(owner)` remains in `netdev.rs`, and `RESERVED_REFUSALS` holds 0x36e to 0x370. The highest code in use is 0x36d.
- **Refusal words are told from lengths at 1514**, the largest frame the driver hands over (it refuses a
  longer one with a word), not 2048: ARP resolution, the ping and the responder test `> 1514`. The
  self-test now also refuses an IPv4 header at the buffer edge with nothing available (no read past the
  array) and an ARP frame of 43 octets in an array of 42.

## Milestone 3, 2026-10-08: UDP and a DNS client (`spec/net/nihsandhi-pinda.t1`, `nama-nirdesha.t1`, `sanchara.t1`)

**Writes code:** two library files, three members in `सञ्चार`, `spec/net/net-udp-demo.t1`. **Changes a frozen production:** no (`crates/sadhana-t1/src` untouched; compiler image 922,146 octets). **Names:** only the rulings above; internal helpers are this file's own.

- **`निःसन्धिपिण्डः`** builds and parses a datagram with the checksum over the IPv4 pseudo-header (a received field of 0 means "none" and is accepted; a computed 0 is sent as 65535), and holds the **allow-list**: eight (host, port) pairs, port 0 admitting every port of a host, **denied by default**.
- **`सञ्चारॱसन्देशप्रेषणम्`** (`लक्ष्यम्, गन्तव्यपत्तनम्, स्रोतपत्तनम्, भारः, भारदैर्घ्यम्`) refuses in this order: the allow-list (`लक्ष्यनिषेधः` 0x367), a payload over 1472 (`पिण्डातिदीर्घनिषेधः` 0x360), a short array (0x363), then the device. **`सञ्चारॱसन्देशग्रहणम्`** (`पत्तनम्, भारः, प्रयासाः`) looks at up to `प्रयासाः` frames, answers ARP for this host on the way, drops what is not UDP, not to this host, not to the port or from a pair not on the list, and leaves length, sender and port in `अन्तिमदैर्घ्यम्`, `अन्तिमस्रोतपता`, `अन्तिमस्रोतपत्तनम्`. None in time is the shared timeout `सञ्चारकालातीतम्` (0x369). **A frame that is malformed (a bad IPv4 header, a bad UDP length or checksum) is IGNORED and the wait goes on, not refused**: the sender may be anyone on the wire. The payload is as long as the UDP length field says; octets after it in the IPv4 packet are not returned. (0x362 and 0x363 are raised by the parse routines, which a program may call itself.)
- **`नामनिर्देशः`** builds an A query and parses its response. A compression pointer must point strictly backward and into the body, a chain must walk strictly backward, at most 16 are followed and the walk is bounded at 400 steps, so a loop is refused as `पिण्डावैधरूपम्` (0x363) and no walk can run long. The response is matched on transaction ID, the QR bit and the question (case-folded); **a response that does not match is IGNORED (status 1), not refused**: it did not come from the server the query went to, and refusing it would let anyone on the wire end a lookup. A response with an OPCODE other than 0 is not ours either (ignored). A nonzero RCODE, or no A record, is `नामानुपलब्धिः` (0x365). **A response of ours with the TC bit set is a distinct answer (2 from the parse), and `सञ्चारॱनामनिर्देशनिर्णयः` refuses it as `उत्तरखण्डितम्` (0x371, ruled 2026-10-08).** A malformed body in a datagram from (server, 53) is ignored like any other stranger; only an answer matching the ID and question ends the lookup. The ID must fit 16 bits (a larger one is refused as 0x363). The first class-IN A record is taken (a CNAME before it is skipped; its owner name is not checked against the question).
- **`सञ्चारॱनामनिर्देशनिर्णयः`** (`सेवकः, नामः, नामदैर्घ्यम्, क्रमाङ्कः`) needs `(सेवकः, 53)` on the allow-list (else 0x367), sends one query from port 49152 + (ID & 4095) and looks at 8 frames; the address is left in `नामनिर्देशःॱअन्तिमपता`. **Choices the ruling left open:** the transaction ID is the caller's (the library has no randomness source, and a fixed ID keeps the logs replayable); there is no retransmission and no gateway (the server must be on the local network, as the ping's target is); "no response in 8 frames" is 0x369, not 0x365.

**Security, said plainly.** The source port is DERIVED from the ID (49152 + its low 12 bits), so a query carries at most 16 bits of entropy, not the ~32 of an independent port and ID: an off-path attacker who can spoof the server's address needs about 2^16 forged responses (each matching the question, which is public) to land one, within the 8-frame wait. **The 8-frame wait is itself a weakness:** any 8 frames that reach the client's port (junk, or answers that do not match) use up the wait, so a lookup can be ended with the timeout (0x369) by 8 cheap datagrams, a lower bar than spoofing a matching answer. Nothing in this milestone changes that; a deadline in time rather than in frames needs a clock the library does not have. The caller must pass an unpredictable ID, but this library has no randomness source, so RFC 5452's requirements are NOT met. **Do not use this client on an untrusted network until a randomness source exists** (and then draw port and ID separately).

**What was tested** (`crates/yantra/tests/net_t1.rs`, ignored; `tools/check-net-t1.sh` for QEMU): the self-test, 69 checks, AGREED on both engines, its three printed messages equal to messages this file builds from RFC 768 and 1035; a UDP echo and a DNS query answered by a responder `.t1` program over `--net-peer`, each frame compared byte for byte and both logs replayed alone to the same output, instruction count and frames sent (the echo also under `YANTRA_VIRTIO_DEFER=1000`); 19 crafted logs for the DNS client (malformed, cut record, pointer to itself, pointer loop, forward pointer, UDP length past the packet, bad UDP checksum, NXDOMAIN, SERVFAIL, no answer, wrong ID, wrong question, a query, a spoofed source, a wrong port, silence); 32 mutants of the library, one per refusal site that changes an answer, each failing the self-test or refused by the build's own differential gate (a 33rd, the pseudo-header's protocol number, is told by the printed messages against the ones built in the test); the redundant sites (a second check that refuses the same messages) are listed in the test, not mutated; QEMU 10.1 resolving `localhost` through slirp's 10.0.2.3 with the capture checked.

**Not done:** a retransmission, a gateway or off-link ARP, TCP (Phase 2), a CNAME chain followed to its target, AAAA, EDNS, a DNS TCP fallback after a truncated response, a source port and ID drawn at random.

## Third naming ruling, 2026-10-08: the truncated response

| Code | Refusal word | Name | IAST | Sense |
|---|---|---|---|---|
| 0x371 | 0x3713333 | उत्तरखण्डितम् | uttarakhaṇḍitam | DNS response truncated: answer cut short (TC bit set) |

The owner ruled the primary. Codes 0x36e to 0x370 stay reserved for Phase 2; the highest code in use is now 0x371. `RULED_REFUSALS` in `netdev.rs` lists it (16 pairs); the `.t1` stack raises it.
