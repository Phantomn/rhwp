# Stored LEFT tab review

This is a separated, normal Hancom-save control, **not** the full #6923 document.
`create.rs` reads `issue6923/148738070_wrapper_table_stored_page_frame.hwp`,
selects s0/p5/t0/c0/p37/t0/c0/p0 (` 창원공장\t  `), and preserves that paragraph's
text, character shapes, source offsets, stored line and inline tab extension.
It isolates the cell in a two-row, one-column TAC table, removes unrelated
merged-cell/zone geometry, and sets each row's declared height to3000HU.
The second row is a clone with `LEFT\tRIGHT`, cleared line segments, and raw
offsets accounting for the8-unit tab. `AFTER TABLE` is a separate body paragraph.
These structural edits are disclosed; the unchanged original remains the
whole-document admission test, not a silently repaired input.

Compile `create.rs` against rhwp and run from repository root with
`output/7353/r19/tabs/` present. It writes `tab-input.hwpx`. Use the configured
`hwp2024-mcp-convert start --input ... --target hwp --engine 2020` service to
normal-save `tab-saved.hwp`, then create `tab-2020.pdf` from that exact HWP with
`--target pdf --engine 2020`. Download the successful jobs; no PDF coordinates
or normal-save LineSegs were manually authored.

- HWP job: `0e169703-1a46-4041-9c35-94b24efdea0d`
- PDF job: `3475f87c-ef5a-4ef7-a23f-43875e198564`
- HOffice11.0.0.9136 /32-bit managed direct DLL / preprocessing none;
  session0 font scope2 mapped and2 registered; PDF one-up method0, one physical page.

SHA-256:

- `tab-input.hwpx`: `4aca8eccb4368cdeca1900f116b9211b4f83a282b9ef19c44ec52accf2b5157e`
- `tab-saved.hwp`: `6e1cee25b2a19486fd74f71766a8d8dfaa2afdd4a44618d7132bfb123efd7e11`
- `tab-2020.pdf`: `14ce759510efb133a168519ba0d28156a2c73b45428fdb3a735199271e4cd4a8`

Independent observations: the first paragraph retains2924HU LEFT advance;
the normally saved visible control has1308HU LEFT advance. Both have a
22220HU stored lane,1200HU line height,1020HU baseline, and720HU spacing.
Normal HWP writes spaces in reserved extension words; the original has zeros.
These words are not grounds for rejecting the valid tab. The PDF shows the
title, LEFT/RIGHT gap, two row boundaries, and AFTER TABLE directly below.
Each3000HU row centers its1200HU line at900HU; outer height is6000HU.

`issue_7353_table_v2_document_flow.rs` checks final SVG glyph displacement,
preserved line membership, final line/outer-table origins and following body.
`issue_7353_table_v2_text.rs` adds separate synthetic contracts: different tab
widths in different paragraphs across a cell split; a tab before/after/between
styled text or spaces; unsupported input rejection. Synthetic contracts are
not evidence of normal Hancom automatic pagination.

Qualified scope: one intact saved row and one explicit LEFT inline tab per
paragraph, positive low-word width, no leader,96dpi. The common engine's
multi-row tab ordinal replay, other tab types, high-word widths, non96dpi and
fresh/recomputed tabs remain unqualified. The default Legacy engine is unchanged.
