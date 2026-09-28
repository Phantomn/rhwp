# Deferred page-number declaration

Authored controls for #7353, not excerpts from #7243. Source starts with the
normal `issue7353_tac_noop_review/noop-saved.hwp` style/page definitions; its
body is replaced with 50 labelled paragraphs and a single empty paragraph
containing PageNumberPos(format0, bottom-center, dash). `late` places it at
paragraph index25; `second` at index40. No manually invented LineSeg is used.

Each `*-input.hwpx` was saved normally by Hancom2020 (11.0.0.9136) to
`*-saved.hwp`. That exact HWP was independently printed to `*-2020.pdf`.
Generation diagnostics: `output/7353/r19/late-page-number/create.rs` and
conversion job/download records. PDF driver: Hancom2020 one-up, no transforms.

| Fixture | HWP save job | PDF job |
|---|---|---|
| late | 7f625e5c-5188-42d7-9bb5-4d7e6e197883 | 96087afe-e90f-472b-be0d-3276ac4b5db8 |
| second | ec5171f2-41f3-4b9b-b255-ab4d80e6ba0d | 57cbb330-25a9-4b23-bab3-467dae19ee25 |

Independent contracts:

- Both PDFs have two physical pages. `late` has `- 1 -`/`- 2 -`; `second`
  has no number on page1 and `- 2 -` on page2. Activation does not restart numbering.
- The empty host remains a real line: normal saved height1000HU, gap500HU;
  the following paragraph is1500HU below. In `second`, it separates ROW40/42
  on page2. It is not a zero-height structural slot.
- A derived no-story control removes only PageNumberPos, not that line.
  Exact body equality checks story isolation. HWPX roundtrip checks the other
  parser but is not an independently authored Hancom HWPX fidelity result.

Initial visual failure: the old automatic-number y formula placed this footer
about18px too high. PDF page2 ink top556.592669pt, bottom566.537319pt. The
follow-up `../footer-position/` matrix qualifies the corrected default footer
anchor and baseline with independent PDF text origins. Activation/blank-line
contracts and the new baseline contract remain separate. Maintainer visual
acceptance of the corrected output is still pending; see stage19 evidence.

SHA256:

```text
cc59bcd01acfce010a4d486fb1895c1ee89cf21923962327aa07396d35881d03 late-input.hwpx
d455bf467b49bf1f67c558fe65211ddc82f1e15b920eb2b1dfdff2c4c51f18d7 late-saved.hwp
83980d54f8437068a8c80686faad2e8a14c10c53103526a55af117c1f27dfd61 late-2020.pdf
02b0a87b80c658122b97821d8d28975be08d5175fa38f9c4823d107671e33bb7 second-input.hwpx
f000277c3fb5bc08f6fd890ddf7cdbee6d1b28d1fdf84aa39394d1c22862ce8e second-saved.hwp
e107b55d0040610e12e2aa84b9e4d5a344eec52b8873d488a99c2a2f012eabd0 second-2020.pdf
```
