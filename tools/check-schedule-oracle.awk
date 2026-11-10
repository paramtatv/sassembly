# check-schedule-oracle.awk -- the schedule log oracle of tools/check-schedule.sh.
# Reads the hex lines one policy-address pair of `run_at` printed, applies every
# rule the old inline Python applied, prints the failures (exit 1) or the summary
# line, MOVES and FIXED (exit 0).  POSIX awk; 64-bit words are handled as 16-digit
# hex strings, never as numbers.  All the work is in END: `exit` inside a function
# there ends the program without re-entering it.
#
# Run index: A=1 B=2 C=3 D=4 E=5 F=6 G=7 H=8 I=9 J=10.

function hv(c) { return index("0123456789abcdef", c) - 1 }
function hexnum(s,   i, x) {
    x = 0
    for (i = 1; i <= length(s); i++) x = x * 16 + hv(substr(s, i, 1))
    return x
}
# exact decimal text of a hex string (Python prints inbox words as decimal ints)
function dec(s,   i, d, j, c, r, carry, out) {
    r = "0"
    for (i = 1; i <= length(s); i++) {
        d = hv(substr(s, i, 1)); carry = d; out = ""
        for (j = length(r); j >= 1; j--) {
            c = substr(r, j, 1) * 16 + carry
            out = (c % 10) out
            carry = int(c / 10)
        }
        while (carry > 0) { out = (carry % 10) out; carry = int(carry / 10) }
        sub(/^0+/, "", out)
        if (out == "") out = "0"
        r = out
    }
    return r
}
# hex string + k (k < 16), as a 16-digit string
function hexadd(s, k,   i, d, out) {
    out = ""
    for (i = length(s); i >= 1; i--) { d = hv(substr(s, i, 1)) + k; k = int(d / 16); out = substr("0123456789abcdef", d % 16 + 1, 1) out }
    return out
}
function fmtx(s,   t) { t = s; sub(/^0+/, "", t); if (t == "") t = "0"; return "0x" t }
# {x:#06b} of a hex string
function fmtb(s,   i, b) {
    b = ""
    for (i = 1; i <= length(s); i++)
        b = b substr("0000000100100011010001010110011110001001101010111100110111101111", hv(substr(s, i, 1)) * 4 + 1, 4)
    sub(/^0+/, "", b)
    while (length(b) < 4) b = "0" b
    return "0b" b
}
function fmtbn(m) { return fmtb(sprintf("%x", m)) }

function die(msg) { print msg; exit 1 }
function badadd(s) { BAD[++NB] = s }
function min2(a, b) { return a < b ? a : b }
function L4(a) { return "[" a[0] ", " a[1] ", " a[2] ", " a[3] "]" }
function lvrepr(r, t) { return "[" LV[r,t,0] ", " LV[r,t,1] ", " LV[r,t,2] ", " LV[r,t,3] "]" }
function wkrepr(r, t) { return "[" WK[r,t,0] ", " WK[r,t,1] ", " WK[r,t,2] ", " WK[r,t,3] "]" }
function offrepr(r, t,   i, s) {
    s = ""
    for (i = 0; i < 4; i++) if (OFF[r,t,i]) s = s (s == "" ? "" : ", ") i
    return "[" s "]"
}
function boxrow(r, k) { return "[" BXD[r,k,0] ", " BXD[r,k,1] ", " BXD[r,k,2] ", " BXD[r,k,3] "]" }
function boxrepr(r) { return "[" boxrow(r,0) ", " boxrow(r,1) ", " boxrow(r,2) ", " boxrow(r,3) "]" }
function lstw(a, n,   k, s) {
    s = ""
    for (k = 1; k <= n; k++) s = s (k > 1 ? ", " : "") a[k]
    return "[" s "]"
}
function sameseq(r1, r2,   t) {
    if (NT[r1] != NT[r2]) return 0
    for (t = 1; t <= NT[r1]; t++) if (WANT[r1,t] != WANT[r2,t]) return 0
    return 1
}
function boxeq(r1, r2,   k, j) {
    for (k = 0; k < 4; k++) for (j = 0; j < 4; j++) if (BX[r1,k,j] != BX[r2,k,j]) return 0
    return 1
}
function boxunsent(r, k) { return BXD[r,k,0] == "0" && BXD[r,k,1] == "0" && BXD[r,k,2] == "0" && BXD[r,k,3] == "0" }
function rowis(r, k, a, b, c, d) { return BXD[r,k,0] == a && BXD[r,k,1] == b && BXD[r,k,2] == c && BXD[r,k,3] == d }
# the first three words of an inbox row against MESSAGES[j]; 0 when none
function msgidx(r, k,   j) {
    for (j = 0; j < 3; j++) if (BXD[r,k,0] == MS0[j] && BXD[r,k,1] == MS1[j] && BXD[r,k,2] == MS2[j]) return j + 1
    return 0
}

function setinit(kind,   i) {
    for (i = 0; i < 4; i++) {
        if (kind == 1) { IL[i] = (i == 3) ? 2 : 0; IW[i] = (i == 3) ? 4 : 10; IB[i] = (i == 3) ? 1 : 0; AP[i] = (i == 3) ? 1 : 8 }
        else if (kind == 2) { IL[i] = 0; IW[i] = (i == 0) ? 15 : 10; IB[i] = 0; AP[i] = 8 }
        else if (kind == 3) { IL[i] = 0; IW[i] = (i == 3) ? 6 : (i == 0) ? 12 : 10; IB[i] = (i == 3) ? 1 : 0; AP[i] = (i == 3) ? 2 : 8 }
        else { IL[i] = 0; IW[i] = (i == 3) ? 0 : 10; IB[i] = (i == 3) ? 1 : 0; AP[i] = 8 }
    }
}

# thread i is legitimately out of the set on turn t of run r
function waiting(r, mode, t, i,   x, y, tw) {
    if (mode == 0) return i == 3
    for (x = 1; x < t; x++) if (EVT[r,x] == 1 && WANT[r,x] == i) {
        tw = NT[r] + 1
        for (y = x + 1; y <= NT[r]; y++) if (EVT[r,y] == 3 && WANT[r,y] == i) { tw = y; break }
        if (t < tw) return 1
    }
    return 0
}

function structural(name, r,   t, m) {
    if (NT[r] == 0) die(name ": no turns at all — the policy refused to schedule anything")
    for (t = 1; t <= NT[r]; t++) {
        if (WANT[r,t] >= 4 || GOT[r,t] >= 4) {
            m = WANT[r,t] > GOT[r,t] ? WANT[r,t] : GOT[r,t]
            die(name " turn " t ": the log names thread " m " and there are only 4 — the index is not a thread at all")
        }
    }
}

function invariants(name, r, mode,   n, t, i, w, ok, want0, m0, run, eat, used, nxt, wk2, endw, anyl, seen, miss, lvbad, ev, cnt) {
    n = NT[r]
    if (n >= 63)
        badadd(name ": ran " n " turns and stopped on the dispatcher's budget, " \
               "not because the work ran out — some thread was never finishing")
    ok = 1
    for (i = 0; i < 4; i++) if (LV[r,1,i] != IL[i] || WK[r,1,i] != IW[i]) ok = 0
    if (!ok)
        badadd(name ": the run began at levels " lvrepr(r,1) " works " wkrepr(r,1) ", expected " \
               L4(IL) " / " L4(IW) " — the thread table was not put back to " \
               "its starting state, so this run is the tail of the previous one")
    want0 = 0
    for (i = 0; i < 4; i++) if (!IB[i] && IW[i]) want0 += 2 ^ i
    m0 = MASK[r,1]
    if (m0 != want0)
        badadd(name ": the first choice was made from " fmtbn(m0) ", expected " \
               fmtbn(want0) " — the run's starting रोधः is " L4(IB) " against works " \
               L4(IW) ", so the set the policy is first offered is fixed by it. A " \
               "run that starts from a different set has not been put back to its " \
               "starting state, and everything below it is about a different program")
    for (t = 1; t <= n; t++) {
        w = WANT[r,t]; used = USED[r,t]; ev = EVT[r,t]
        if (w != GOT[r,t]) {
            badadd(name " turn " t ": the policy chose thread " w " but thread " GOT[r,t] \
                   " is the one that ran — control did not arrive where it was sent, " \
                   "so the switch and the decision have come apart")
            return
        }
        if (!OFF[r,t,w])
            badadd(name " turn " t ": thread " w " was chosen out of the set " \
                   offrepr(r,t) " it was never offered — the policy is picking from its own " \
                   "idea of who can run rather than from the dispatcher's, which is " \
                   "the one that knows about blocking")
        for (i = 0; i < 4; i++) {
            if (OFF[r,t,i] && WK[r,t,i] == 0)
                badadd(name " turn " t ": thread " i " was offered with no work left — " \
                       "the dispatcher offered a thread that is finished")
            if (!waiting(r, mode, t, i) && !OFF[r,t,i] && WK[r,t,i] > 0)
                badadd(name " turn " t ": thread " i " has work " WK[r,t,i] " and was not " \
                       "offered, and nothing was blocking it — the dispatcher is " \
                       "withholding a runnable thread")
        }
        eat = (ev == 1) ? 0 : min2(min2(AP[w], 2), WK[r,t,w])
        if (used != eat)
            badadd(name " turn " t ": thread " w " ate " used " units, expected " eat \
                   " (appetite " AP[w] ", slice 2, work left " WK[r,t,w] \
                   (ev == 1 ? ", and it blocked itself on द्वारम् this turn)" : ")"))
        lvbad = 0
        for (i = 0; i < 4; i++) if (LV[r,t,i] > 2) lvbad = 1
        if (lvbad)
            badadd(name " turn " t ": levels " lvrepr(r,t) " — 2 is the bottom")
        if (t < n) {
            ok = 1
            nxt = "["
            for (i = 0; i < 4; i++) {
                cnt = WK[r,t,i] - ((i == w) ? used : 0)
                nxt = nxt (i ? ", " : "") cnt
                if (WK[r,t+1,i] != cnt) ok = 0
            }
            nxt = nxt "]"
            if (!ok)
                badadd(name " turn " t ": work went " wkrepr(r,t) " -> " wkrepr(r,t+1) ", but thread " w \
                       " ate " used " so it should have gone " nxt " — work is appearing " \
                       "or vanishing between turns")
        } else {
            anyl = 0; endw = "["
            for (i = 0; i < 4; i++) {
                cnt = WK[r,t,i] - ((i == w) ? used : 0)
                endw = endw (i ? ", " : "") cnt
                if (cnt != 0) anyl = 1
            }
            endw = endw "]"
            if (anyl)
                badadd(name ": the run ended with work " endw " still outstanding — the " \
                       "policy stopped scheduling while threads were still runnable")
        }
    }
    delete seen
    for (t = 1; t <= n; t++) seen[WANT[r,t]] = 1
    miss = ""
    for (i = 0; i < 4; i++) if (!(i in seen)) miss = miss (miss == "" ? "" : ", ") i
    if (miss != "")
        badadd(name ": thread(s) [" miss "] never ran once in " n " turns — starved " \
               "outright, not merely delayed")
}

# first priority violation of run r -> PVT (0 if none) and its fields
function pvfirst(r,   t, i, runnable, lowest, nr, who) {
    PVT = 0
    for (t = 1; t <= NT[r]; t++) {
        nr = 0; lowest = 99
        for (i = 0; i < 4; i++) if (OFF[r,t,i]) { nr++; if (LV[r,t,i] < lowest) lowest = LV[r,t,i] }
        if (!nr) continue
        if (LV[r,t,WANT[r,t]] != lowest) {
            who = ""
            for (i = 0; i < 4; i++) if (OFF[r,t,i] && LV[r,t,i] == lowest) who = who (who == "" ? "" : ", ") i
            PVT = t; PVW = WANT[r,t]; PVG = LV[r,t,WANT[r,t]]; PVL = lowest; PVWHO = "[" who "]"
            return
        }
    }
}

function starvation(name, r, by_level,   t, i, w, y, nr, lowest, t0, x, ok, between) {
    for (i = 0; i < 4; i++) SEENT[i] = 0
    for (t = 1; t <= NT[r]; t++) {
        w = WANT[r,t]
        nr = 0; lowest = 0
        for (i = 0; i < 4; i++) if (OFF[r,t,i]) { if (!nr || LV[r,t,i] < lowest) lowest = LV[r,t,i]; nr++ }
        for (i = 0; i < 4; i++) ELIG[i] = (OFF[r,t,i] && (!by_level || LV[r,t,i] == lowest)) ? 1 : 0
        if (SEENT[w]) {
            t0 = SEENT[w]
            for (i = 0; i < 4; i++) between[i] = 0
            for (x = t0 + 1; x <= t - 1; x++) between[WANT[r,x]] = 1
            for (y = 0; y < 4; y++) {
                if (ELIG[y] && y != w && SEENE[w,y] && !between[y]) {
                    badadd(name ": thread " w " ran at turn " t0 " and again at turn " \
                           t ", and thread " y " was eligible at both and ran at " \
                           "neither — at one level the turn must come round before " \
                           "anyone takes a second")
                    return
                }
            }
        }
        SEENT[w] = t
        for (i = 0; i < 4; i++) SEENE[w,i] = ELIG[i]
    }
}

function accounting(name, r, moves,   t, i, w, used, ex, why, same) {
    for (t = 1; t <= NT[r] - 1; t++) {
        w = WANT[r,t]; used = USED[r,t]
        if (!moves) {
            same = 1
            for (i = 0; i < 4; i++) if (LV[r,t+1,i] != LV[r,t,i]) same = 0
            if (!same) {
                badadd(name " turn " t ": levels moved " lvrepr(r,t) " -> " lvrepr(r,t+1) ", but this policy's " \
                       "गणनम् does nothing — something outside the policy is editing " \
                       "the level, which means the seam leaks")
                return
            }
            continue
        }
        if (used >= 2) {
            ex = min2(LV[r,t,w] + 1, 2)
            why = "ate its whole slice (" used " of 2) and must drop a level"
        } else {
            ex = LV[r,t,w] - 1; if (ex < 0) ex = 0
            why = "ate only " used " of 2 and must rise a level"
        }
        if (LV[r,t+1,w] != ex) {
            badadd(name " turn " t ": thread " w " " why ", going " LV[r,t,w] " -> " ex \
                   ", but the next turn records it at " LV[r,t+1,w])
            return
        }
    }
}

function blocking(name, r,   t, n, nh, nf, held, free, h1, hl, f1, ran, nran, bef, i, s) {
    n = NT[r]; nh = 0; nf = 0; held = ""; free = ""; ran = ""; nran = 0; bef = ""
    for (t = 1; t <= n; t++) {
        if (!OFF[r,t,3] && WK[r,t,3] > 0) { nh++; if (nh == 1) h1 = t; hl = t; held = held (nh > 1 ? ", " : "") t }
        if (OFF[r,t,3]) { nf++; if (nf == 1) f1 = t; free = free (nf > 1 ? ", " : "") t }
    }
    if (!nh) {
        badadd(name ": thread 3 was offered on every turn — रोधः never kept it out " \
               "of a single choice, so nothing here is about blocking")
        return
    }
    if (!nf) {
        badadd(name ": thread 3 was never offered on any of " n " turns — it was " \
               "blocked and stayed blocked, so the release never fired and a blocked " \
               "thread is indistinguishable from a finished one")
        return
    }
    if (h1 != 1 || hl >= f1) {
        badadd(name ": thread 3 was withheld with work left on turns [" held "] and " \
               "offered on [" free "] — it must be held from turn 1 and, once released, " \
               "stay released until its work runs out; a thread that goes back to " \
               "blocked is being re-blocked by something no one wrote")
        return
    }
    for (t = 1; t <= n; t++) if (WANT[r,t] == 3) {
        nran++
        if (t < f1) bef = bef (bef == "" ? "" : ", ") t
    }
    if (bef != "")
        badadd(name ": thread 3 ran at turn(s) [" bef "], " \
               "before it was released at turn " f1 " — the block did not hold")
    if (!nran)
        badadd(name ": thread 3 was released at turn " f1 " and still never ran — " \
               "the release changed a bit and not a decision")
    if (WK[r,f1,0] > 4)
        badadd(name ": thread 3 became runnable at turn " f1 ", when thread 0 " \
               "still had " WK[r,f1,0] " work left — the only writer of रोधः is thread 0 " \
               "at 4, so something else cleared it")
}

# BK/RL/WV: (turn, thread) of the BLOCK / RELEASE / WOKE events of run r
function evfill(r,   t) {
    BKn = 0; RLn = 0; WVn = 0
    for (t = 1; t <= NT[r]; t++) {
        if (EVT[r,t] == 1) { BKn++; BKt[BKn] = t; BKw[BKn] = WANT[r,t] }
        if (EVT[r,t] == 2) { RLn++; RLt[RLn] = t; RLw[RLn] = WANT[r,t] }
        if (EVT[r,t] == 3) { WVn++; WVt[WVn] = t; WVw[WVn] = WANT[r,t] }
    }
}
function eventdie(name, r,   t) {
    for (t = 1; t <= NT[r]; t++)
        if (EVT[r,t] > 3)
            die(name " turn " t ": event " EVT[r,t] " is not one of the four the log " \
                "defines, so the top four bits are carrying something else")
}

function dvaram(name, r,   k, tb, trr, tw, w, nheld, nrun, ranl, t, lst, gotl, why, gotstr) {
    eventdie(name, r)
    evfill(r)
    if (!(BKn == 3 && BKw[1] == 2 && BKw[2] == 1 && BKw[3] == 3)) {
        badadd(name ": the threads that blocked THEMSELVES were " \
               lstw(BKw, BKn) " on turns " lstw(BKt, BKn) ", expected " \
               "[2, 1, 3] — the countdowns say who sits down when, and if that order " \
               "is wrong every order below it is about a different run")
        return
    }
    if (!(RLn == 3 && RLw[1] == 0 && RLw[2] == 0 && RLw[3] == 0)) {
        badadd(name ": the head was released by " lstw(RLw, RLn) ", and only " \
               "thread 0 releases anything here")
        return
    }
    gotstr = lstw(WVw, WVn)
    if (!(WVn == 3 && WVw[1] == 2 && WVw[2] == 1 && WVw[3] == 3)) {
        if (gotstr == "[1, 2, 3]") why = "that is the order they were CHOSEN in, not the order they waited in — a set, not a queue"
        else if (gotstr == "[3, 1, 2]") why = "that is last-in-first-out — a stack, not a queue"
        else why = "which is neither the order they blocked in nor any order at all"
        badadd(name ": they blocked [2, 1, 3] and came back " gotstr ". First in must be " \
               "first out, so they must come back [2, 1, 3]; " why)
        return
    }
    for (k = 1; k <= 3; k++) {
        tb = BKt[k]; w = BKw[k]; trr = RLt[k]; tw = WVt[k]
        if (!(tb < trr && trr < tw)) {
            badadd(name ": thread " w " blocked at turn " tb ", the " k "th release " \
                   "was turn " trr " and it came back at turn " tw " — a thread must " \
                   "block, then be released, then run, in that order")
            return
        }
        nheld = 0; ranl = ""
        for (t = 1; t <= NT[r]; t++) {
            if (tb < t && t < tw && !OFF[r,t,w] && WK[r,t,w] > 0) nheld++
            if (tb < t && t < tw && WANT[r,t] == w) ranl = ranl (ranl == "" ? "" : ", ") t
        }
        if (ranl != "")
            badadd(name ": thread " w " blocked itself at turn " tb " but ran at " \
                   "turn(s) [" ranl "] before it was released — the block did not hold")
        if (nheld != tw - tb - 1)
            badadd(name ": thread " w " was out of the offered set on " nheld " of " \
                   "the " (tw - tb - 1) " turns between blocking at " tb " and coming back " \
                   "at " tw " — a thread on an endpoint is out of every one of them")
    }
}

function carried(name, r,   k, w, j, a, b, c, ord) {
    split("first second third", ORD, " ")
    if (!boxunsent(r, 0))
        badadd(name ": thread 0 sent all three messages and was sent none, " \
               "but its inbox holds " boxrow(r,0) " — something is writing into a " \
               "thread that never sat down on the endpoint")
    for (k = 0; k < 3; k++) {
        w = (k == 0) ? 2 : (k == 1) ? 1 : 3
        a = MS0[k]; b = MS1[k]; c = MS2[k]
        if (rowis(r, w, a, b, c, "0")) continue
        if (boxunsent(r, w)) {
            badadd(name ": thread " w " was released at the " ORD[k+1] " send and came " \
                   "back holding nothing, and it should have been [" a ", " b ", " c ", 0] — the " \
                   "release moved a bit and carried no message, which is exactly what " \
                   "C-002e4 already did")
        } else if ((j = msgidx(r, w)) > 0) {
            badadd(name ": thread " w " was the " ORD[k+1] " thread sent to and came back " \
                   "with the " ORD[j] " message [" BXD[r,w,0] ", " BXD[r,w,1] ", " BXD[r,w,2] "]. Messages go out in send order, " \
                   "so either the queue handed over a thread that had not waited longest " \
                   "and the message followed it, or the sender is not advancing through " \
                   "them — the wake order above says which")
        } else if (BXD[r,w,3] != "0") {
            badadd(name ": thread " w " came back with " boxrow(r,w) ", whose fourth word is " \
                   BXD[r,w,3] " and not the sender 0 — 999 is what the table " \
                   "carries in that slot, so the sender never wrote its own identity " \
                   "and the receiver cannot say who it heard from")
        } else {
            badadd(name ": thread " w " came back holding " boxrow(r,w) ", which is not any of " \
                   "the three messages " MSGS " — the four words crossed the switch " \
                   "as something other than what was sent")
        }
    }
}

function rendezvous(name, r,   tb, trel, tw, t, ranl, nheld, i, k, early, ok, ox, ol) {
    eventdie_none = 1
    evfill(r)
    if (!(BKn == 1 && BKw[1] == 0)) {
        badadd(name ": the threads that blocked themselves were " \
               lstw(BKw, BKn) ", and here exactly one must — thread 0, " \
               "the SENDER, because it sends into an empty endpoint. Thread 3 " \
               "receiving must not block: it arrives to find the sender already " \
               "sitting there, and a receive that finds a sender completes in the " \
               "turn it is made")
        return
    }
    tb = BKt[1]
    early = ""
    for (k = 1; k <= RLn && early == ""; k++) if (RLt[k] < tb) early = RLt[k] SUBSEP RLw[k] SUBSEP 2
    for (k = 1; k <= WVn && early == ""; k++) if (WVt[k] < tb) early = WVt[k] SUBSEP WVw[k] SUBSEP 3
    if (early != "") {
        split(early, ea, SUBSEP)
        badadd(name ": turn " ea[1] " records event " ea[3] " before " \
               "anything had blocked at turn " tb " — the sender is only proved to " \
               "have found the endpoint EMPTY because its own block is the first " \
               "event in the run, and this run has one before it")
        return
    }
    if (!(RLn == 1 && RLw[1] == 3)) {
        badadd(name ": the sender was let go by " lstw(RLw, RLn) ", and " \
               "only thread 3 receives here — a wake that came from anywhere " \
               "else is not a receive")
        return
    }
    if (!(WVn == 1 && WVw[1] == 0)) {
        badadd(name ": the threads that came back from inside सञ्चारः were " \
               lstw(WVw, WVn) ", expected just the sender 0")
        return
    }
    trel = RLt[1]; tw = WVt[1]
    if (!(tb < trel && trel < tw)) {
        badadd(name ": the sender blocked at turn " tb ", the receive was turn " \
               trel " and it came back at turn " tw " — it must block, then be " \
               "received from, then run, in that order")
        return
    }
    ranl = ""; nheld = 0
    for (t = 1; t <= NT[r]; t++) {
        if (tb < t && t < tw && WANT[r,t] == 0) ranl = ranl (ranl == "" ? "" : ", ") t
        if (tb < t && t < tw && !OFF[r,t,0] && WK[r,t,0] > 0) nheld++
    }
    if (ranl != "")
        badadd(name ": the sender blocked at turn " tb " but ran at turn(s) [" ranl "] " \
               "before the receive at turn " trel " — the block did not hold, and a " \
               "send that does not block is a mailbox")
    if (nheld != tw - tb - 1)
        badadd(name ": the sender was out of the offered set on " nheld " of the " \
               (tw - tb - 1) " turns between blocking at " tb " and coming back at " \
               tw " — a thread sitting on an endpoint is out of every one of them")
    for (i = 0; i < 4; i++) ox[i] = 0
    for (t = tb + 1; t <= tw - 1; t++) ox[WANT[r,t]] = 1
    ol = ""
    for (i = 0; i < 4; i++) if (ox[i] && i != 0 && i != 3) ol = ol (ol == "" ? "" : ", ") i
    if (ol != "1, 2")
        badadd(name ": threads [" ol "] ran while the sender waited from " \
               "turn " tb " to turn " tw ", expected [1, 2] — neither touches द्वारम्, " \
               "and their running is what separates a blocked sender from a stalled " \
               "dispatcher")
    for (k = 0; k < 4; k++) {
        if (k == 3) ok = rowis(r, k, "13", "4096", "32768", "0"); else ok = boxunsent(r, k)
        if (ok) continue
        if (k == 0 && rowis(r, k, "13", "4096", "32768", "0")) {
            badadd(name ": the SENDER came back holding " boxrow(r,k) " — the words it sent. " \
                   "A sender receives nothing here; an inbox filled on the sending " \
                   "side means the wake path copied its own registers back, which is " \
                   "what a receiver does and not what a sender does")
        } else if (k == 3 && boxunsent(r, k)) {
            badadd(name ": thread 3 received from a waiting sender and came " \
                   "back holding nothing, and it should have been [13, 4096, 32768, 0] — the " \
                   "receive cleared a block and took no words, so the queue was used " \
                   "as a wakeup and not as a rendezvous")
        } else if (k == 3 && BXD[r,k,3] != "0") {
            badadd(name ": thread 3 holds " boxrow(r,k) ", whose fourth word is " BXD[r,k,3] " " \
                   "and not the sender 0 — 999 is what the table carries in that " \
                   "slot, so the sender never wrote its own identity")
        } else {
            badadd(name ": thread " k " holds " boxrow(r,k) " and should hold " \
                   (k == 3 ? "[13, 4096, 32768, 0]" : "[0, 0, 0, 0]") " — the four " \
                   "words are supposed to go from thread 0 to thread 3 and " \
                   "nowhere else")
        }
    }
}

function promotion(name, r,   t, nr, nreal, rt, rw, rb, ra, k, later, t2, bef, aft, w, tb, j, behind, s, stay, lvw) {
    nr = 0; nreal = 0
    for (t = 1; t <= NT[r] - 1; t++) {
        if (EVT[r,t] == 1) {
            nr++; rt[nr] = t; rw[nr] = WANT[r,t]; rb[nr] = LV[r,t,WANT[r,t]]; ra[nr] = LV[r,t+1,WANT[r,t]]
            if (ra[nr] < rb[nr]) nreal++
        }
    }
    if (!nr) {
        badadd(name ": not one thread sat down on द्वारम्, so there was no thread " \
               "that ate less than its slice and no promotion to earn")
        return
    }
    if (!nreal) {
        stay = ""
        for (k = 1; k <= nr; k++) stay = stay (k > 1 ? ", " : "") "thread " rw[k] " stayed at " rb[k] " across its block on turn " rt[k]
        badadd(name ": " nr " thread(s) blocked and not one level rose (" stay \
               "). Either गणनम् is not lifting a thread that ate less than its " \
               "slice, or every block happened at level 0 where the lift is a " \
               "no-op — and then this run shows nothing either way")
        return
    }
    for (k = 1; k <= nr; k++) {
        if (!(ra[k] < rb[k])) continue
        tb = rt[k]; w = rw[k]; bef = rb[k]; aft = ra[k]
        t2 = 0
        for (t = tb + 1; t <= NT[r]; t++) if (WANT[r,t] == w) { t2 = t; break }
        if (!t2) {
            badadd(name ": thread " w " rose " bef " -> " aft " by blocking at turn " \
                   tb " and then never ran again — a promotion the thread never " \
                   "cashes in is a number that changed")
            return
        }
        if (LV[r,t2,w] != aft) {
            badadd(name ": thread " w " was lifted to level " aft " on turn " tb " and " \
                   "came back at level " LV[r,t2,w] " on turn " t2 " — nothing ran it in " \
                   "between, so its level was edited by something outside गणनम्")
            return
        }
        behind = 0; s = ""
        for (j = 0; j < 4; j++) if (OFF[r,t2,j]) {
            s = s (s == "" ? "" : ", ") "(" j ", " LV[r,t2,j] ")"
            if (j != w && LV[r,t2,j] >= bef) behind = 1
        }
        if (!behind) {
            badadd(name ": thread " w " rose " bef " -> " aft " by blocking at turn " \
                   tb ", and when it next ran at turn " t2 " the runnable threads " \
                   "stood at [" s "] — none of them was " \
                   "at " bef " or worse, so the rise cannot have put it in front of " \
                   "anybody and it changed a number and not a decision")
            return
        }
    }
}

function twin(name, base_name, rb, rf, row,   nb_, nf_, hs, nho, t, stray, same, anyh, ids) {
    nb_ = NT[rb]; nf_ = NT[rf]
    if (CH[rf] != CH[rb] || AC[rf] != AC[rb])
        badadd("the " name " run installed " fmtx(CH[rf]) "/" fmtx(AC[rf]) " and the " \
               base_name " run " fmtx(CH[rb]) "/" fmtx(AC[rb]) " — the whole of " \
               "this run is that it is that one again, and a run under a " \
               "different policy is not a control for anything")
    nho = 0; hs = ""
    for (t = 0; t < nf_; t++) if (EVT[rf,t+1] == 4) { nho++; HOF[nho] = t; hs = hs (nho > 1 ? ", " : "") (t + 1) }
    if (nf_ != nb_) {
        badadd("the " base_name " run took " nb_ " turns and the " name " run " \
               nf_ ". The fast path removes DISPATCHER ENTRIES, not turns: " \
               "the same threads run in the same order and the same number of " \
               "words are written, two of them on the entry that hands over")
    } else if (!nho) {
        badadd("the " name " run carries no handoff word at all, so the fast path " \
               "did not fire in the run that installs it — and a control which " \
               "is identical to its subject measures nothing. This arm asserted " \
               "word-for-word equality until " row "; it does not any more, and an " \
               "unchanged log is now the failure it used to be the pass")
    } else {
        for (t = 0; t < nb_; t++) ISHO[t] = 0
        for (t = 1; t <= nho; t++) ISHO[HOF[t]] = 1
        stray = -1
        for (t = 0; t < nb_; t++) if (TW[rb,t+1] != TW[rf,t+1] && !ISHO[t]) { stray = t; break }
        if (stray >= 0) {
            t = stray
            badadd("the " name " run differs from the " base_name " run on turn " \
                   (t + 1) ", which is not a handoff: " TW[rb,t+1] " against " \
                   TW[rf,t+1] ". The fast path is the only difference between " \
                   "these two runs, so a turn that moves without being a handoff " \
                   "is a scheduling decision that changed — either the दौड़ do not " \
                   "start from the same state, or गणनम् is not being called for a " \
                   "thread that ran")
        }
        same = -1
        for (t = 1; t <= nho; t++) if (TW[rb,HOF[t]+1] == TW[rf,HOF[t]+1]) { same = HOF[t]; break }
        if (same >= 0) {
            t = same
            badadd("turn " (t + 1) " of the " name " run is marked a handoff and is " \
                   "word for word the " base_name " run's turn: " TW[rf,t+1] ". A " \
                   "handoff word is the one place the two logs may differ, and " \
                   "one that does not differ is a label on nothing")
        }
    }
    anyh = 0
    for (t = 1; t <= nb_; t++) if (EVT[rb,t] == 4) anyh = 1
    if (anyh)
        badadd("the " base_name " run carries a handoff word, so the fast path is " \
               "installed in the run that is supposed to be the baseline — the " \
               "difference " row " measures would then be between two fast paths")
    if (!boxeq(rf, rb))
        badadd("the " name " run's inboxes came out " boxrepr(rf) " and the " \
               base_name " run's " boxrepr(rb) ", and the two runs are the same run " \
               "— a message that lands differently in a copy of a run is a " \
               "message that was not decided by the queue")
    HOSTR[rf] = hs
}

function list_from(a, n,   k, s) {
    s = ""
    for (k = 1; k <= n; k++) s = s (k > 1 ? ", " : "") a[k]
    return "[" s "]"
}

function main(   nv, i, t, s, which, r, n, k, j, ok, name) {
    nv = 0
    for (i = 1; i <= NR; i++) {
        s = L[i]
        sub(/^[ \t\r\n]+/, "", s); sub(/[ \t\r\n]+$/, "", s)
        if (s == "") continue
        if (s !~ /^[0-9a-fA-F]+$/) { print "check-schedule-oracle: not a hex line: " s > "/dev/stderr"; exit 1 }
        s = tolower(s)
        sub(/^0+/, "", s)
        if (length(s) > 16) { print "check-schedule-oracle: word wider than 64 bits: " s > "/dev/stderr"; exit 1 }
        while (length(s) < 16) s = "0" s
        V[++nv] = s
    }
    if (nv < 40)
        die("only " nv " lines printed; expected at least forty (two policy " \
            "addresses, a turn count and at least one turn, ten times over)")
    split("first second third fourth fifth sixth seventh eighth ninth tenth", WHICH, " ")
    i = 0
    for (r = 1; r <= 10; r++) {
        which = WHICH[r]
        if (i + 3 > nv) die("the " which " run's header is missing after " i " lines")
        n = hexnum(V[i + 3])
        if (n > 1000000) {
            if (hexnum(substr(V[i + 3], 1, 8)) > 0 || n > 1000000)
                die("the " which " run says " dec(V[i + 3]) " turns and then four inboxes of 4 words, " \
                    "which is " dec(hexadd(V[i + 3], 19)) " lines, but only " (nv - i) " follow it")
        }
        if (i + 3 + n + 16 > nv)
            die("the " which " run says " n " turns and then four inboxes of 4 words, " \
                "which is " (3 + n + 16) " lines, but only " (nv - i) " follow it")
        CH[r] = V[i + 1]; AC[r] = V[i + 2]; NT[r] = n
        for (t = 1; t <= n; t++) TW[r,t] = V[i + 3 + t]
        for (k = 0; k < 4; k++) for (j = 0; j < 4; j++) {
            BX[r,k,j] = V[i + 3 + n + 4 * k + j + 1]
            BXD[r,k,j] = dec(BX[r,k,j])
        }
        i += 3 + n + 16
    }
    if (i != nv) die(nv " lines printed; the ten runs account for " i)
    for (r = 1; r <= 10; r++) for (t = 1; t <= NT[r]; t++) {
        s = TW[r,t]
        WANT[r,t] = hexnum(substr(s, 15, 2)); GOT[r,t] = hexnum(substr(s, 13, 2)); USED[r,t] = hexnum(substr(s, 11, 2))
        for (k = 0; k < 4; k++) { LV[r,t,k] = hv(substr(s, 10 - k, 1)); WK[r,t,k] = hv(substr(s, 6 - k, 1)) }
        MASK[r,t] = hv(substr(s, 2, 1)); EVT[r,t] = hv(substr(s, 1, 1))
        for (k = 0; k < 4; k++) OFF[r,t,k] = int(MASK[r,t] / (2 ^ k)) % 2
    }
    MS0[0] = "13"; MS1[0] = "4096"; MS2[0] = "32768"
    MS0[1] = "15"; MS1[1] = "8192"; MS2[1] = "32800"
    MS0[2] = "12"; MS1[2] = "12288"; MS2[2] = "32832"
    MSGS = "[[13, 4096, 32768], [15, 8192, 32800], [12, 12288, 32832]]"
    split("first second third", ORD, " ")

    structural("श्रेणी", 1)
    structural("चक्रम्", 2)
    structural("द्वारम्", 3)
    structural("प्रेषकः", 4)
    structural("उन्नतिः", 5)
    setinit(1)
    invariants("श्रेणी", 1, 0)
    invariants("चक्रम्", 2, 0)

    setinit(2)
    invariants("द्वारम्", 3, 1)
    dvaram("द्वारम्", 3)
    carried("द्वारम्", 3)
    starvation("द्वारम्", 3, 0)
    accounting("द्वारम्", 3, 0)

    invariants("प्रेषकः", 4, 1)
    eventdie("प्रेषकः", 4)
    rendezvous("प्रेषकः", 4)
    starvation("प्रेषकः", 4, 0)
    accounting("प्रेषकः", 4, 0)

    invariants("उन्नतिः", 5, 1)
    dvaram("उन्नतिः", 5)
    carried("उन्नतिः", 5)
    starvation("उन्नतिः", 5, 1)
    accounting("उन्नतिः", 5, 1)
    promotion("उन्नतिः", 5)
    pvfirst(5)
    if (PVT)
        badadd("उन्नतिः turn " PVT ": thread " PVW " ran at level " PVG " while thread(s) " PVWHO \
               " were runnable at level " PVL " — this run installs श्रेणी, and a policy " \
               "that lets a worse level through is not the one whose promotion is under " \
               "test here")

    for (k = 1; k <= 2; k++) {
        name = (k == 1) ? "श्रेणी" : "चक्रम्"
        s = ""; j = 0
        for (t = 1; t <= NT[k]; t++) if (EVT[k,t] != 0) { j++; s = s (j > 1 ? ", " : "") t }
        if (j)
            badadd(name ": turn(s) [" s "] record a द्वारम् event, and द्वारम् is shut " \
                   "for this run — the event field is not reporting what it says")
        ok = 1
        for (t = 0; t < 4; t++) if (!boxunsent(k, t)) ok = 0
        if (!ok)
            badadd(name ": the inboxes came out " boxrepr(k) " with द्वारम् shut for this run — " \
                   "nothing sent anything, so either they were not cleared between runs " \
                   "or something is writing messages nobody sent")
    }

    pvfirst(1)
    if (PVT)
        badadd("श्रेणी turn " PVT ": thread " PVW " ran at level " PVG " while thread(s) " PVWHO \
               " were runnable at level " PVL " — priority is not ordering anything, and a " \
               "count of runs would never have shown it")
    starvation("श्रेणी", 1, 1)
    starvation("चक्रम्", 2, 0)
    accounting("श्रेणी", 1, 1)
    accounting("चक्रम्", 2, 0)
    blocking("श्रेणी", 1)
    blocking("चक्रम्", 2)

    drops = 0
    for (t = 2; t <= NT[1]; t++) if (LV[1,t,3] < LV[1,t-1,3]) drops++
    if (drops < 2)
        badadd("श्रेणी: thread 3 started at level 2 and its level fell " \
               drops " time(s) across the run — it eats less than its slice every turn, " \
               "so the interactivity rule should have lifted it more than once")
    ok = 0
    for (t = 1; t <= NT[1]; t++) if (WANT[1,t] == 3)
        for (j = 0; j < 4; j++) if (j != 3 && OFF[1,t,j] && LV[1,t,j] > LV[1,t,3]) ok = 1
    if (!ok)
        badadd("श्रेणी: thread 3 was never chosen while another thread was runnable at a " \
               "worse level — its promotion changed a number and never changed an order, " \
               "which is the only thing the promotion is for")

    if (sameseq(1, 2))
        badadd("both policies produced the same order of threads. Either the same policy " \
               "was installed twice, or the dispatcher is not calling through नीतिः at all " \
               "— and if it is not, the interface is decoration and nothing can be swapped " \
               "in later")
    pvfirst(2)
    if (!PVT)
        badadd("चक्रम् never once ran a thread while a better-placed one was runnable, so " \
               "it is obeying levels it is supposed to ignore — the two policies are two " \
               "names for one policy, and the seam has not been shown to carry anything")
    if (CH[4] != CH[2] || AC[4] != AC[2])
        badadd("the प्रेषकः run installed " fmtx(CH[4]) "/" fmtx(AC[4]) " and not चक्रम्'s " \
               fmtx(CH[2]) "/" fmtx(AC[2]) " — the same reason as the द्वारम् run, and it " \
               "is also what makes the two comparable")
    if (CH[5] != CH[1] || AC[5] != AC[1])
        badadd("the उन्नतिः run installed " fmtx(CH[5]) "/" fmtx(AC[5]) " and not श्रेणी's " \
               fmtx(CH[1]) "/" fmtx(AC[1]) " — the whole of this run is that the policy " \
               "which reads levels is the one on the endpoint, and it is not")
    if (sameseq(5, 3))
        badadd("the उन्नतिः run scheduled exactly as the द्वारम् run did, and they differ " \
               "by the two words in नीतिः — so either श्रेणी was not installed or the " \
               "levels it keeps are not reaching a decision")

    twin("यमज", "द्वारम्", 3, 6, "C-002e3")
    twin("प्रेषकयमज", "प्रेषकः", 4, 7, "C-002e3c")
    if (CH[3] != CH[2] || AC[3] != AC[2])
        badadd("the द्वारम् run installed " fmtx(CH[3]) "/" fmtx(AC[3]) " and not चक्रम्'s " \
               fmtx(CH[2]) "/" fmtx(AC[2]) " — the order a wait queue comes out in must not " \
               "be attributable to a policy that sorts, and चक्रम् is the one that cannot")

    # ---- उपकार्यम् (run 8) ----
    structural("उपकार्यम्", 8)
    if ((CH[8] == CH[1] && AC[8] == AC[1]) || (CH[8] == CH[2] && AC[8] == AC[2]))
        badadd("the उपकार्यम् run installed " fmtx(CH[8]) "/" fmtx(AC[8]) ", which is one of " \
               "the two policies already in the image. The seam's claim is that a THIRD " \
               "thing can be written into those two words, and a policy that is already " \
               "there has not been shown to be replaceable by anything new")
    if (sameseq(8, 1) || sameseq(8, 2))
        badadd("the उपकार्यम् run scheduled in exactly the order " \
               (sameseq(8, 1) ? "श्रेणी" : "चक्रम्") " did. Then nothing " \
               "distinguishes a decision taken in userspace from one the kernel took " \
               "itself, which is the only thing this run exists to show")
    nacts = 0
    for (t = 1; t <= NT[8]; t++) if (EVT[8,t] == 5) ACTS[++nacts] = t
    if (!nacts)
        badadd("no turn of the उपकार्यम् run carries event 5 — the kernel never " \
               "called out at all, so whatever chose these threads was in the image's " \
               "kernel and the upcall is a name for a routine")
    for (k = 1; k <= nacts; k++) {
        t = ACTS[k]
        if (GOT[8,t] != 3)
            badadd("turn " t " of the उपकार्यम् run is marked an upcall and thread " GOT[8,t] \
                   " ran on it, not नियामकः (3) — the call went somewhere, but " \
                   "not out to the scheduler it was addressed to")
        if (WANT[8,t] == 3)
            badadd("turn " t " of the उपकार्यम् run is marked an upcall and चयनम् had " \
                   "NAMED नियामकः. Then it is an ordinary turn wearing the label: an " \
                   "upcall is the case where the thread that runs is the one the policy " \
                   "went out to ASK, and it cannot have asked for itself")
    }
    for (t = 1; t <= NT[8]; t++) {
        if (WANT[8,t] == 3 && EVT[8,t] != 5)
            badadd("turn " t " of the उपकार्यम् run chose नियामकः as ordinary work. It has " \
                   "no work; it is the party being asked, and a scheduler that competes " \
                   "for the slice it is handing out is not the seam doc 11 §3.4 asks for")
        if (EVT[8,t] == 5) continue
        s = ""; j = 0; mx = -1
        for (i = 0; i < 4; i++) if (OFF[8,t,i] && i != 3) { j++; s = s (j > 1 ? ", " : "") i; mx = i }
        if (j && WANT[8,t] != mx)
            badadd("turn " t " of the उपकार्यम् run offered [" s "] and चयनम् named " WANT[8,t] ", " \
                   "not " mx ". नियामकः decides by highest runnable index — a rule " \
                   "neither श्रेणी nor चक्रम् can express, because neither has any notion " \
                   "of index order — and a choice that does not follow it was not made by " \
                   "the thread this run says made it")
    }
    if (NT[8]) {
        ok = 1
        for (i = 0; i < 4; i++) if (LV[8,1,i] != 0 || WK[8,1,i] != ((i == 3) ? 0 : 10)) ok = 0
        if (!ok)
            badadd("the उपकार्यम् run began at levels " lvrepr(8,1) " works " wkrepr(8,1) ", expected " \
                   "[0, 0, 0, 0] / [10, 10, 10, 0] — नियामकः must start with no work of its own " \
                   "and the other three equal, or the order below is a level's doing")
        if (MASK[8,1] != 7)
            badadd("the उपकार्यम् run's first choice was made from " fmtbn(MASK[8,1]) ", expected " \
                   "0b0111 — नियामकः starts PARKED on the endpoint, so it is never " \
                   "in the set, and a run that starts elsewhere is a different program")
        if (NT[8] >= 63)
            badadd("the उपकार्यम् run took " NT[8] " turns and stopped on the dispatcher's " \
                   "budget rather than because the work ran out")
        for (i = 0; i < 4; i++) ER[i] = 0
        for (t = 1; t <= NT[8]; t++) ER[WANT[8,t]] = 1
        s = ""
        for (i = 0; i < 4; i++) if (i != 3 && !ER[i]) s = s (s == "" ? "" : ", ") i
        if (s != "")
            badadd("the उपकार्यम् run never ran thread(s) [" s "] — a userspace " \
                   "scheduler that starves a thread outright is not bounded scheduling, " \
                   "it is no scheduling")
    }
    if (BXD[8,3,3] != "9")
        badadd("नियामकः's inbox carries sender " BXD[8,3,3] " and not 9. The " \
               "identity in the fourth word is what says the call came from the KERNEL " \
               "rather than from another thread, and a sender that is one of the four " \
               "threads is the rendezvous this program already had")
    if (nacts) {
        wset = MASK[8,ACTS[nacts]]
        if (BX[8,3,0] != sprintf("%016x", wset))
            badadd("the last upcall offered " fmtbn(wset) " and नियामकः's inbox holds " \
                   fmtb(BX[8,3,0]) " in its first word. The set the dispatcher " \
                   "built and the set the scheduler was told are printed by two different " \
                   "parties, and the whole claim is that the second is the first — if it " \
                   "is not, नियामकः decided about a machine that does not exist")
    }
    for (k = 0; k < 4; k++)
        if (k != 3 && !boxunsent(8, k))
            badadd("thread " k "'s inbox in the उपकार्यम् run came out " boxrow(8,k) ", and " \
                   "nobody sent it anything — the upcall speaks to नियामकः alone")

    # ---- मौनम् (run 9) ----
    structural("मौनम्", 9)
    if (CH[9] != CH[8] || AC[9] != AC[8])
        badadd("the मौनम् run installed " fmtx(CH[9]) "/" fmtx(AC[9]) " and the उपकार्यम् run " \
               fmtx(CH[8]) "/" fmtx(AC[8]) " — this run is the eighth one with a SILENT " \
               "नियामकः, so the two words in नीतिः must be the same two words. A kernel " \
               "given a different policy for the case is a kernel that was told in " \
               "advance, and the claim is that it finds out")
    if (NT[9] && NT[8] && TW[9,1] != TW[8,1])
        badadd("the मौनम् run's first turn is " TW[9,1] " and the उपकार्यम् run's is " \
               TW[8,1] ". Nothing has been answered yet on turn 1 and the kernel " \
               "cannot know anything, so the first question must be the same question — " \
               "the two runs may diverge only after an answer failed to arrive, or what " \
               "changed was the table and not the silence")
    nasks = 0; ngave = 0
    for (t = 1; t <= NT[9]; t++) {
        if (EVT[9,t] == 5) ASKS_[++nasks] = t
        if (EVT[9,t] == 6) GAVE[++ngave] = t
    }
    if (!nasks)
        badadd("no turn of the मौनम् run carries event 5 — the kernel never asked " \
               "at all, so there is no silence here to survive, only a kernel deciding " \
               "for itself under another name")
    ok = 1
    for (k = 1; k <= nasks; k++) if (ASKS_[k] != k) ok = 0
    if (nasks && !ok)
        badadd("the मौनम् run's upcall turns are " list_from(ASKS_, nasks) " and not the first " nasks \
               " turns of the run. उपकार्यचयनम् consumes an answer when it uses one, so " \
               "nothing can run between an unanswered question and the next ask; a work " \
               "turn in among them means something was answered and this run is not the " \
               "silence it says it is")
    if (nasks && nasks != 3)
        badadd("the मौनम् run asked " nasks " times before giving up and the bound is " \
               "3. Asking again is the only evidence in this log that the last " \
               "question went unanswered, so one ask proves no silence — and a bound that " \
               "is not the stated one is not a bound the program holds itself to")
    for (k = 1; k <= nasks; k++) {
        t = ASKS_[k]
        if (GOT[9,t] != 3 || WANT[9,t] == 3)
            badadd("turn " t " of the मौनम् run is marked an upcall, चयनम् named " WANT[9,t] " and " \
                   "thread " GOT[9,t] " ran — an upcall is the case where the thread that runs " \
                   "is the one that was asked (3) and it cannot have asked " \
                   "itself")
        if (USED[9,t])
            badadd("turn " t " of the मौनम् run is an upcall and " USED[9,t] " units were eaten. " \
                   "नियामकः has no work of its own; a scheduler that eats the slice it " \
                   "was handed to allocate is doing something other than being asked")
    }
    if (ngave != 1)
        badadd(ngave " turns of the मौनम् run carry event 6 and exactly one " \
               "must. Giving up is a thing that happens once: a run that never declares " \
               "it is bounded only by this file's SIGKILL, which is the harness rescuing " \
               "the machine, and a run that declares it repeatedly has renamed its loop")
    if (ngave && nasks && GAVE[1] != ASKS_[nasks] + 1)
        badadd("the मौनम् run gave up on turn " GAVE[1] " and its last ask was turn " \
               ASKS_[nasks] " — the give-up is the turn the last unanswered question runs " \
               "out on, and a gap between them is turns the kernel spent neither " \
               "asking nor deciding")
    if (ngave) {
        s = ""; j = 0
        for (k = 1; k <= nasks; k++) if (ASKS_[k] > GAVE[1]) { j++; s = s (j > 1 ? ", " : "") ASKS_[k] }
        if (j)
            badadd("the मौनम् run asked again on turn(s) [" s "] " \
                   "after declaring the upcall broken on turn " GAVE[1] " — a kernel that " \
                   "keeps asking a scheduler it has already given up on has not defined an " \
                   "outcome")
    }
    for (t = 1; t <= NT[9]; t++) {
        if (WANT[9,t] == 3 && EVT[9,t] != 5)
            badadd("turn " t " of the मौनम् run chose नियामकः as ordinary work. It has no " \
                   "work, and a silent scheduler is still not a candidate for the slice " \
                   "it stopped handing out")
        if (EVT[9,t] == 5 || !ngave || t < GAVE[1]) continue
        s = ""; j = 0; mn = -1
        for (i = 3; i >= 0; i--) if (OFF[9,t,i] && i != 3) mn = i
        for (i = 0; i < 4; i++) if (OFF[9,t,i] && i != 3) { j++; s = s (j > 1 ? ", " : "") i }
        if (j && WANT[9,t] != mn)
            badadd("turn " t " of the मौनम् run is after the give-up, offered [" s "], and " \
                   "चयनम् named " WANT[9,t] " rather than " mn ". The fallback is the " \
                   "LOWEST runnable index — the exact inverse of नियामकः's rule, so that " \
                   "no fallback turn can be read as an answered one — and a choice that " \
                   "follows neither rule was made by nobody this run names")
    }
    if (NT[9] && sameseq(9, 8))
        badadd("the मौनम् run scheduled in exactly the order the उपकार्यम् run did. The " \
               "fallback is the inverse of the rule नियामकः answers by, so identical " \
               "orders mean either the silence changed nothing or somebody is still " \
               "answering")
    if (NT[9]) {
        ok = 1
        for (i = 0; i < 4; i++) if (LV[9,1,i] != 0 || WK[9,1,i] != ((i == 3) ? 0 : 10)) ok = 0
        if (!ok)
            badadd("the मौनम् run began at levels " lvrepr(9,1) " works " wkrepr(9,1) ", expected [0, 0, 0, 0] / " \
                   "[10, 10, 10, 0] — it is the उपकार्यम् run's table with a silent नियामकः, and " \
                   "a different table makes it a different program instead")
        if (NT[9] >= 63)
            badadd("the मौनम् run took " NT[9] " turns and stopped on the dispatcher's " \
                   "budget. Running out of budget is the machine being switched off, not " \
                   "an outcome it defined: the whole of this run is that the work still " \
                   "finishes when the scheduler goes quiet")
        for (i = 0; i < 4; i++) ER[i] = 0
        for (t = 1; t <= NT[9]; t++) ER[WANT[9,t]] = 1
        s = ""
        for (i = 0; i < 4; i++) if (i != 3 && !ER[i]) s = s (s == "" ? "" : ", ") i
        if (s != "")
            badadd("the मौनम् run never ran thread(s) [" s "] — a kernel whose fallback " \
                   "starves a thread outright has survived the silence and lost the " \
                   "machine anyway")
        t = NT[9]
        anyl = 0; endw = "["
        for (i = 0; i < 4; i++) {
            cnt = WK[9,t,i] - ((i == WANT[9,t]) ? USED[9,t] : 0)
            endw = endw (i ? ", " : "") cnt
            if (cnt != 0) anyl = 1
        }
        endw = endw "]"
        if (anyl)
            badadd("the मौनम् run ended with work " endw " still outstanding — it stopped " \
                   "scheduling while threads were still runnable, which is the silence " \
                   "reaching the work rather than being contained")
    }
    if (BXD[9,3,3] != "9")
        badadd("in the मौनम् run नियामकः's inbox carries sender " BXD[9,3,3] " and " \
               "not 9 — it was asked 3 times before the kernel gave up, so " \
               "the words of the last question must be there with the kernel's identity " \
               "on them; an empty inbox means the silence was never even asked into")
    if (nasks) {
        wset = MASK[9,ASKS_[nasks]]
        if (BX[9,3,0] != sprintf("%016x", wset))
            badadd("the मौनम् run's last question offered " fmtbn(wset) " and नियामकः's " \
                   "inbox holds " fmtb(BX[9,3,0]) " — the set the dispatcher built " \
                   "and the set the scheduler was handed are printed by two parties and " \
                   "the claim is that they are one set")
    }
    for (k = 0; k < 4; k++)
        if (k != 3 && !boxunsent(9, k))
            badadd("thread " k "'s inbox in the मौनम् run came out " boxrow(9,k) ", and nobody " \
                   "sent it anything — the upcall speaks to नियामकः alone, silent or not")

    # ---- अनशनम् (run 10) ----
    structural("अनशनम्", 10)
    if (CH[10] != CH[2] || AC[10] != AC[2])
        badadd("the अनशनम् run installed " fmtx(CH[10]) "/" fmtx(AC[10]) " and चक्रम् is " \
               fmtx(CH[2]) "/" fmtx(AC[2]) ". Round-robin alone is what this run must be " \
               "ordered by: it never reads a level, so no part of the order can be laid " \
               "at the door of the lift a blocking thread earns — that is C-002e6's " \
               "claim, made once, over a single block")
    setinit(3)
    invariants("अनशनम्", 10, 0)
    nrel = 0
    for (t = 1; t <= NT[10]; t++) if (OFF[10,t,3] && (t == 1 || !OFF[10,t-1,3])) REL[++nrel] = t
    nsb = 0
    for (t = 1; t <= NT[10]; t++) if (WANT[10,t] == 3 && EVT[10,t] == 1) SB[++nsb] = t
    if (nrel != 3)
        badadd("thread 3 came back into the set " nrel " time(s) in the " \
               "अनशनम् run and 3 are asked for. One block is an incident and two " \
               "a coincidence: it takes a third before coming round is a property of the " \
               "scheduler rather than a thing that happened once")
    if (nsb < 2)
        badadd("thread 3 blocked itself on turn(s) " list_from(SB, nsb) " of the अनशनम् run " \
               "and at least 2 are asked for. A thread blocked at startup and " \
               "released has been shown to come back ONCE; that it goes back down again by " \
               "its own action is the whole of 'blocks, is released and blocks again'")
    for (k = 1; k <= nsb; k++) {
        t = SB[k]; ok = 0
        for (j = t + 1; j <= NT[10]; j++) if (!OFF[10,j,3]) ok = 1
        if (!ok)
            badadd("thread 3 blocked itself on turn " t " of the अनशनम् run and was " \
                   "in the set on every turn after it — a block that costs no turn is a " \
                   "label, and there is then no lost turn to give back")
    }
    for (k = 1; k <= nrel; k++) {
        t = REL[k]; later = 0
        for (j = t; j <= NT[10]; j++) if (WANT[10,j] == 3) { later = j; break }
        if (!later) {
            badadd("thread 3 was put back into the set on turn " t " of the अनशनम् " \
                   "run with work left and was never chosen again in " NT[10] " turns — " \
                   "starved outright, which is the case C-002i could not reach and the " \
                   "reason this run exists")
            continue
        }
        for (i = 0; i < 4; i++) cntr[i] = 0
        for (j = t; j <= later - 1; j++) cntr[WANT[10,j]]++
        s = ""
        for (i = 0; i < 4; i++) if (cntr[i] > 1) s = s (s == "" ? "" : ", ") i
        if (s != "")
            badadd("thread 3 came back into the set on turn " t " of the अनशनम् run " \
                   "and was not chosen until turn " later ", and thread(s) [" s "] ran " \
                   "twice in between. The turn it lost by waiting went to somebody who " \
                   "had already had one — that is the bound, and it is the same bound " \
                   "starvation() states for a thread that never leaves the set, asked " \
                   "across the block instead of around it")
    }
    if (!rowis(10, 3, "12", "12288", "32832", "0"))
        badadd("thread 3's inbox in the अनशनम् run holds " boxrow(10,3) " and the " \
               "third message with its sender is [12, 12288, 32832, 0]. " \
               "The releases carry the three rows in send order, so the last one is fixed " \
               "by the same table the द्वारम् run's are — an inbox that holds something " \
               "else was filled by something other than the release this run counts")
    for (k = 0; k < 4; k++)
        if (k != 3 && !boxunsent(10, k))
            badadd("thread " k "'s inbox in the अनशनम् run came out " boxrow(10,k) ", and nobody " \
                   "sent it anything — thread 0 speaks to the cycler alone here")
    AN[1] = "श्रेणीचयनम्"; AV[1] = CH[1]
    AN[2] = "श्रेणीगणनम्"; AV[2] = AC[1]
    AN[3] = "चक्रचयनम्"; AV[3] = CH[2]
    AN[4] = "चक्रगणनम्"; AV[4] = AC[2]
    AN[5] = "उपकार्यचयनम्"; AV[5] = CH[8]
    AN[6] = "उपकार्यगणनम्"; AV[6] = AC[8]
    for (k = 1; k <= 6; k++)
        if (!(substr(AV[k], 1, 8) == "00000000" && substr(AV[k], 9, 2) == "80"))
            badadd("नीतिः held " fmtx(AV[k]) " for " AN[k] ", which is not an address in the loaded image")
    ok = 1
    for (k = 1; k <= 6; k++) for (j = k + 1; j <= 6; j++) if (AV[k] == AV[j]) ok = 0
    if (!ok) {
        s = ""
        for (k = 1; k <= 6; k++) s = s (k > 1 ? ", " : "") AN[k] "=" fmtx(AV[k])
        badadd("the six policy entry points are not six distinct addresses: " s)
    }

    if (NB) {
        s = BAD[1]
        for (k = 2; k <= NB; k++) s = s "\n" BAD[k]
        print s
        exit 1
    }
    nA = NT[1]; nB = NT[2]; nC = NT[3]; nD = NT[4]; nE = NT[5]
    nF = NT[6]; nG = NT[7]; nH = NT[8]; nI = NT[9]; nJ = NT[10]
    print "श्रेणी ran " nA " turns and चक्रम् " nB ", both finishing all the work; priority " \
          "ordered every श्रेणी turn, thread 3 climbed " drops " levels by behaving and then " \
          "ran ahead of threads still stuck below it, and the two orders differ. द्वारम् ran " \
          nC ": threads [2, 1, 3] sat down on the endpoint in that order, came back in it, " \
          "and each came back holding the message sent to the head at the time. प्रेषकः ran " \
          nD ": thread 0 sent into an empty endpoint and had to sit down on it itself, " \
          "and thread 3 took the four words out of it without waiting at all. उन्नतिः " \
          "ran " nE ": the same run as द्वारम् with श्रेणी installed, and every thread that " \
          "sat down below the top level was lifted for eating nothing and then ran ahead of " \
          "a thread standing where the block had found it. यमज ran " nF ": the द्वारम् run " \
          "again with the fast path installed, word for word except on turns " \
          "[" HOSTR[6] "], where a thread ran without चयनम् naming it — the " \
          "same threads, in the same order, eating the same units, on fewer dispatcher " \
          "entries. प्रेषकयमज ran " nG ": the प्रेषकः run again with the fast path " \
          "installed, word for word except on turns [" HOSTR[7] "] — the " \
          "same difference in the receive direction, where it is the arriving receiver " \
          "that hands straight back to the sender it just unparked. उपकार्यम् ran " nH ": " \
          "a third policy in the same two words, " nacts " of those turns a call OUT to " \
          "नियामकः, and every other turn the highest-numbered runnable thread — a rule no " \
          "kernel policy in this image can express. मौनम् ran " nI ": the same run with a " \
          "नियामकः that answers nothing, asked " nasks " times, given up on at turn " \
          (ngave ? GAVE[1] : "-") " by the program's own bound rather than by this " \
          "file's SIGKILL, and every thread's work finished anyway. अनशनम् ran " nJ ": thread " \
          "3 went down onto the endpoint and came back " nrel " times, and on " \
          "none of those returns did another thread take a second turn ahead of it — the " \
          "fairness the other runs state for a thread that never leaves the set, asked " \
          "across the block"
    printf "MOVES"
    printf " %s", CH[1]; printf " %s", AC[1]; printf " %s", CH[2]; printf " %s", AC[2]; printf " %s", CH[8]; printf " %s", AC[8]
    printf "\n"
    printf "FIXED"
    for (r = 1; r <= 10; r++) {
        printf " %016x", NT[r]
        for (t = 1; t <= NT[r]; t++) printf " %s", TW[r,t]
    }
    for (r = 1; r <= 10; r++) for (k = 0; k < 4; k++) for (j = 0; j < 4; j++) printf " %s", BX[r,k,j]
    printf "\n"
}

{ L[NR] = $0 }
END { main() }
