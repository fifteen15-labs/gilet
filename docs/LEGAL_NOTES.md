# Anorak — legal research notes

**This is research, not legal advice.** I am not a solicitor and this is not a legal
opinion. It is a sourced survey of the statutes, cases and observable industry
practice that bear on publishing a third-party Football Manager save reader. It has
not been reviewed by anyone qualified. If real money or a real threat letter ever
enters the picture, pay a solicitor who does IP and software licensing — this
document is a starting brief for that conversation, not a substitute for it.

Jurisdiction: **UK-focused** (the developer is UK-based, the EULA is governed by
English law). Section 9 adds a short US note, because a public website reaches
everywhere.

Every substantive claim below carries a URL. Where I could not find a primary
source, I say so rather than inferring.

Confidence labels used throughout:

- **Settled** — black-letter statute or binding precedent directly on point.
- **Arguable** — a reasonable, well-supported reading, but the other side has a case.
- **Uncertain** — genuinely unresolved; nobody should be confident.

---

## 0. Summary

**Personal use.** Fine, and on firmer ground than he probably assumes. He did not
decompile anything, no encryption or DRM was bypassed, the file format is not
protected by copyright (*SAS Institute v World Programming*), and CDPA s296A voids the
EULA's anti-reverse-engineering clause to the extent it purports to restrict observing
and studying. The EULA itself concedes the point — clause 7(e) says "except where
permitted by law". **Settled.**

**Publishing with donations.** Legally it changes surprisingly little; practically it
changes the exposure a lot. The donation link is close to the *least* risky element —
it does not engage the EULA's non-commercial clauses (those attach to use of *the
Product*, not to software he writes), and the database-right exception he relies on
(reg 19) has no non-commercial limitation. What publishing actually changes is
**visibility**: it puts him on SEGA's radar and gives them cheap, non-judicial levers.

**The one clause that genuinely bites** is EULA §7(h) — "create data or executable
programs which mimic data or functionality in the Product unless such functionality is
provided to you in the Editors". Unlike 7(e) it has **no "except where permitted by
law" carve-out**, and s296A probably does not reach it because it restricts *creating a
program* rather than *observing* one. Whether it is enforceable is **genuinely
uncertain**. But breach of it is a **contract** claim, not copyright — nominal damages,
no statutory damages, no criminal exposure.

**Precedent is strongly reassuring and legally worthless.** A full-corpus search of
GitHub's 21,613 published DMCA notices returns **zero** matches for Sports Interactive,
Football Manager, FMRTE or Genie Scout. FM Genie Scout has run a donation-gated model
for 15+ years while advertising that it reveals hidden attributes; FMRTE has *sold* a
save editor since 2008 while admitting in its own EULA that this "might be against"
SI's EULA. SI hosts an "Editors Hideaway" forum and a live 2011 Genie Scout
advertisement thread on its own domain. None of this is a licence, and non-enforcement
creates no estoppel — but it is a well-founded basis for expecting to be left alone.

**Top three precautions:**

1. **Never ship game data, and cap CSV export at shortlists.** No bundled saves, no
   databases, no "export all players". This keeps every extraction the *user's* lawful
   act under CDPA s50D and reg 19, and it is the one line the tool community itself
   polices (Genie Scout declines bulk export on EULA grounds).
2. **Name and market it carefully.** Keep "Anorak"; put "Football Manager" only in a
   descriptive strapline (TMA 1994 s11(2)(c)); no logos, no mark in the domain, and a
   clear "not affiliated with or endorsed by Sports Interactive or SEGA" disclaimer.
   Never position it as an alternative to SI's paid in-game editor — that framing
   costs him both the trade mark defence and SI's goodwill.
3. **Stay read-only and never circumvent anything.** No process attachment, no memory
   reading, no touching Denuvo — and **stop immediately if SI ever encrypts the save
   format**, because that single change flips both the UK and US analyses from "no
   technological measure exists" to "circumvention".

**Biggest legal risk:** EULA clause 7(h), as a breach of contract claim (§10).
**Biggest practical risk:** unilateral platform action — a Steam/SEGA account ban or a
complaint to Apple — which needs no lawyer, no notice and no proof, and against which
being legally right is no protection (§10).

---

## 1. The factual predicate

The legal analysis is only as good as the facts, so these are stated up front. They
come from the project's own `docs/SAVE_FORMAT.md` and `CLAUDE.md`.

| Fact | Legal significance |
| --- | --- |
| Reads `.fm` save files from the user's own disk | No access to SI servers, no unauthorised access |
| **Read-only** — never writes to saves | Not a "modification" of the Product |
| Does not attach to the game process, no `ReadProcessMemory`, no code injection | Distinguishes it from most existing FM tools (see §7) |
| Does not decompile or disassemble the game executable | **Critical** — takes s50B and the *LzLabs* problem off the table |
| Save is **zstd-compressed, not encrypted** | Strong argument no technological protection measure exists |
| No licence check, DRM or access control bypassed | Denuvo protects the executable; Anorak never touches it |
| Ships **no game data** — user must own the game and supply their own save | No distribution of SI's copyright works or database |
| Format derived by inspecting bytes in his own save files | Observation of program *output*, not of program *code* |
| Goal includes surfacing Current Ability / Potential Ability, which the game hides | The most legally and commercially provocative feature |

Two points worth flagging because they change the analysis:

**CA/PA is not yet implemented.** `SAVE_FORMAT.md` §6 records it as "not yet
located". The briefing below treats it as an intended feature, because that is the
stated goal, but nothing has shipped yet that reveals hidden data. That matters for
timing: the riskiest feature is still ahead, not behind.

**The tool is at the conservative end of the spectrum.** The project's own prior-art
notes record that FM Scouting Tool 26 and `robeady/fm-explorer` both read the running
game's memory via `OpenProcess`/`ReadProcessMemory` against `game_plugin.dll`. Those
are considerably more invasive techniques than parsing a file at rest, and both are
publicly distributed. `robeady/fm-explorer` is a live public GitHub repository with no
takedown notice on it
([github.com/robeady/fm-explorer](https://github.com/robeady/fm-explorer)).

---

## 2. Question 1 — The EULA

### 2.1 Which document actually applies

This took some digging and the answer is slightly awkward.

- The FM26 Steam store page links to a third-party EULA at
  [store.steampowered.com/eula/3551340_eula_0](https://store.steampowered.com/eula/3551340_eula_0).
- That page is not itself a EULA. It is a redirect card whose only links are to
  **`https://privacy.sega.com/en/fm_eula`** and `https://privacy.sega.com/en/fm_privacy`.
- `privacy.sega.com/en/fm_eula` serves the **SEGA Europe End User License Agreement,
  effective 12 December 2024**
  ([privacy.sega.com/en/sega-europe-end-user-license-agreement](https://privacy.sega.com/en/sega-europe-end-user-license-agreement)).

**There is no FM26-specific EULA document that I could find.** SEGA published
game-specific EULAs for FM23 and FM24
([FM24](https://privacy.sega.com/en/fm24-eula-end-user-license-agreement),
[FM23](https://privacy.sega.com/en/fm23-eula-end-user-license-agreement)) but
`privacy.sega.com/en/fm26-eula-end-user-license-agreement` returns **HTTP 404**. The
operative document for FM26 appears to be the general SEGA Europe EULA. Confidence
that this is the right document: **arguable, not settled** — it is what the Steam
link resolves to, which is good evidence, but I could not find SEGA saying so in
terms. Anyone relying on this should re-check the link, because SEGA moves these.

The FM23/FM24 texts and the general SEGA Europe text are materially identical on
every clause that matters here, so the ambiguity does not change the analysis.

### 2.2 The clauses that matter

**Licence grant — Section 2:**

> "SEGA hereby grants you a non-exclusive, non-transferable, limited, fully revocable
> right and license to install, access and use one (1) copy of the Product solely and
> exclusively for your **personal and non-commercial use**."

**Section 7 — License Conditions. "You SHALL NOT:"**

> "(a) exploit the Product or any of its parts commercially, including, but not
> limited to, at a cyber (Internet) café, computer gaming centre or any other
> location-based site"

> "(e) reverse engineer, derive source code, modify, decompile, disassemble, copy, or
> create derivative works of the Product, in whole or in part, **except where
> permitted by law**"

> "(f) remove, disable or circumvent any security protections, proprietary notices or
> labels contained on or within the Product"

> "(h) **create data or executable programs which mimic data or functionality in the
> Product unless such functionality is provided to you in the Editors**"

**"Editors" is a defined term:**

> "the editing software you have just downloaded or any part of the Game Software or
> any third party software **authorised for use with the Game Software by SEGA** which
> allows you to construct new variations, modifications, derivations, adaptations,
> copies or improvements of the Game Software"

**Section 18 — Technical Protection Measures:**

> "This Product may be protected by anti-cheat/hacking software and/or Denuvo
> Anti-Tamper Protection Technology." … "If you disable or otherwise tamper with the
> Denuvo Anti-Tamper Technology, the Product may not operate properly and you are in
> material breach of this Agreement."

**Section 21 — Governing law:** England and Wales, exclusive jurisdiction of the
English courts (California law and arbitration for US/Canada residents).

### 2.3 Reading these against Anorak

**Does it prohibit reverse engineering outright? No — 7(e) is expressly subject to
"except where permitted by law".** That carve-out is doing enormous work, and §3
below is about exactly what it lets through. Confidence: **settled** (it is the plain
text).

**Does it mention third-party tools or save files? Not directly.** I searched the full
text: there is **no clause about save files, save games, or game data**. There is no
clause about third-party utilities as such. Section 6 covers user-generated
audio-visual content (streams, Let's Plays) and expressly excludes Mods, which go to
Schedule 2.

**The clause actually aimed at Anorak is 7(h), not 7(e).** This is the single most
important finding in this section and it is easy to miss. Clause 7(h) prohibits
creating "data or executable programs which mimic data or functionality in the
Product" unless the functionality is provided in an SEGA-authorised Editor. Anorak is
an executable program that reproduces functionality found in the Product (looking up
players, reading their attributes) and, in its intended form, surfaces data the
Product holds. On a literal reading, 7(h) bites.

Three things make 7(h) worse than 7(e) for him:

1. **It has no "except where permitted by law" carve-out.** 7(e) does; 7(h) does not.
2. **It is not obviously caught by the statutory void-ing provisions.** CDPA s296A
   voids terms restricting *observing, studying and testing*, back-up copies and
   s50B decompilation. 7(h) does not restrict observation — it restricts the
   *creation of a program*. That is a different act, and s296A does not plainly reach
   it. See §3.5.
3. **The "Editors" definition is circular in SEGA's favour.** Functionality is
   permitted only via software "authorised for use … by SEGA". SEGA has not
   authorised Anorak, so by definition the exception does not apply.

Confidence that 7(h) is the primary contractual exposure: **arguable, and I would
put it high.** Confidence about whether 7(h) is *enforceable*: **uncertain** — see
§3.5 and §3.6.

### 2.4 The Steam Subscriber Agreement

Relevant if he acquired the game through Steam
([store.steampowered.com/subscriber_agreement](https://store.steampowered.com/subscriber_agreement/)):

- **§2.A** grants a licence "for your personal, non-commercial use".
- **§2.G** — "you may not, in whole or in part, copy, photocopy, reproduce, publish,
  distribute, translate, reverse engineer, derive source code from, modify,
  disassemble, decompile, create derivative works based on…"
- **§4.B** — "You agree that you will not create Cheats or assist third parties in
  any way to create or use Cheats".
- **§10** — for EU/UK users, "This Agreement is governed by the law of the country
  where you have your habitual residence", i.e. English law for him.

Two observations. First, §2.G has **no "except where permitted by law" carve-out**,
which makes it more absolute on its face than SEGA's 7(e) — but the CDPA voids
offending terms regardless of whether the drafter remembered to carve them out
(s296A operates on the term, not on the drafter's intention). Second, the "Cheats"
prohibition in §4.B is aimed at multiplayer integrity; FM is predominantly
single-player and a read-only viewer of one's own save is a stretch from what §4.B
targets. But it is not a zero — a tool whose selling point is revealing values the
designer deliberately concealed is closer to the spirit of "cheat" than a mod
manager is. Confidence: **arguable**.

---

## 3. Question 2 — Does the anti-reverse-engineering clause bind him in the UK?

This is the crux, and the answer is more favourable than the EULA text suggests, but
for a narrower reason than "reverse engineering is legal in the UK".

### 3.1 The statutory scheme

**CDPA s50BA — observing, studying and testing**
([legislation.gov.uk](https://www.legislation.gov.uk/ukpga/1988/48/section/50BA)):

> "(1) It is not an infringement of copyright for a lawful user of a copy of a
> computer program to observe, study or test the functioning of the program in order
> to determine the ideas and principles which underlie any element of the program if
> he does so while performing any of the acts of loading, displaying, running,
> transmitting or storing the program which he is entitled to do.
>
> (2) Where an act is permitted under this section, it is irrelevant whether or not
> there exists any term or condition in an agreement which purports to prohibit or
> restrict the act (such terms being, by virtue of section 296A, void)."

**CDPA s296A — avoidance of certain terms**
([legislation.gov.uk](https://www.legislation.gov.uk/ukpga/1988/48/section/296A)):

> "(1) Where a person has the use of a computer program under an agreement, any term
> or condition in the agreement shall be void in so far as it purports to prohibit or
> restrict—
> (a) the making of any back up copy of the program which it is necessary for him to
> have for the purposes of the agreed use;
> (b) where the conditions in section 50B(2) are met, the decompiling of the program; or
> (c) the observing, studying or testing of the functioning of the program in
> accordance with section 50BA."

**CDPA s50B — decompilation**
([legislation.gov.uk](https://www.legislation.gov.uk/ukpga/1988/48/section/50B)):
permits converting a low-level-language program into a higher-level language where
necessary to obtain information to create an independent interoperable program, and
where the information is not used for any other purpose. s50B(3) disapplies the
conditions where the user "has readily available to him the information necessary to
achieve the permitted objective".

**s50A** covers back-up copies by a lawful user.

### 3.2 Why this matters less than it looks — he did not decompile anything

The question asks me to address this head-on, so: **s50B is irrelevant to Anorak, and
that is good news, not bad.**

s50B is the messy, condition-laden provision. It only applies to *decompilation* —
converting object code into something higher-level. It comes with the "permitted
objective" test, the "readily available" bar, and the prohibition on using the
information for any other purpose. It is where reverse engineers get into trouble.

Anorak never decompiled the FM executable. It read a data file the program wrote. So
none of s50B's conditions need to be satisfied, and none of its traps apply.

### 3.3 Does s50BA cover reading a *data file* rather than the *program*?

This is the genuinely interesting question and the honest answer is: **it does not
need to, and it probably does anyway.**

**The "it does not need to" argument (stronger).** s50A–50BA are *defences to
copyright infringement in a computer program*. You only need a defence if you have
done something that would otherwise infringe. What has he actually done?

- He opened a file on his own disk and looked at the bytes.
- He decompressed it with a standard, publicly documented codec (zstd).
- He worked out what the fields mean.

Looking at bytes in a file you lawfully possess is not, without more, a restricted
act under s16 CDPA. He is not reproducing the FM *program*; he is not issuing copies
to the public; he is not adapting it. The only copying is of the *save file's
contents* into his own RAM — and the save file's contents are analysed separately in
§4, because they are data and database material, not program code. If no restricted
act in the program occurs, s50BA is not needed. Confidence: **arguable, and I think
it is the stronger framing.**

**The "it probably does anyway" argument.** If a court did want a statutory hook,
s50BA fits reasonably well. A save file is the observable *output* of running the
program. Determining how a program encodes its state is squarely "determin[ing] the
ideas and principles which underlie any element of the program". And the condition —
that observation happen "while performing any of the acts of loading, displaying,
running, transmitting or storing the program which he is entitled to do" — is
satisfied: he ran the game to generate the saves.

The textual difficulty is that s50BA speaks of observing "the functioning of the
program", and one could argue that studying a file at rest, days later, with the game
closed, is not observing the program functioning. That is a real argument. But it is
a formalistic one, and it proves too much: on that reading, watching a program's
screen output would count but reading the file it just wrote would not, which makes
no sense as policy.

**The best authority for the output-vs-code line is *IBM v LzLabs*.** In
*IBM United Kingdom Ltd v LzLabs GmbH* [2025] EWHC 532 (TCC), O'Farrell J read
s50BA narrowly and found against the reverse engineers — but the line she drew is
precisely the line Anorak sits on the right side of:

> "A lawful user is entitled to observe output…to determine behaviour…They are not
> entitled to gain access to source or object code and reproduce the expression"

([RPC analysis](https://www.rpclegal.com/thinking/tech/reverse-engineering-of-ibm-mainframe-software-in-breach-of-software-licence-ibm-v-lzlabs-part-1/))

LzLabs lost because they disassembled IBM object code, transferred code fragments,
and recreated IBM data structures — the court called it "deliberate and systematic
disregard of the terms". Anorak does none of that. It observes output. Permission to
appeal was refused by the Court of Appeal on 4 July 2025, *LZLabs GmbH v IBM UK Ltd*
[2025] EWCA Civ 842
([Solicitors Journal](https://www.solicitorsjournal.com/sjarticle/lzlabs-v-ibm-court-of-appeal-refuses-permission-to-appeal-intellectual-property-breach)),
so the TCC judgment stands as the leading recent UK authority.

**I must present the other side fairly.** *LzLabs* is being written up as
"[Contractual Bars Trump Software-Directive Defences](https://www.casemine.com/commentary/uk/-lzlabs-v-ibm:-contractual-bars-trump-software-directive-defences-%E2%80%93-a-high-hurdle-for-fact%E2%80%93heavy-tcc-appeals-/view)".
The direction of travel in the most recent UK authority is *narrowing* the statutory
exceptions and *upholding* licence restrictions on activity that falls outside them.
That should temper any confidence drawn from *SAS*.

### 3.4 *SAS Institute v World Programming* — the most helpful case

This is the closest thing to authority in his favour, and it is strong.

**CJEU C-406/10** (2 May 2012) held that **the functionality of a computer program,
the programming language, and the format of data files are not protected by
copyright** — they do not constitute a form of expression of the program
([Wikipedia case summary](https://en.wikipedia.org/wiki/SAS_Institute_Inc_v_World_Programming_Ltd);
[Kluwer Copyright Blog](https://legalblogs.wolterskluwer.com/copyright-blog/decrypting-the-code-cjeu-sas-vs-world-programming/);
[RPC](https://www.rpclegal.com/thinking/ip/no-copyright-in-software-functionality-sas-v-wpl-the-final-chapter/)).

Note the caveat the court left open: data file formats *might* still be protected
under the InfoSoc Directive if they constitute the author's "own intellectual
creation". Nobody has successfully run that argument on a binary save format, and it
is a high bar after *Football Dataco* (§4.2), but it is not zero.

**What WPL actually did is the point.** WPL had **no access to SAS source code**. It
studied the program's *behaviour* and its manuals. That is methodologically the same
thing Anorak did — study observable behaviour and output, infer the structure, write
an independent implementation.

**And the licence restriction was held void.** Arnold J held that WPL's use of the
SAS Learning Edition fell within Article 5(3), and that to the extent SAS's licence
terms (which restricted the Learning Edition to non-production purposes) contradicted
that, **those terms were null and void under Article 9(1) of the Software Directive**
— the predecessor of Article 8 of 2009/24/EC, and the source of CDPA s296A
([Lexology](https://www.lexology.com/library/detail.aspx?g=1642296d-e897-4cb4-be06-4636b1f5cd85);
[8 New Square](https://8newsquare.co.uk/sas-institute-inc-v-world-programming-ltd-2013-ewca-civ-1482/)).

So the structure is: SAS wrote a licence term restricting what WPL could do with the
software; WPL did it anyway; the court held the term void because statute said the
activity was permitted. That is exactly the argument available against SEGA's 7(e).

Note the one thing SAS *won* on: WPL's **manual** infringed SAS's manual, because it
reproduced expression. The lesson generalises — copy behaviour and structure freely,
never copy text or assets.

### 3.5 The tension the statute does not resolve: clause 7(h)

**Where the EULA says one thing and statute says another, s296A wins — but only for
the acts s296A actually lists.** This is the gap, and it is the most important
unresolved point in this document.

| EULA clause | Restricts | Voided by s296A? |
| --- | --- | --- |
| 7(e) reverse engineer / decompile | observing, studying, decompiling | **Yes, to that extent.** s296A(1)(b) and (c) are directly on point, and 7(e) concedes it with "except where permitted by law". |
| 7(h) create programs that mimic data or functionality | *creating an independent program* | **Probably not.** s296A voids terms restricting observation, back-ups and s50B decompilation. Creating a program is none of those. |

So the likely position is: **SEGA cannot stop him working out the format, but may
still be able to say, as a matter of contract, that he agreed not to build this
thing.** That is an uncomfortable and genuinely unresolved place to land.

Counter-arguments available to him, none of them a slam dunk:

- **Article 8 / s296A purposive reading.** If the right to observe and study is
  meaningless unless you can act on what you learn, a term forbidding you to act on
  it restricts the observation "in so far as" it makes it pointless. *SAS* supports
  this in spirit — WPL's whole purpose was building a competitor, and the court did
  not treat that as taking it outside Article 5(3). Confidence: **arguable**.
- **Copyright cannot be extended by contract into unprotected subject matter.**
  Functionality and file formats are not protected (*SAS*). 7(h) tries to reclaim by
  contract what copyright expressly does not give. There is no UK authority striking
  a term down on that basis, so this is **uncertain**.
- **Consumer Rights Act 2015, Part 2.** He is a consumer. A term is unfair if,
  contrary to good faith, it causes a significant imbalance in the parties' rights to
  the detriment of the consumer (CRA 2015 s62), and Schedule 2 sets out an
  indicative "grey list"
  ([CMA guidance](https://assets.publishing.service.gov.uk/media/5a7f8b58ed915d74e33f716e/Unfair_Terms_Main_Guidance.pdf);
  [legislation.gov.uk s63 notes](https://www.legislation.gov.uk/ukpga/2015/15/section/63/notes)).
  A clause purporting to forbid a consumer from ever writing software that "mimics
  functionality" of a game he bought is a plausible candidate for unfairness. But I
  found **no case applying the CRA to a game EULA's reverse-engineering or
  anti-tooling clause**, so this is **uncertain and untested**.

### 3.6 Breach of contract is not copyright infringement — and that changes everything

This is the practical point that most reduces the risk, and it deserves its own
heading.

Even on the worst reading — 7(h) is valid, and he breached it — the consequence is a
**breach of contract claim by SEGA against him personally**, not copyright
infringement. That matters enormously:

- **Remedy is contractual.** SEGA's obvious remedy is to terminate his licence (the
  grant is "fully revocable") and seek an injunction. Damages for a hobbyist's
  breach of a game EULA would be very hard to quantify and plausibly nominal.
- **No copyright damages, no additional damages, no criminal exposure.** The
  criminal provisions in CDPA s107 attach to dealing in infringing copies. He is not
  distributing infringing copies of anything.
- **Privity.** The EULA binds *him*. It does not bind his users, except that they
  have their own EULAs. And it does not make the *software he wrote* unlawful — a
  breach of a personal covenant does not taint the artefact.
- **Forum.** Section 21 gives exclusive jurisdiction to the English courts. For SEGA
  that means real litigation costs to pursue a hobbyist over a free tool.

This is why the realistic threat is a **cease-and-desist letter, a Steam account ban,
or a platform takedown**, not a lawsuit. See §8.

### 3.7 The Software Directive and post-Brexit status

**Directive 2009/24/EC Article 5(3)**
([legislation.gov.uk](https://www.legislation.gov.uk/eudr/2009/24/article/5)):

> "The person having a right to use a copy of a computer program shall be entitled,
> without the authorisation of the rightholder, to observe, study or test the
> functioning of the program in order to determine the ideas and principles which
> underlie any element of the program if he does so while performing any of the acts
> of loading, displaying, running, transmitting or storing the program which he is
> entitled to do."

**Article 6** is the decompilation provision (the source of s50B).

**Article 8, second paragraph**
([legislation.gov.uk](https://www.legislation.gov.uk/eudr/2009/24/article/8)):

> "Any contractual provisions contrary to Article 6 or to the exceptions provided for
> in Article 5(2) and (3) shall be null and void."

**Post-Brexit status.** The Directive itself no longer has direct effect in the UK,
but this changes nothing practical, because it was implemented into the CDPA and
**ss50A–50BA, 296A remain in force unamended**. *SAS* (2012) is **retained EU case
law**: CJEU judgments handed down on or before 31 December 2020 remain binding on UK
courts, with the Supreme Court and Court of Appeal able to depart from them
([Travers Smith](https://www.traverssmith.com/knowledge/knowledge-container/retained-eu-law-10-key-questions/);
[Retained EU Law (Revocation and Reform) Act 2023](https://www.legislation.gov.uk/ukpga/2023/28)).
In any event the UK Court of Appeal applied *SAS* domestically in [2013] EWCA Civ
1482, so it is binding English precedent on its own footing. Confidence: **settled**.

### 3.8 Anti-circumvention — no TPM was bypassed

FM26 "Incorporates 3rd-party DRM: Denuvo Anti-tamper"
([Steam store page](https://store.steampowered.com/app/3551340/Football_Manager_26/)).
That sounds alarming and is not, for a specific reason.

**CDPA s296ZF(1)** defines technological measures as technology "designed, in the
normal course of its operation, to **protect** a copyright work", and **s296ZF(3)**
ties "protection" to "the prevention or restriction of acts that are not authorised
by the copyright owner"
([legislation.gov.uk](https://www.legislation.gov.uk/ukpga/1988/48/section/296ZF)).
**s296ZF(2)** treats a measure as "effective" where use is controlled "through an
access control or protection process such as encryption, scrambling or other
transformation of the work, or a copy control mechanism".

Applying that:

- **Denuvo protects the executable. Anorak never touches the executable.** No
  circumvention, on any reading. Confidence: **settled**.
- **zstd compression is not a TPM.** The honest risk here is the phrase "or other
  transformation", which compression arguably is. But the definitional limb is
  *purpose*: the measure must be "designed … to protect a copyright work" and to
  prevent unauthorised acts. Standard, publicly documented compression applied to
  make a 188 MB file into a 44 MB one is designed to save disk space. There is no
  key, no secret, no access control. Confidence: **arguable, and strongly so** — but
  I could not find a UK case deciding that plain compression is not a TPM, so it is
  not settled.
- Note s296ZA and s296ZF(1) both apply to "a copyright work **other than a computer
  program**". Computer programs have their own narrower regime in **s296**, which
  targets *dealing in* circumvention devices. Anorak is not a circumvention device.
- *Nintendo v PC Box* (C-355/12, 23 January 2014) held that video games are "complex
  works" falling under the InfoSoc Directive rather than being excluded as computer
  programs, that "effective technological measure" is construed broadly, **but** that
  protection is subject to **proportionality**, and that TPM protection should not
  inhibit devices with "a commercially significant, non-infringing purpose or use"
  ([Lexology](https://www.lexology.com/library/detail.aspx?g=ebc62273-e213-44da-b457-288752f4ea6a);
  [8 New Square](https://8newsquare.co.uk/nintendo-co-ltd-v-pc-box-srl-case-c-355-12/)).
  A read-only save viewer that requires you to own the game has an obvious
  substantial non-infringing purpose.

---

## 4. Question 3 — Copyright in the format vs the data

These are two completely different questions and conflating them is the most common
error in this area.

### 4.1 Is the file format protectable? Almost certainly not

**Settled, and this is the most solid ground in the whole document.**

*SAS Institute v World Programming* (CJEU C-406/10) holds that **the format of data
files is not a form of expression of a computer program and is not protected by
copyright** under the Software Directive
([Kluwer](https://legalblogs.wolterskluwer.com/copyright-blog/decrypting-the-code-cjeu-sas-vs-world-programming/)).
The reasoning: data file formats are the means by which users exploit the program's
functions, and functionality is not protected.

So: the fact that a person record is `u32 surname_id, u8, u32 common_name_id, u8,
u32 name_length, [u8] name, u16 day_of_year, u16 year` is **not** SI's copyright.
Knowing it is not infringing. Writing it down in `SAVE_FORMAT.md` is not infringing.
Publishing that documentation is not infringing.

The residual caveat from *SAS*: a format *might* attract protection under the InfoSoc
Directive if it is the author's "own intellectual creation". After *Football Dataco*
(below) killed the "skill and labour" test, a binary layout dictated by engineering
constraints is a poor candidate. Confidence that the format is unprotected:
**settled as to the Software Directive; arguable as to the residual InfoSoc point,
which nobody has won.**

### 4.2 Is the player data protectable? Partly — and it is messier

Three separate rights could sit in the FM database.

**(a) Copyright in the data itself.** Individual facts — that a player is called
Erling Braut Haaland and was born on 21 July 2000 — are not copyright works. Facts
are not protected. Confidence: **settled**.

**(b) Copyright in the database as a compilation.** Requires originality in the
*selection or arrangement* of contents. *Football Dataco v Yahoo!* (C-604/10, 1 March
2012) held that a compilation is protected only where, through selection or
arrangement, the author "expresses his creative ability in an original manner by
making free and creative choices", and that **significant labour and skill does not
in itself justify protection**
([Kluwer, "skill and labour is dead"](https://legalblogs.wolterskluwer.com/copyright-blog/football-dataco-skill-and-labour-is-dead/);
[IPKat](https://ipkitten.blogspot.com/2012/03/database-dismay-for-dataco.html)).
A comprehensive database that tries to include *every* professional footballer has
made few selective choices, almost by definition. Confidence: **arguable that
database copyright is weak here.**

**(c) Sui generis database right.** This is the real one. Under the Copyright and
Rights in Databases Regulations 1997, it arises where there has been substantial
investment in **obtaining, verifying or presenting** the contents, lasts 15 years,
and vests in the "maker" who took the initiative and assumed the risk
([Pinsent Masons overview](https://www.pinsentmasons.com/out-law/guides/database-rights-the-basics)).

SI plainly invests substantially. It runs a network of **more than 1,300 researchers**
worldwide — around 100 head researchers and roughly 1,000 assistants — and by 2018
the database held **799,643 past and present players**
([FourFourTwo](https://www.fourfourtwo.com/features/whats-it-scout-football-manager-a-sports-interactive-expert-reveals-all);
[PCGamesN](https://www.pcgamesn.com/football-manager-2015/football-managers-sports-interactive-run-the-biggest-scouting-network-in-football)).

**But there is a genuine and underappreciated wrinkle.** *British Horseracing Board v
William Hill* (C-203/02, 9 November 2004) holds that database right protects
investment in **seeking out and collecting existing** materials, and **does not
protect investment in the creation of data**
([CJEU judgment PDF](https://curia.europa.eu/juris/showPdf.jsf?docid=64559&pageIndex=0&doclang=en&mode=req&occ=first&part=1&cid=9204340);
[5RB case note](https://www.5rb.com/case/british-horseracing-board-v-william-hill/)).

Apply that to FM:

- Player **names, dates of birth, club affiliations** are pre-existing facts that SI
  *obtains* and *verifies*. Squarely within database right.
- **Current Ability, Potential Ability and the 1–20 attribute ratings are not
  pre-existing facts. SI's researchers create them.** They are opinions generated by
  SI, which did not exist before SI generated them. On a straight *BHB* reading,
  investment in generating those numbers is investment in **creating** data, which
  does not count towards database right.

That is a real argument that the very data Anorak most wants to show — CA/PA — is the
data least well protected by database right. I want to be honest that this is
**arguable, not settled**: I found no case applying *BHB* to game-generated
attribute ratings, the distinction between "creating" and "verifying" is notoriously
slippery, and SI would argue the ratings are assessments *of* observed real-world
performance, i.e. verification of obtained data.

*Correction to a premise in the brief:* the question assumed SI "licenses real player
data (names, attributes) from third parties". I could not verify that. The evidence
points the other way — SI's attribute data comes from **its own in-house researcher
network**, not a third-party data licence. SI's published FM26 licensing announcements
cover **leagues, clubs and competitions** (FIFA, UEFA, Premier League, WSL and
others) and say nothing about buying player data
([footballmanager.com/news/football-manager-26-licences](https://www.footballmanager.com/news/football-manager-26-licences)).
So the likely rightsholder in the player database is SI itself, which simplifies
matters — there is no third-party data licensor with an independent claim.
Confidence: **arguable**; I found no statement from SI on the record about
third-party player-data licensing either way.

### 4.3 Does reading your own save and displaying it infringe? Probably not

The chain of reasoning:

1. **The user lawfully possesses the save file.** It is on their disk, generated by
   software they bought.
2. **Anorak copies its contents into RAM.** That is technically "copying" under CDPA
   s17, including transient copies. So a restricted act does occur, and it needs a
   justification.
3. **s28A (temporary copies) does not help.** It expressly excludes computer programs
   *and databases*
   ([legislation.gov.uk](https://www.legislation.gov.uk/ukpga/1988/48/section/28A)).
   Worth stating because it is a natural place to look and it is a dead end.
4. **CDPA s50D does help, and it is the best hook.**
   ([legislation.gov.uk](https://www.legislation.gov.uk/ukpga/1988/48/section/50D)):

   > "It is not an infringement of copyright in a database for a person who has a
   > right to use the database … to do, in the exercise of that right, anything which
   > is necessary for the purposes of access to and use of the contents of the
   > database".

   And **s296B voids contrary terms**
   ([legislation.gov.uk](https://www.legislation.gov.uk/ukpga/1988/48/section/296B)):

   > "Where under an agreement a person has a right to use a database or part of a
   > database, any term or condition in the agreement shall be void in so far as it
   > purports to prohibit or restrict the performance of any act which would but for
   > section 50D infringe the copyright in the database."

5. **For database right, reg 19 of the 1997 Regulations does similar work**
   ([legislation.gov.uk](https://www.legislation.gov.uk/uksi/1997/3032/regulation/19/made)):

   > "(1) A lawful user of a database which has been made available to the public in
   > any manner shall be entitled to extract or re-utilise **insubstantial parts** of
   > the contents of the database for any purpose.
   > (2) … any term or condition in the agreement shall be void in so far as it
   > purports to prevent that person from extracting or re-utilising insubstantial
   > parts…"

**So the user is well covered, and contract cannot take it away.** The important
structural point: **the acts are the user's, not the developer's.** Anorak ships no
data. Every extraction happens on the user's machine, from the user's own lawfully
acquired copy, under the user's own licence. He is not extracting anything; he is
supplying a tool.

The limit is the word **"insubstantial"** in reg 19. A user who exports the entire
799,000-player database would be extracting a substantial part, and reg 19 would not
cover them. Which brings us to CSV.

### 4.4 Does CSV export change the analysis? Yes, at the margin

Exporting a shortlist of 30 players: plainly insubstantial. Covered by reg 19 and
uncontroversial.

Exporting the whole database: **substantial extraction**, outside reg 19, and
potentially an infringement of database right *by the user*.

Note the exception that might otherwise save it does not. Reg 20 permits fair dealing
with a substantial part only where extracted "for the purpose of illustration for
teaching or research **and not for any commercial purpose**", and the source is
indicated
([legislation.gov.uk](https://www.legislation.gov.uk/uksi/1997/3032/regulation/20/made)).
A user building a scouting spreadsheet is not doing teaching or research.

Two consequences for design, which are cheap to implement and materially reduce risk:

- **Scope exports to shortlists and filtered views, not "export everything".** A
  "dump the full database to CSV" button converts a defensible tool into a database
  extraction utility. This is the single most avoidable self-inflicted risk in the
  product.
- **The exposure sits with the user in any event**, but a developer who ships a
  one-click bulk-extraction feature invites an argument about authorising
  infringement, which is a real doctrine in UK copyright law.

Confidence: **arguable**, with the direction of travel clear.

### 4.5 Accessory liability — the question publishing actually raises

Personal use raises only *his* conduct. Publishing raises a second question: is he
liable for what his **users** do? Two doctrines, and both are reassuring.

**Authorising infringement (CDPA s16(2)).** *CBS Songs Ltd v Amstrad Consumer
Electronics Plc* [1988] UKHL 15 is the leading authority and it is squarely helpful.
Amstrad sold twin-deck cassette recorders, advertised them, and knew perfectly well
what buyers did with them. The House of Lords held this was **not** authorisation:
"authorise" means to grant or purport to grant the right to do the infringing act, and
neither a manufacturer nor a machine can give a purchaser authority to copy. Supplying
the means to infringe, **even with knowledge that infringement will occur**, is not
authorising it
([judgment PDF](https://www.ip4all.co.uk/wp-content/uploads/cbslimitedvamstradhol.pdf);
[case note](https://www.casemine.com/judgement/uk/5a8ff87560d03e7f57ec0dc1)).

Anorak is a considerably better case than Amstrad's: it has an obvious primary use
that infringes nothing at all (viewing your own save), it ships no infringing content,
and it is not marketed as a way to appropriate SI's database. Confidence: **arguable,
and strong** — Amstrad is directly on point.

**Inducing breach of contract.** The more realistic theory, since users are bound by
their own EULAs. But *OBG Ltd v Allan* [2007] UKHL 21 sets a demanding test: there
must be a contract, an actual breach, conduct that procured or induced it, knowledge
of the term breached (or blind-eye knowledge), and — the hard limb — the defendant
must have **intended** to procure the breach. Knowledge that a breach will follow is
not enough; it must be an end or a means, not merely a foreseeable side effect
([House of Lords judgment](https://publications.parliament.uk/pa/ld200607/ldjudgmt/jd070502/obg-1.htm);
[Lexology on the knowledge/intention requirement](https://www.lexology.com/library/detail.aspx?g=f0e1e1ff-bc35-4251-9935-31ef03e808d7)).

The practical lesson is about **marketing copy, not code**. A tool described as "a
save file viewer" does not evidence an intention to procure EULA breaches. A tool
marketed as "see the ratings SI don't want you to see" or "beat the EULA" starts to
supply the intention limb for free. This is the cheapest risk control in the entire
document: *write the marketing carefully.*

---

## 5. Question 4 — What the donation link changes

Short answer: **less than instinct suggests, but it is not nothing, and it matters in
four different places for four different reasons.** The critical distinction the brief
asked me to be precise about is between **donations for the tool** and **selling the
data**, and those really are different.

### 5.1 The distinction that does the most work

| | What it is | Assessment |
| --- | --- | --- |
| **Donations for the tool** | Voluntary payments for software *he wrote*, which contains none of SI's content | Low risk |
| **Selling the data** | Charging for access to SI's player database, or shipping saves/extracts | High risk — this is where database right and 7(a) genuinely bite |

Anorak is squarely the first. It ships no game data (§1). A user without their own FM
save gets nothing. What a donor is paying for is **his parser and his UI**, which are
his own copyright. **Nobody is buying SI's data, because none is being supplied.**

Confidence: **arguable and strong**. This framing survives contact with every clause
below.

### 5.2 The EULA's non-commercial clauses — probably not engaged

This is the point most people get wrong, and getting it right matters.

**EULA §2** licenses "one (1) copy of the Product solely and exclusively for your
personal and non-commercial use". **§7(a)** prohibits "exploit[ing] **the Product** or
any of its parts commercially".

Read them carefully: **both attach to use of *the Product*.** The Product is defined as
"Game Software, Editors, Additional Content, Physical Materials and Key Code" — i.e.
Football Manager itself
([SEGA EULA](https://privacy.sega.com/en/sega-europe-end-user-license-agreement)).

He is not commercially exploiting Football Manager. He is not renting it out, running
it in an internet café, reselling it, or charging anyone for access to it. He plays his
own copy personally, and separately writes his own software. **Donations for Anorak are
not commercial exploitation of the Product** any more than a paid YouTube channel about
FM is.

The counter-argument SEGA would run: 7(a) says "the Product **or any of its parts**",
and a tool that surfaces the Product's data arguably monetises "a part" of it
indirectly. That is a stretch, but it is not frivolous — and it gets stronger the more
the tool's value proposition is *the data* rather than *the interface*.

**But note what this does not touch: clause 7(h) has no commercial element at all.** It
prohibits creating mimicking programs full stop, free or paid. So **donations do not
worsen the biggest legal risk** (§10) — 7(h) bites identically either way. That is a
genuinely important and slightly counter-intuitive conclusion.

Confidence: **arguable**. I found no case on a game EULA's non-commercial clause
applied to donation-funded companion software.

### 5.3 Database right exceptions — here it matters

This is where "commercial" has real statutory bite.

**Reg 20** of the Copyright and Rights in Databases Regulations 1997 permits fair
dealing with a **substantial part** only where extracted "for the purpose of
illustration for teaching or research **and not for any commercial purpose**", with
the source indicated
([legislation.gov.uk](https://www.legislation.gov.uk/uksi/1997/3032/regulation/20/made)).

Two observations:

1. **Reg 20 was never available anyway.** A user building a shortlist is not doing
   "illustration for teaching or research", commercial or not. So donations do not
   lose him an exception he had.
2. **Reg 19 — the one that actually protects users — has no commercial limitation.**
   It entitles a lawful user to extract insubstantial parts "**for any purpose**", and
   voids contrary contract terms
   ([legislation.gov.uk](https://www.legislation.gov.uk/uksi/1997/3032/regulation/19/made)).
   "Any purpose" includes commercial ones.

So the protection he relies on survives the donation link, and the exception he loses
was never his. Confidence: **settled** on the statutory wording.

### 5.4 Fair dealing — mostly irrelevant, but know which is which

For completeness, since the brief asked. UK fair dealing exceptions split on this:

| Exception | Non-commercial required? |
| --- | --- |
| s29 research and private study | **Yes** — research must be non-commercial |
| s29A text and data mining | **Yes** — non-commercial research only |
| s30 criticism, review, news reporting | No |
| s30(1ZA) quotation | No |
| s30A parody, caricature, pastiche | No |

The honest position: **none of these is doing meaningful work for Anorak.** His defence
rests on s50D/s296B and reg 19 (§4.3), not on fair dealing. But if he ever leaned on
s29 for the *research phase* of format discovery, the donation link would undercut it.
Another reason the format work being already done and documented is helpful.

### 5.5 Trade mark: "in the course of trade" — donations probably cross it

Trade mark infringement under TMA 1994 s10 requires use "in the course of trade". This
does **not** require profit, and the threshold is low — non-profit and voluntary-funded
activity can still be in the course of trade where it is more than purely private.

Practical effect: **a donation link probably does put him in the course of trade.** So
the "it's just a hobby" argument should not be relied on. The real protection is the
s11(2)(c) referential-use defence in §6, which does not care whether he trades.

Confidence: **arguable**, and I would advise assuming the worse position.

### 5.6 Tax — small, real, easily handled

HMRC's **trading allowance** exempts up to **£1,000 a year** of trading or casual
income. Below that, no need to tell HMRC (subject to specific exceptions). Above it,
he must **register for Self Assessment**
([gov.uk](https://www.gov.uk/guidance/tax-free-allowances-on-property-and-trading-income)).

Donations for software are very likely trading income rather than gifts, because they
are connected to something supplied. Practical advice: **track the total**, and if it
approaches £1,000, register. This is administrative, not risky.

### 5.7 The precedent point that should reassure him most

**FM Genie Scout has run precisely this model — free build with ads, ad-free build for
a personal donation, handled by the developer directly — for over fifteen years, on the
most prominent FM tools site, while openly advertising that it reveals attributes
invisible in FM.** No objection from SI (§7.2).

And FMRTE has gone considerably further: an actual **paid product** at €7.99 or
€2.50/month, sold since 2008, whose own EULA concedes its use "might be against Sports
Interactive Ltd. EULA" (§7.4). Still selling.

If a paid commercial editor that writes to saves has survived eighteen years, a
donation link on a read-only viewer is not what changes SI's mind. **The donation link
is close to the least risky thing in this entire document.**

---

## 6. Question 5 — Trade mark

This is the most tractable area in the document. The law gives a clear, usable answer,
and the line between safe and unsafe is bright.

### 6.1 The statutory defence is directly on point

**Trade Marks Act 1994, s11(2)**, as amended by the Trade Marks Regulations 2018/825
with effect from 14 January 2019
([legislation.gov.uk](https://www.legislation.gov.uk/ukpga/1994/26/section/11)):

> "A registered trade mark is not infringed by—
> (a) the use by an individual of his own name or address,
> (b) the use of signs or indications which are not distinctive or which concern the
> kind, quality, quantity, intended purpose, value, geographical origin, the time of
> production of goods or of rendering of services, or other characteristics of goods
> or services, or
> **(c) the use of the trade mark for the purpose of identifying or referring to goods
> or services as those of the proprietor of that trade mark, in particular where that
> use is necessary to indicate the intended purpose of a product or service (in
> particular, as accessories or spare parts),**
> provided the use is **in accordance with honest practices in industrial or commercial
> matters**."

**s11(2)(c) was added specifically to codify referential use.** Saying "Anorak is a save
file viewer for Football Manager" is using the mark to refer to SI's goods as SI's
goods, and it is necessary to indicate Anorak's intended purpose. That is what
s11(2)(c) exists for. Confidence: **settled** as to the provision applying; the
question is always the honest-practices proviso.

### 6.2 The honest practices test — the four ways to lose it

*Gillette Company v LA-Laboratories* (C-228/03, 17 March 2005) is the leading
authority. LA-Laboratories sold replacement blades marked "all Parason Flexor and
Gillette Sensor handles are compatible with this blade", without any licence, and won
([IPPT full text PDF](https://www.ippt.eu/sites/ippt/files/2005/IPPT20050317_ECJ_Gillette_v_LA_Laboratories.pdf);
[CMS analysis](https://cms-lawnow.com/en/ealerts/2005/04/use-of-third-party-trade-marks-permissible-provided-use-is-honest)).

Use is **not** in accordance with honest practices where:

1. it **gives the impression of a commercial connection** with the proprietor;
2. it **takes unfair advantage** of the mark's distinctive character or repute;
3. it **discredits or denigrates** the mark; or
4. the third party **presents its product as an imitation or replica**.

*BMW v Deenik* (C-63/97) established the same principle earlier for services —
an independent garage may advertise that it services BMWs.

Map these onto Anorak:

| Gillette limb | Anorak's position | Risk |
| --- | --- | --- |
| Commercial connection | Cured by a prominent disclaimer + distinctive own name | Low |
| Unfair advantage | Using "Football Manager" in a *domain* or *product name* would be | **Controllable** |
| Discredit | Avoid "SI hides this from you" framing | Low |
| Imitation/replica | Do **not** market as an alternative to SI's official Editor | **Watch this** |

The fourth limb is the sharp one and it connects to §8.2: **positioning Anorak as a
free substitute for SI's £7.49 in-game editor is exactly the framing that forfeits the
honest-practices defence.** FM Live Editor markets itself as a "High-octane alternative
to the official In-Game Editor" (§7.6) — that is the thing not to copy.

### 6.3 SEGA's own position on referential use is a gift

There is a real irony here worth having in the back pocket.

In *Manchester United FC v SEGA & Sports Interactive* (Business and Property Courts,
issued August 2018, discontinued July 2021), Manchester United sued over FM's use of
its name and over SI's use of a generic crest rather than the official one. **SEGA's
defence was that the club name was "a legitimate reference to the Manchester United
football team in a football context"**, relying on nearly 30 years of uncontested use,
and arguing that restricting it would be an "unreasonable restraint on the right to
freedom of expression"
([William Fry](https://www.williamfry.com/knowledge/manchester-united-settle-trade-mark-dispute-with-sega/);
[RPC](https://www.rpclegal.com/thinking/ip/segas-battle-against-man-utd-in-football-manager-trade-mark-case-ends-in-settlement/)).

It settled on a **no-admissions basis**, with the club renamed "Manchester UFC" as a
goodwill gesture, SEGA maintaining throughout that it **did not need a licence**
([Sky Sports](https://www.skysports.com/football/news/11667/12374507/manchester-united-to-be-renamed-on-football-manager-following-trademark-settlement);
[VGC](https://www.videogameschronicle.com/news/football-manager-will-no-longer-use-the-manchester-united-name-following-a-trademark-dispute/)).

So SI/SEGA have publicly and formally advanced **precisely the referential-use argument
Anorak would rely on**. That does not bind them, but it makes an aggressive trade mark
claim against a descriptive strapline awkward for them, and it is worth knowing.

Two further lessons from that case:

- **It ran for three years.** Even a well-resourced defendant found a trade mark claim
  expensive and eventually settled for a rename. He should not want to test his
  defence, however good it is.
- **Man Utd's complaint included that SI's generic crest *encouraged third-party logo
  packs*.** Third-party FM add-ons featured in the litigation as a grievance — against
  SI, not against the tool makers.

### 6.4 What is safe and what crosses the line

**Safe:**

- Product name **"Anorak"** — distinctive, unrelated, no implication of origin.
- Strapline: *"A save file viewer for Football Manager"* — textbook s11(2)(c).
- *"Football Manager is a trade mark of Sports Interactive Limited. This tool is not
  affiliated with or endorsed by Sports Interactive or SEGA."*
- Domain `anorak.app` or similar with no mark in it.
- Plain-text references to "FM26" in changelogs to indicate compatibility.

**Crosses the line:**

- **"Football Manager" or "FM" in the product name** — e.g. "FM Anorak", "Football
  Manager Scout". This is use as a badge of origin, not reference.
- **Any domain containing the mark** — `fmanorak.com`, `footballmanagertools.com`.
  Domains signal origin strongly, and they also expose him to a cheap dispute-
  resolution complaint that costs SEGA very little to bring.
- **SI or SEGA logos, the FM wordmark, or FM's distinctive styling.** Logos are figurative
  marks; there is no descriptive necessity to reproduce them, so limb 2 of *Gillette*
  fails immediately.
- **Game screenshots** in marketing.
- Anything implying endorsement, partnership, or official status.

### 6.5 Passing off, and whether disclaimers work

Separately from registered marks, **passing off** requires the classic trinity from
*Reckitt & Colman v Borden* ("Jif Lemon"): goodwill, misrepresentation, and damage.
Follow §6.4 and there is no misrepresentation as to origin, so the claim does not get
off the ground.

**Do disclaimers work?** Partially, and he should be realistic. A disclaimer is
**good evidence on the honest-practices proviso and on the misrepresentation limb of
passing off** — it shows he took care not to imply a connection. It is **not a cure**
for a name or domain that itself misrepresents origin: courts have repeatedly held that
a disclaimer buried at the bottom of a page does not undo a confusing brand. The
sequence matters — **choose a non-infringing name first, then disclaim.** A disclaimer
is a seatbelt, not a licence.

### 6.6 What I could not verify

**I did not confirm the UK IPO register entries for "FOOTBALL MANAGER", "SPORTS
INTERACTIVE" or "FM".** The register search was not completed, so I cannot give
registration numbers, owners or classes. This is stated as a gap rather than assumed.

What can be said: SEGA/SI clearly assert rights in "Football Manager" (SI is named as
the proprietor in third-party disclaimers such as MacScout26's), and Manchester United
held EU registrations covering computer software and games in the litigation above. It
would be sensible to run the register search at
[trademarks.ipo.gov.uk](https://trademarks.ipo.gov.uk/) before launch, and to check
that **"Anorak"** itself is clear in class 9 — a check that has also not been done.
Confidence in the underlying analysis: **high**; in the register specifics: **unverified**.

---

## 7. Question 6 — Precedent and SI's observed posture

This turned out to be the most informative section, and it changes the practical
picture more than any point of law above.

**The one-line summary: prohibited on paper, tolerated in practice, never enforced.**

### 7.1 The enforcement record is a definitive zero

This was tested properly rather than inferred from search results. GitHub publishes
every DMCA notice it receives in a public repository
([github.com/github/dmca](https://github.com/github/dmca)). A full-corpus search of
**21,613 notices, current to 31 July 2026**, returns:

| Search term | Matches |
| --- | --- |
| "Sports Interactive" | **0** |
| "Football Manager" | **0** |
| FMRTE | **0** |
| Genie Scout | **0** |
| fmscout / sigames / sports-interactive | **0** |

SEGA appears in a handful of unrelated notices (*Xuccess Heaven*, 2019; a *Sonic
Unleashed* mobile clone, 2020). Nothing FM-related, ever.

**This is a searched-and-found-nothing result, stated as such.** Its limits matter:
the repository covers only DMCA notices GitHub received and published. It cannot rule
out **private cease-and-desist letters**, notices sent to other hosts, or trademark
and contract claims made outside the DMCA process. Absence of published takedowns is
strong evidence, not proof.

Separately, no FM-tool litigation surfaced. The only significant SI/SEGA IP litigation
found is *Manchester United v SEGA & Sports Interactive* (2018–2021), a trade mark
dispute about club crest and name usage which settled — nothing to do with tools
([LawInSport](https://www.lawinsport.com/topics/item/sega-s-battle-against-man-utd-in-football-manager-trade-mark-case-ends-in-settlement)).

### 7.2 FM Genie Scout — 20 years, still shipping, monetised

- Current for FM26: build 1531, updated 10 June 2026, compatible with FM 26.3.2,
  **153,819 downloads**. Exclusive to fmscout.com since 2009, and running since FM
  2007 ([fmscout.com](https://www.fmscout.com/a-fm-genie-scout-26.html)).
- Author is **Eugene Tarabanovsky** (*correction to the brief's premise: not
  "Tarasenko"; no evidence for "Nizzy"*).
- It **openly advertises "observing attributes invisible in FM"** — i.e. exactly the
  CA/PA reveal Anorak is aiming at.
- **Monetisation is donationware, not a licensed sale** — a free build with two banner
  ads, and an ad-free "g" edition obtained by donating, with "donations handled by
  Eugene" personally. *This is a near-exact match for the "Buy Me a Coffee" model, run
  publicly for 15+ years without objection.*
- The GS25 gap in the release series is explained by **FM25 being cancelled**, not by
  legal pressure — GS12 through GS24 and GS26 release pages all resolve; only GS25 404s.

### 7.3 The ecosystem polices bulk export — and cites the EULA to do it

This is the single most useful document found, because it independently confirms the
analysis at §4.4. Genie Scout's own download page carries this notice:

> "**EXPORTING DATA:** Please understand it is forbidden by the EULA from SI and SEGA
> to export all the data from the results. You should be able to export only basic
> information such as player name, club, age, etc."
> ([fmscout.com](https://www.fmscout.com/a-fm-genie-scout-26.html))

A 20-year-old tool that freely reveals hidden CA/PA nonetheless **draws its own line at
bulk data extraction**, unprompted, citing the SI/SEGA EULA. The community's own
understanding of where the red line sits matches where the statutory analysis puts it:
displaying data to its owner is fine; wholesale extraction of the database is not.

Confidence: **high** as evidence of ecosystem norms; it is a vendor's self-assessment,
not a legal ruling.

### 7.4 FMRTE — commercial, sold since 2008, and it concedes the risk in writing

- **Actively sold today** as FMRTE 26: €7.99 lifetime single licence, or €2.50/month
  / ~€11.04/year subscription ([fmrte.com/fmrte](https://www.fmrte.com/fmrte/)).
  Licensor is **BraCa Soft** (*correction: I could not verify "Ruben Gonzalez"*).
- It attaches to the running game process and **writes** to saves in real time —
  vastly more invasive than Anorak.
- Its own EULA concedes the point in terms:

  > "FMRTE is an unofficial editor for Football Manager and **it's use might be against
  > Sports Interactive Ltd. EULA**."
  > ([fmrte.com/eula](https://www.fmrte.com/eula/))

So a commercial vendor has sold a product it publicly describes as *possibly EULA-
violating*, for money, since 2008, and has never been stopped. No cease-and-desist
found.

### 7.5 SI's observed conduct: affirmative toleration, not mere inaction

- **SI hosts a forum section for this.** "Editors Hideaway" sits under SI's own
  community site
  ([community.sports-interactive.com/forums/forum/26-editors-hideaway](https://community.sports-interactive.com/forums/forum/26-editors-hideaway/)).
- **A promotional thread for Genie Scout has stood on SI's own forum since October
  2011 and is still live**, marketing the tool's ability to observe attributes
  invisible in FM and its donation-gated edition
  ([thread](https://community.sports-interactive.com/forums/topic/226045-the-official-fm-genie-scout-12-thread/)).
  Advertising for a CA/PA-revealing tool, unremoved on the rightsholder's own platform
  for roughly fifteen years.
- **The clearest official-ish statement found**, from SI forum moderator "XaW"
  (11 March 2025), on whether FMRTE trips the in-game editor flag:

  > "That only shows for the official in-game editor. **FMRTE is not something SI
  > supports in any way shape or form**, and thus no one in here can help with it."
  > ([thread](https://community.sports-interactive.com/forums/topic/591166-fmrte-in-in-game-editor-used/))

  Note the wording carefully: **"does not support"**, not "is prohibited" and not "we
  will act against". Confidence: **medium** — SI forum moderators are typically
  volunteers, so this is not a corporate legal position.
- **Materially telling side-fact:** FM flags saves that used the *official* in-game
  editor, but does **not** detect or flag FMRTE. SI built detection for its own
  product and not for third-party ones.

**Explicit gap: I found no Miles Jacobson statement on third-party tools at all.**
Searches across interviews, X/Twitter, LinkedIn and the SI forums for his comments on
editors, scouting tools, hidden attributes or CA/PA revelation surfaced nothing. That
is reported as *searched and not found*, not as *he never said anything* — his X
account and 20+ years of forum posts are not fully search-indexed.

### 7.6 SI sells the same capability — the strongest fairness point

The brief asked whether SI's own paid editor is relevant. It is, and it cuts in his
favour.

**Football Manager 26 In-Game Editor**, developer Sports Interactive, publisher SEGA,
released 4 November 2025, **£7.49 / $8.99**. Its own Steam description advertises
adjusting "every player's entire profile, from individual **Attributes to their
Current and Potential Ability**"
([Steam](https://store.steampowered.com/app/3551410/Football_Manager_26_InGame_Editor/)).
Reception is "Overwhelmingly Negative" — 8% positive from 1,269 reviews.

SI's own bug tracker treats hidden-attribute visibility in the official editor as an
intended, supported feature
([bug tracker](https://community.sports-interactive.com/bugtracker/1644_football-manager-26-bugs-tracker/1890_pre-game-in-game-editors/in-game-editor/editing-players-and-viewing-hidden-attributes-in-search-r38028/)).

So the "hidden data must stay hidden" position is not one SI actually holds — it sells
that data's visibility for £7.49. The EULA's 7(h) carve-out for "Editors" is
essentially a commercial moat, not a design-integrity principle.

**And a direct paid competitor operates completely openly.** "FM Live Editor 26",
built by FM Scout's own owner, launched 23 January 2026, **56,699 downloads**, licensed
at **£5.49 / €5.99 / $6.99**, edits attributes, CA/PA, finances and contracts — and
markets itself in as many words as a *"High-octane alternative to the official In-Game
Editor"*, updated "within hours of new FM patches"
([fmscout.com](https://www.fmscout.com/a-fm-live-editor-26.html)). A third-party
product undercutting SI's own DLC on price, by name, has drawn no visible response.

### 7.7 Live open-source save parsers on GitHub

All public and active, none disabled or DMCA'd (checked via the GitHub API for
archived/disabled status):

| Repo | What it is | Last push |
| --- | --- | --- |
| `dylanvir/fm-save-parser` | "Open-source parser for FM 2024 save files. **Documents the Zstandard binary format.**" | 2026-06-16 |
| `mavarobli/FMSuperScout` | FM26, loads full save (45k+ players), role ratings | 2026-07-27 |
| `Einzigart/MacScout26` | Native macOS scouting for FM26 saves | 2026-08-01 |
| `robeady/fm-explorer` | Reads process memory via FMScoutFramework | (live, no notice) |

`dylanvir/fm-save-parser` is the notable one: a repository whose **stated purpose is
publicly documenting FM's save binary format**, untouched. `Einzigart/MacScout26` is
notable for a different reason — it is native macOS FM26 save scouting, i.e. **direct
competition for Anorak's exact niche**, and it is alive.

There is also a public Thunderstore mod database for FM26 including an official BepInEx
pack, and a free "FM26 Player Export" plugin that dumps player lists to CSV/HTML
([Thunderstore](https://thunderstore.io/c/football-manager-26/)) — with FM Scout
publishing a guide on making FM26 Unity mods
([guide](https://www.fmscout.com/c-help.html?id=13128)).

### 7.8 FM26 context

FM25 was cancelled in February 2025 for not meeting "internal quality standards"; FM26
shipped 4 November 2025 as the series' first Unity-engine entry
([Wikipedia](https://en.wikipedia.org/wiki/Football_Manager_26)). The Unity switch
appears to have made tooling **easier**, not harder — IL2CPP Unity brought the standard
BepInEx modding stack to FM for the first time and a modding scene formed immediately.

**No evidence was found of new save encryption, obfuscation or anti-tool measures in
FM26**, and no SI statement about tools for FM26. The behavioural counter-evidence is
strong: Genie Scout and FM Live Editor both track FM26 patch versions (26.0.0 →
26.3.2) and ship updates within hours. Again: searched and not found, not proven absent.

### 7.9 What this precedent is and is not worth

**What it supports:** a well-founded practical expectation that SI does not pursue
third-party tools, including commercial ones, including ones that reveal CA/PA,
including ones that undercut its own DLC. Eighteen years, zero enforcement actions,
affirmative hosting of tool discussion on its own forums.

**What it does not support:** any legal conclusion. Toleration is not a licence.
Non-enforcement creates no estoppel in these circumstances, does not amend the EULA,
and can stop the day a new legal or commercial director takes a different view.
Trade mark rights can be weakened by non-enforcement; **contract and copyright rights
essentially cannot**. He should plan for the observed posture and be ready for the
paper one.

---

## 8. Question 7 — Practical risk-reduction steps

### 8.0 A live template worth copying

Before the list: **`Einzigart/MacScout26`** is a currently-live, public GitHub project
doing almost exactly what Anorak proposes — native macOS FM26 save parsing, with a
**Ko-fi tip link** — and its README carries this notice
([GitHub](https://github.com/Einzigart/MacScout26)):

> "MacScout26 is an independent, unofficial tool. Sports Interactive and SEGA do not
> support or approve it. Football Manager is a trademark of Sports Interactive
> Limited."

Note what it does: **"Football Manager" appears in the description, not in the product
name**; affiliation is expressly denied; the mark is acknowledged as SI's. It is
closed-source under a proprietary licence. This is the pattern to follow, and someone
is already running it publicly in the same niche without incident.

By contrast `dylanvir/fm-save-parser` — "a clean-room parser for Football Manager 2024
save files" — carries **no trademark disclaimer at all** and is also untouched
([GitHub](https://github.com/dylanvir/fm-save-parser)). Note its framing though:
"clean-room" is a good word to be able to use truthfully.

### 8.1 Naming and presentation

- **Keep "Anorak" as the product name.** It is distinctive, unrelated to SI's marks,
  and carries no implication of endorsement. Do not rename to anything containing
  "FM" or "Football Manager". *(Worth a separate check that "Anorak" itself is clear
  of conflicting UK marks in class 9 — that check has not been done.)*
- **Use the mark descriptively, in the strapline, not the title.** "A save file
  viewer for Football Manager" is referential use. "Anorak Football Manager Scout" is
  not.
- **Never use SI or SEGA logos, the FM wordmark, or FM's typography, colours or UI
  styling.** Logos are where descriptive use defences fall apart fastest.
- **Do not register or use a domain containing "footballmanager" or "fm" plus a mark.**
  A domain is a strong signal of origin and is where a cheap, effective UDRP-style
  complaint becomes available.
- **Ship an explicit disclaimer** on the site, in the README, and in the app's About
  screen. Copy the MacScout26 wording; it is well judged.
- **Never use screenshots of the game itself** in marketing. Screenshots of *Anorak's
  own UI* are yours.

### 8.2 Marketing copy — the cheapest and most important control

Per §4.5, the accessory-liability theories against him turn almost entirely on
**intention**, and intention is evidenced by what he writes.

| Write this | Not this |
| --- | --- |
| "Reads your own save file" | "Unlocks the game's hidden data" |
| "A scouting companion" | "See what SI don't want you to see" |
| "Requires your own copy of the game" | "Get the in-game editor's data for free" |
| "Independent, unofficial" | "The better editor" |

The last row matters commercially as well as legally: positioning explicitly against
SI's £7.49 DLC turns a tolerated hobby tool into a competitor, which is the thing most
likely to change SI's calculus. FM Live Editor does exactly this and has been fine —
but it is the riskiest thing in the ecosystem, not the safest.

### 8.3 Do not bundle game data

- **Ship no `.fm` files, no extracted databases, no player name lists, no club lists,
  no sample saves.** The tool must require the user to supply their own save. This is
  what keeps every extraction the *user's* act under reg 19 / s50D, not his.
- **Do not ship SI's graphics, kits, badges, or any game asset.**
- Publishing `SAVE_FORMAT.md` is fine — the format is not protected (§4.1). Publishing
  a dump of what the format *contains* is not.

### 8.4 Do not circumvent anything, and keep it that way

- No process attachment, no `ReadProcessMemory`, no code injection, no hooking, no
  touching Denuvo. He is already on the right side of all of these, and this is the
  main reason the analysis comes out well. **This is a constraint to preserve, not a
  box already ticked forever** — the temptation to attach to the process to resolve
  CA/PA faster is exactly the wrong trade.
- If SI ever **encrypts** the save format, stop. Reading an encrypted save would move
  him from "no TPM exists" to "circumvention", and both the UK (s296ZA) and US
  (§1201) analyses flip hard.
- Keep the parser **clean-room**: derived from observation of his own saves, never
  from decompiled SI code or from anyone else's decompilation. Document that
  provenance — `research/` and `SAVE_FORMAT.md` already do this well, and that
  paper trail is a genuine asset if he is ever challenged.

### 8.5 Constrain export

Per §4.4 and the ecosystem norm at §7.3:

- **Shortlist and filtered exports: yes.** Insubstantial parts, covered by reg 19.
- **"Export entire database": no.** This is the single most avoidable self-inflicted
  risk in the product. Genie Scout, a 20-year-old tool, declines to do it and says so
  on its own download page.
- Consider a soft cap on export size, and no "export all players" affordance.

### 8.6 Source vs binaries — they differ, and source is safer

This distinction is real and worth acting on.

**Publishing source is materially safer than publishing binaries:**

- Source and format documentation is closer to **speech and factual description**. The
  format is unprotected (§4.1), so documenting it is lawful and hard to attack.
- A source repository does not itself do anything to anyone's save file. The user must
  build and run it, which strengthens the "the acts are the user's" framing at §4.3.
- Under both the UK device provisions (s296) and US §1201(a)(2) trafficking rules, the
  target is a *circumvention device*. Neither the source nor the binary is one here,
  but source is even further from it.

**Binaries carry the platform risk, which is the practical exposure:**

- A signed, notarised macOS app depends on an **Apple Developer ID**. A complaint to
  Apple is a cheap, fast lever for SEGA that requires no litigation, and losing a
  Developer ID is a far more serious personal consequence than a contract claim.
- Binaries hosted on his own site are within his control; binaries on GitHub Releases,
  Homebrew, or an app store are subject to those platforms' takedown processes.

**Practical suggestion:** source on GitHub under a permissive licence; binaries
self-hosted; and keep the ability to take the binaries down quickly without losing the
project.

### 8.7 Licence choice

His own code is his own copyright and he may licence it as he likes — the EULA does
not touch that. Considerations:

- **MIT or Apache-2.0** is the conventional choice and signals hobby/community intent.
  **Apache-2.0** adds an express patent grant and a contribution clause; MIT is
  simpler.
- **GPL-3.0** would prevent someone taking his parser and shipping a closed commercial
  editor with it. Given FM Live Editor and FMRTE are paid products, this is a real
  scenario and arguably the better fit if he cares about it.
- **A proprietary/source-available licence** is what MacScout26 chose. Viable, but it
  forfeits the "open, inspectable, community" framing that makes a tool look benign.
- **Add a disclaimer of warranty and an explicit "you must own the game" condition of
  use.** Neither is legally bulletproof but both help characterise the tool.
- **Do not licence it in a way that suggests you own rights in the FM data.** Licence
  the *code*, and say so.

### 8.8 Honour takedown requests — and how

The single highest-value practical commitment. If a letter arrives:

1. **Do not ignore it, and do not argue publicly.** A defiant blog post converts a
   form letter into a matter of principle for SEGA's legal team.
2. **Comply promptly with the specific request**, even if the underlying legal claim
   looks weak. Taking a download offline costs him nothing permanent.
3. **Then, if he wants, seek advice** about narrowing and republishing.
4. **Do not accept a broad undertaking** (e.g. "never publish anything relating to
   Football Manager") without advice — comply with the narrow ask, not the broad one.
5. **Keep a published contact address** so a complaint reaches him rather than his
   host or Apple. Being easy to contact is genuinely protective: it makes the cheap
   path for SEGA a letter to him rather than a platform takedown.

### 8.9 Housekeeping

- **Keep the SI/SEGA account separate from the project.** Publishing under an account
  tied to his Steam identity makes an account ban trivially easy.
- **Accept that a Steam/SEGA account ban is the most likely adverse outcome** and
  decide in advance whether that is tolerable. It probably costs him his FM licence
  and possibly his Steam library.
- **Version-pin the claims.** Say "tested against FM 26.x" rather than implying
  ongoing official compatibility.

---

## 9. US note

A public website reaches everywhere, so this is worth a section. The headline is
counter-intuitive: **US copyright and DMCA exposure is lower than in the UK, but US
contract exposure is higher.** The statutory protection he enjoys at home does not
travel.

### 9.1 DMCA §1201 — almost certainly no circumvention

**17 U.S.C. §1201(a)(1)(A)**: "No person shall circumvent a technological measure
that effectively controls access to a work protected under this title."

**§1201(a)(3)(B)** defines the key term: a measure "effectively controls access" if,
in the ordinary course of its operation, it "requires the application of information,
or a process or a treatment, **with the authority of the copyright owner**, to gain
access to the work" ([copyright.gov](https://www.copyright.gov/title17/92chap12.html);
[Cornell LII](https://www.law.cornell.edu/uscode/text/17/1201)).

The decisive words are **"with the authority of the copyright owner"**. Zstandard is
a public, openly specified, freely implementable format. Decompressing it needs no
key, no credential and no permission from SI.

Two cases make this concrete:

***Lexmark v. Static Control Components*, 387 F.3d 522 (6th Cir. 2004)** — the closest
analogy. The Sixth Circuit held Lexmark's authentication sequence did not effectively
control access, because "**No security device … protects access to the Printer Engine
Program Code and no security device accordingly must be circumvented to obtain access
to that program code**", and "It is the purchase of a Lexmark printer that allows
'access' to the program." The court's lock analogy: "one would not say that a lock on
the back door of a house 'controls access' to a house whose front door does not
contain a lock"
([Wikisource](https://en.wikisource.org/wiki/Lexmark_Int%27l_v._Static_Control_Components/Opinion_of_the_Court)).
Map it across: it is the purchase and installation of FM that allows access to the
save file.

***MDY Industries v. Blizzard*, 629 F.3d 928 (9th Cir. 2010)** — even more precisely
on point. The Ninth Circuit split World of Warcraft into components and held Warden
did **not** effectively control access to the literal code and individual non-literal
elements, **because players could access those from their own hard drives**. Only the
"dynamic non-literal elements" — the live server-side gameplay experience — were
access-controlled, and that was where MDY lost
([FindLaw](https://caselaw.findlaw.com/court/us-9th-circuit/1548042.html)). A local
save file on the user's own disk is squarely in the category the court held was *not*
access-controlled.

Honest caveats:

- There is a **circuit split**. *Chamberlain v. Skylink*, 381 F.3d 1178 (Fed. Cir.
  2004) requires a "critical nexus between access and protection"
  ([FindLaw](https://caselaw.findlaw.com/court/us-federal-circuit/1104584.html)); MDY
  expressly declined to follow it. This does not matter here, because the argument is
  not "we circumvented but did not infringe" — it is "there is nothing to
  circumvent", which wins in every circuit.
- Weak protection is not automatically no protection: in *RealNetworks v. Streambox*
  (W.D. Wash. 2000) a weak "Secret Handshake" still qualified, because it required
  authorisation from the rightsholder. Compression does not.
- **If SI ever encrypts the save format or adds a key, this analysis flips.**

Confidence: **high** that reading a zstd-compressed, unencrypted local save is not
§1201 circumvention; not settled, because no US case has ruled on compression-as-TPM.

**§1201(f)** (reverse engineering for interoperability) is a fallback that should not
be needed, and note two limits: it only excuses *circumvention*, and — critically —
it is **waivable by contract** in the US (see 9.3).

### 9.2 17 U.S.C. §117 — unavailable, but not needed

§117 permits "the owner of a copy of a computer program" to make a copy as "an
essential step in the utilization of the computer program"
([Cornell LII](https://www.law.cornell.edu/uscode/text/17/117)).

*Vernor v. Autodesk*, 621 F.3d 1102 (9th Cir. 2010) held a user is a licensee rather
than an owner where the copyright owner "(1) specifies that the user is granted a
license; (2) significantly restricts the user's ability to transfer the software; and
(3) imposes notable use restrictions", and that licensees "are not entitled to claim
the essential step defense"
([official PDF](https://cdn.ca9.uscourts.gov/datastore/opinions/2010/09/10/09-35969.pdf)).
*MDY* applied this to a game: WoW players are licensees. The SEGA EULA satisfies all
three Vernor factors comfortably.

But §117 is beside the point. It addresses copying *the program*. Anorak does not
install, run, adapt or copy FM's executable. And a save file is data generated on the
user's machine by their own play — there is a strong argument the user owns that copy
even while merely licensing the binary.

### 9.3 The real US risk: EULA clauses are enforceable there

**This is the sharpest UK/US divergence and the headline of this section.**

| | **UK** | **US** |
| --- | --- | --- |
| Anti-reverse-engineering clause | **Void by statute** (CDPA s296A) | **Enforceable contract** |
| Can you waive decompilation rights? | No | Yes |
| Can you waive fair use / fair dealing? | No | Yes, if freely agreed |

*Bowers v. Baystate Technologies*, 320 F.3d 1317 (Fed. Cir. 2003) enforced a
shrink-wrap prohibition on reverse engineering and held the contract claim not
preempted by the Copyright Act
([overview](https://en.wikipedia.org/wiki/Bowers_v._Baystate_Technologies,_Inc.)).

*Davidson & Associates v. Jung*, 422 F.3d 630 (8th Cir. 2005) — the bnetd case — held
that defendants "expressly relinquished their rights to reverse engineer" by agreeing
to Blizzard's EULA, **making the §1201(f) statutory exemption unavailable to them**,
and that "private parties are free to contractually forego the limited ability to
reverse engineer a software product"
([FindLaw](https://caselaw.findlaw.com/court/us-8th-circuit/1029777.html)).

So: everything §3 says about s296A voiding SEGA's clause is **UK-only**. A US court
would likely enforce clause 7(e) and 7(h) as written.

### 9.4 The MDY safety net: covenant vs condition

This materially caps the downside even if a US court found a breach. *MDY* held that
"For a licensee's violation of a contract to constitute copyright infringement, there
must be a nexus between the condition and the licensor's exclusive rights of
copyright", and found Blizzard's anti-bot terms were **covenants, not conditions**,
because "Glider does not infringe any of Blizzard's exclusive rights … the use does
not alter or copy WoW software"
([FindLaw](https://caselaw.findlaw.com/court/us-9th-circuit/1548042.html)).

- **Breach a condition** → licence fails → copyright infringement → statutory damages
  up to $150,000 per work wilfully infringed, plus fees.
- **Breach a covenant** → breach of contract only → actual damages, likely nominal
  for a free tool.

Anorak is in a **stronger position than Glider**: it does not run alongside the game,
does not automate play, touches no live server, evades no anti-cheat, and does not
alter or copy the game software.

### 9.5 File formats under US law

**17 U.S.C. §102(b)**: "In no case does copyright protection … extend to any idea,
procedure, process, system, method of operation, concept, principle, or discovery"
([Cornell LII](https://www.law.cornell.edu/uscode/text/17/102)).

*Baker v. Selden*, 101 U.S. 99 (1880): "Blank account-books are not the subject of
copyright" ([Cornell LII](https://www.law.cornell.edu/supremecourt/text/101/99)). A
save file's structure is the ledger ruling, not the expressive work.

*Sega v. Accolade*, 977 F.2d 1510 (9th Cir. 1992): "Where disassembly is the only way
to gain access to the ideas and functional elements embodied in a copyrighted
computer program … disassembly is a fair use"
([BitLaw](https://www.bitlaw.com/source/cases/copyright/Sega-Accolade.html)). *Sony v.
Connectix*, 203 F.3d 596 (9th Cir. 2000) reached the same result for intermediate
copying of the PlayStation BIOS
([Copyright Office summary](https://www.copyright.gov/fair-use/summaries/sony-connectix-9thcir2000.pdf)).
Both cover conduct *more* invasive than reading a file.

*Google v. Oracle*, 593 U.S. 1 (2021) held Google's copying of the Java SE declaring
code "was a fair use of that material as a matter of law", and described declaring
code as "further than are most computer programs … from the core of copyright"
([Cornell LII](https://www.law.cornell.edu/supremecourt/text/18-956)). Note its shape:
it is a **fair use** holding that expressly assumed copyrightability without deciding
it, so it supports rather than supplies the argument.

### 9.6 CFAA — no exposure

18 U.S.C. §1030 requires access "without authorization" or that "exceeds authorized
access" ([Cornell LII](https://www.law.cornell.edu/uscode/text/18/1030)). The user
runs the tool on their own computer, on their own file. *Van Buren v. United States*,
593 U.S. 374 (2021) adopted a "gates-up-or-down" test and rejected treating
terms-of-service violations as CFAA crimes
([Cornell LII](https://www.law.cornell.edu/supremecourt/text/19-783)). Confidence:
**settled — no CFAA exposure**, unless the tool starts touching remote servers.

### 9.7 §1201 triennial exemptions — none apply, and none are needed

The 2024 rulemaking was the ninth triennial; the tenth is in progress for exemptions
running October 2027–2030 ([copyright.gov/1201](https://www.copyright.gov/1201/)).
The video game classes at 37 CFR 201.40(b) cover games with discontinued
authentication servers, and preservation by **eligible libraries, archives and
museums** where the game is "no longer reasonably available in the commercial
marketplace" and access is on-premises only
([Cornell LII](https://www.law.cornell.edu/cfr/text/37/201.40)). The 2024 rulemaking
**denied** expansion to off-premises access
([Final Rule PDF](https://www.govinfo.gov/content/pkg/FR-2024-10-28/pdf/2024-24563.pdf)).
FM is commercially available and its servers are live, so nothing applies — which is
fine, because exemptions are only needed if you are circumventing something.

---

## 10. The two biggest risks

The brief asked for these to be flagged separately, because they are different in kind
and the mitigations do not overlap.

### 🚩 Biggest LEGAL risk: EULA clause 7(h), as a contract claim

**Not copyright. Not the DMCA. Not database right. Not reverse engineering.**

Every one of those comes out reasonably well:

- The **format** is not protected (*SAS*, settled).
- He **did not decompile** anything, so s50B and the *LzLabs* trap do not apply.
- The anti-reverse-engineering clause 7(e) is **expressly subject to "except where
  permitted by law"**, and s296A voids it to the extent it restricts observing and
  studying anyway.
- **No TPM was circumvented** — the save is compressed, not encrypted, and Denuvo
  guards an executable he never touches.
- The **user's** extraction is protected by s50D/s296B and reg 19, and contract cannot
  take that away.
- **Accessory liability is weak** (*CBS Songs v Amstrad*; *OBG v Allan*).

What is left is **clause 7(h)**:

> "create data or executable programs which mimic data or functionality in the Product
> unless such functionality is provided to you in the Editors"

It is the one clause aimed squarely at what Anorak *is*, rather than at how it was
built. And it is dangerous for three specific reasons:

1. **No "except where permitted by law" carve-out** — unlike 7(e), which concedes the
   statutory override in its own text.
2. **s296A probably does not reach it.** s296A voids terms restricting *observing,
   studying, testing*, back-ups and s50B decompilation. 7(h) restricts *creating a
   program*. That is a different act, and the statutory list is closed.
3. **The "Editors" definition is circular** — the exception applies only to software
   "authorised … by SEGA", which Anorak by definition is not.

**Why it is nonetheless survivable:** the consequence is a **breach of contract claim
against him personally**, not copyright infringement. No statutory damages, no
additional damages, no criminal exposure, damages likely nominal for a free tool, and
SEGA must litigate in the English courts at its own cost. The realistic worst case is a
letter and a takedown, not a judgment.

**Honest uncertainty:** whether 7(h) is enforceable at all is **genuinely unresolved**.
The arguments against it — purposive reading of s296A/Article 8, the principle that
contract should not extend copyright into unprotected subject matter (*SAS*), and the
Consumer Rights Act 2015 unfairness test — are all respectable and all **untested**. I
found no case applying the CRA to a game EULA's anti-tooling clause. Nobody should be
confident here, in either direction.

### ⚠️ Biggest PRACTICAL (non-legal) risk: unilateral platform action

**SEGA does not need to be legally right to hurt him, and does not need a lawyer.**

The mechanisms that require no legal process, no notice, no proof and no court:

- **A Steam / SEGA account ban.** The licence is "fully revocable" on its face. This
  costs him his FM licence — i.e. **the game he needs in order to develop and test the
  tool at all** — and possibly his wider Steam library. It is free for SEGA to do.
- **A complaint to Apple** about a notarised app under his Developer ID. Losing a
  Developer ID has consequences well beyond this project.
- **A complaint to GitHub, a host, or a package manager**, which typically act first
  and ask questions later.

This asymmetry is the real story: the legal analysis is largely favourable, and it
**does not protect him from any of the above**. Eighteen years of zero enforcement
(§7.1) is genuine comfort, but it is comfort about SI's *disposition*, not about his
*position*.

Mitigations, in order of value: keep the development Steam account separate from the
published project (§8.9); self-host binaries so no platform is a single point of
failure (§8.6); publish a contact address so complaints reach him first (§8.8); and
decide in advance that he will comply immediately with any request rather than fight.

### Honourable mentions (product, not legal)

- **CA/PA is not located yet.** `SAVE_FORMAT.md` §6 is candid that the `+13` candidate
  is probably not Current Ability. His own design rule — "a wrong Current Ability is
  worse than a missing one" — is exactly right, and shipping a plausible-but-wrong
  number would do more damage to the project than any letter from SEGA.
- **Format churn.** FM moved 26.0.0 → 26.3.2 within months. Competing tools ship
  updates "within hours of new FM patches" (§7.6). That is the actual ongoing cost of
  publishing, and it is a commitment to strangers, not just a hobby.
- **Donation admin.** See §5 on the trading allowance — small, but real once money
  moves.

---

## 11. What I could not verify

Stated explicitly rather than papered over.

| Gap | Status |
| --- | --- |
| **An FM26-specific EULA** | `privacy.sega.com/en/fm26-eula-…` returns **404**. FM26's Steam EULA link resolves to `privacy.sega.com/en/fm_eula`, which serves the **SEGA Europe EULA effective 12 Dec 2024**. I infer that governs FM26; SEGA does not say so in terms. **Re-check before relying on it.** |
| **UK IPO register entries** | Did not complete the register search. No registration numbers, owners or classes for "FOOTBALL MANAGER", "SPORTS INTERACTIVE" or "FM". Also unchecked: whether **"Anorak"** is clear in class 9. |
| **Miles Jacobson / official SI statements on third-party tools** | Searched interviews, X/Twitter, LinkedIn and the SI forums. **Found nothing.** Reported as searched-and-not-found, not as "he never commented" — his accounts and 20+ years of forum posts are not fully indexed. |
| **Private cease-and-desist letters** | The GitHub DMCA corpus covers only published DMCA notices to GitHub. Private letters, notices to other hosts, and contract/trade mark claims outside the DMCA process **would not appear**. Zero published takedowns ≠ zero enforcement. |
| **Whether SI licenses player data from third parties** | The brief assumed this. I could **not** verify it and the evidence points the other way — SI's attribute data comes from its own ~1,300-strong researcher network, and its FM26 licensing announcements cover leagues, clubs and competitions only. |
| **CJEU C-406/10 primary text** | curia.europa.eu returned 403 and EUR-Lex returned empty. *SAS* holdings are sourced from the UK Court of Appeal record and reputable secondary analysis (RPC, Kluwer, 8 New Square) rather than the judgment text itself. |
| **CRA 2015 applied to a game EULA** | Found **no case** applying the unfair terms regime to a game EULA's anti-reverse-engineering or anti-tooling clause. The §3.5 argument is untested. |
| **Compression as a TPM** | Found **no UK or US case** deciding whether plain, unencrypted compression is a technological protection measure. The (strong) argument that it is not rests on statutory purpose, not authority. |

---

## 12. Primary sources

**UK statute**

- [CDPA 1988 s50B (decompilation)](https://www.legislation.gov.uk/ukpga/1988/48/section/50B)
- [CDPA 1988 s50BA (observing, studying, testing)](https://www.legislation.gov.uk/ukpga/1988/48/section/50BA)
- [CDPA 1988 s50D (databases — lawful user)](https://www.legislation.gov.uk/ukpga/1988/48/section/50D)
- [CDPA 1988 s28A (temporary copies)](https://www.legislation.gov.uk/ukpga/1988/48/section/28A)
- [CDPA 1988 s296A (void terms — programs)](https://www.legislation.gov.uk/ukpga/1988/48/section/296A)
- [CDPA 1988 s296B (void terms — databases)](https://www.legislation.gov.uk/ukpga/1988/48/section/296B)
- [CDPA 1988 s296ZA (circumvention)](https://www.legislation.gov.uk/ukpga/1988/48/section/296ZA) · [s296ZF (TPM definitions)](https://www.legislation.gov.uk/ukpga/1988/48/section/296ZF)
- [Copyright and Rights in Databases Regulations 1997, reg 19](https://www.legislation.gov.uk/uksi/1997/3032/regulation/19/made) · [reg 20](https://www.legislation.gov.uk/uksi/1997/3032/regulation/20/made)
- [Trade Marks Act 1994 s11](https://www.legislation.gov.uk/ukpga/1994/26/section/11)
- [Retained EU Law (Revocation and Reform) Act 2023](https://www.legislation.gov.uk/ukpga/2023/28)
- [Directive 2009/24/EC art 5](https://www.legislation.gov.uk/eudr/2009/24/article/5) · [art 8](https://www.legislation.gov.uk/eudr/2009/24/article/8)

**Cases**

- *SAS Institute v World Programming* (CJEU C-406/10; [2013] EWCA Civ 1482) — [overview](https://en.wikipedia.org/wiki/SAS_Institute_Inc_v_World_Programming_Ltd) · [RPC](https://www.rpclegal.com/thinking/ip/no-copyright-in-software-functionality-sas-v-wpl-the-final-chapter/) · [Kluwer](https://legalblogs.wolterskluwer.com/copyright-blog/decrypting-the-code-cjeu-sas-vs-world-programming/)
- *IBM v LzLabs* [2025] EWHC 532 (TCC) — [RPC analysis](https://www.rpclegal.com/thinking/tech/reverse-engineering-of-ibm-mainframe-software-in-breach-of-software-licence-ibm-v-lzlabs-part-1/) · [judgment PDF](https://www.brickcourt.co.uk/images/uploads/articles/IBM_final_judgment.pdf) · PTA refused [2025] EWCA Civ 842
- *British Horseracing Board v William Hill* (C-203/02) — [judgment PDF](https://curia.europa.eu/juris/showPdf.jsf?docid=64559&pageIndex=0&doclang=en&mode=req&occ=first&part=1&cid=9204340)
- *Football Dataco v Yahoo!* (C-604/10) — [Kluwer](https://legalblogs.wolterskluwer.com/copyright-blog/football-dataco-skill-and-labour-is-dead/)
- *Nintendo v PC Box* (C-355/12) — [Lexology](https://www.lexology.com/library/detail.aspx?g=ebc62273-e213-44da-b457-288752f4ea6a)
- *Gillette v LA-Laboratories* (C-228/03) — [full text PDF](https://www.ippt.eu/sites/ippt/files/2005/IPPT20050317_ECJ_Gillette_v_LA_Laboratories.pdf)
- *CBS Songs v Amstrad* [1988] UKHL 15 — [judgment PDF](https://www.ip4all.co.uk/wp-content/uploads/cbslimitedvamstradhol.pdf)
- *OBG Ltd v Allan* [2007] UKHL 21 — [House of Lords](https://publications.parliament.uk/pa/ld200607/ldjudgmt/jd070502/obg-1.htm)
- *Manchester United v SEGA & SI* — [William Fry](https://www.williamfry.com/knowledge/manchester-united-settle-trade-mark-dispute-with-sega/)
- US: *Lexmark v Static Control* · *MDY v Blizzard* · *Chamberlain v Skylink* · *Vernor v Autodesk* · *Davidson v Jung* · *Bowers v Baystate* · *Sega v Accolade* · *Sony v Connectix* · *Google v Oracle* · *Van Buren* — URLs inline at §9

**The EULAs**

- [SEGA Europe EULA (eff. 12 Dec 2024) — operative for FM26](https://privacy.sega.com/en/sega-europe-end-user-license-agreement)
- [FM26 Steam EULA link](https://store.steampowered.com/eula/3551340_eula_0) → [privacy.sega.com/en/fm_eula](https://privacy.sega.com/en/fm_eula)
- [FM24 EULA](https://privacy.sega.com/en/fm24-eula-end-user-license-agreement) · [FM23 EULA](https://privacy.sega.com/en/fm23-eula-end-user-license-agreement) · [FM22 EULA](https://store.steampowered.com/eula/1569040_eula_1)
- [Steam Subscriber Agreement](https://store.steampowered.com/subscriber_agreement/)

**Precedent / ecosystem**

- [GitHub DMCA notice corpus](https://github.com/github/dmca) — 21,613 notices to 31 Jul 2026, zero FM matches
- [FM Genie Scout 26](https://www.fmscout.com/a-fm-genie-scout-26.html) · [FMRTE pricing](https://www.fmrte.com/fmrte/) · [FMRTE EULA](https://www.fmrte.com/eula/)
- [SI "Editors Hideaway" forum](https://community.sports-interactive.com/forums/forum/26-editors-hideaway/) · [2011 Genie Scout thread](https://community.sports-interactive.com/forums/topic/226045-the-official-fm-genie-scout-12-thread/)
- [FM26 In-Game Editor (Steam, £7.49)](https://store.steampowered.com/app/3551410/Football_Manager_26_InGame_Editor/) · [FM Live Editor 26](https://www.fmscout.com/a-fm-live-editor-26.html)
- [MacScout26](https://github.com/Einzigart/MacScout26) · [fm-save-parser](https://github.com/dylanvir/fm-save-parser) · [fm-explorer](https://github.com/robeady/fm-explorer)
