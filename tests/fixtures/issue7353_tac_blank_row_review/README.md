# No-ink stored row before a TAC table

Independent Hancom-saved diagnostic for #7353. This is **not** proof that the
original #6923 document or its negative margins are supported.

`create.rs --simple` starts with the existing TAC-space diagnostic and restores
the original #6923 DocInfo. It selects `s0/p5/t0/c0/p69`: 60 ordinary spaces at
offsets0..59, followed by a TAC table at offset60. Paragraph margins and hanging
indent are preserved. The parent is a26000HU-high1×1 table. The child is replaced
by a readable1×1 table, preserving its45360×14847HU size and original first cell's
padding/alignment, but with the plain label `TABLE ON SECOND LINE`. The following
body paragraph is `AFTER CELL: no missing or duplicated text`. All saved lines
are cleared before serializing fresh HWPX. Hancom owns the final stored lines.

Compile against rhwp and run from the repository root. The script writes
`output/7353/r19/tac-next/space-row-input.hwpx`. Convert using the canonical MCP
client, explicitly `--engine 2020 --timeout-seconds 180`:

1. Input → `space-row-saved.hwp`, job`e4815085-7531-4927-98fb-947f42502d90`.
2. That same saved HWP → `space-row-2020.pdf`, job`62c5ae68-2cf5-47c9-b55e-b4b04c33702c`.

Both succeeded with Hancom11.0.0.9136, managed direct DLL host,32-bit,
preprocess`none`, fonts2registered/0failed. PDF has1page,printMethod0.

- HWP SHA-256: `2cc07256967fcbcf0aa3e15502a8d9bc9ca318b2d3ac3ea8d8e468e09c6344d6`
- PDF SHA-256: `f14d17d323189f8c86e3c459aef572d60f84b0e1fb8c08123ab232e7b022fb4f`

The input XML preserves the original -1HU outer margins. **Hancom normalizes
them to0**, changing the second row's height from14845 to14847HU. The first row
is1400HU high, baseline1190, spacing716; the second starts at2116HU. Its stored
indentation flag adds1000HU to the550HU left margin. Parent padding283HU gives
child displacement(1833,2399)HU from the parent origin. These independent saved
metrics, the same-HWP PDF and actual final RenderTree/SVG are compared, not just
page counts or helper outputs. The body origin is(5669,7087)HU; the parent has
26000HU height and the following paragraph starts another516HU below it.

The native/fresh WASM review is in `output/7353/r19/tac-next/review/`. Compare
parent-to-child top gap, second-row horizontal inset, both table outlines, label
and following body. Line darkness and fallback glyph appearance remain different.

`create.rs` without `--simple` preserves the original child contents, writing
`carrier-input.hwpx`. Its normal HWP/PDF remain in output: HWP job
`2aa91c6d-8364-45bc-a0a1-c13fa0ac635d`, PDF job
`16a5f793-857f-45a0-9e29-ffb4ecbc7117`. After the blank-row change, that diagnostic
still rejects `text preview run outside occupied line`. The untouched original
still rejects its negative TAC margins. Neither is reported as a visual pass.

The split-budget test changes only the parent's break policy in memory; it is a
synthetic contract, not another Hancom reference. It checks a budget too short
for the blank row, a budget1HU too short for the following table, complete blank
row consumption, one child on continuation, final paint origins and termination.
