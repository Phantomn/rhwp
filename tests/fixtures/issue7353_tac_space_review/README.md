# Stored space before an inline table

Independent one-page diagnostic for #7353, not a reduced original with fabricated
LineSegs and not evidence that the complete #6923 document is supported.

`create.rs` authors fresh HWPX using the #6923 `s0/p5/t0/c0/p7` carrier:
one ordinary space at source offset0, table control after it, hanging indent-4624HU.
The original three-cell child is replaced with a readable1×1 `SPACE BEFORE TABLE`
control. Its width is21201HU and height5000HU. The parent is48182×15000HU with
283HU cell padding. Parent/label/following prose use a new plain paragraph style;
the original carrier style and its space's character shape are preserved.
Page decoration is disabled for this newly authored diagnostic. All saved lines
are cleared **before** authoring; Hancom owns the final saved lines.

Generation: compile `create.rs` against the local rhwp library and execute from
the repository root. Convert via the canonical asynchronous MCP client
(`tools/hwp-convert-mcp-2024-client-20260824-011002.tar.gz`), explicitly
`--engine 2020 --timeout-seconds 1800`:

1. `space-input.hwpx` → `space-saved.hwp`, job`261ce608-1d2f-48dc-ae31-4f19ffab902c`.
2. The **same saved HWP** → `space-2020.pdf`, job`684416fc-e6d9-40be-b717-9e5948887943`.

Both completed with Hancom11.0.0.9136, managed direct DLL host,32-bit,
preprocess`none`, mapped/registered fonts2,failed0. PDF has1page,printMethod0.

| File | SHA-256 |
| --- | --- |
| space-input.hwpx | c69ae04f7fd6bc1201c72e9df3f111ec88bdbbc52e42b1522217f1decdba841a |
| space-saved.hwp | 0b1285473f1c075afbacd15a8730aa0ba1bd214a130295eed8b7f7fc3d3a36e0 |
| space-2020.pdf | e0793009ae217a228a7025ac270de2d6c34c0b069dd543e39e5e366d2afff286 |

Independent expectations: source body origin(5669,7087)HU, parent padding283HU,
saved carrier height5000HU/baseline4250HU. Its first row has no indentation flag:
the hanging indent must not shift that row. Preserve the space's positive advance
between padding and the child table. PDF vertical edges from `pdftocairo -svg`
are x56.609375pt(parent) and x66.804688pt(child). The regression compares this
relative separation with a300dpi printer-dot budget; exact source dimensions and
space/table adjacency have exact coordinate assertions. This is not a page-count
or golden relaxation.

Review material: `output/7353/r19/tac-insets/review/`, same page/region,96dpi,
no registration transform. Check the nested table's left edge, outer parent,
label and following `AFTER CELL` paragraph. Font appearance and stroke darkness
differences are visible and not evidence of complete visual identity.

Earlier diagnostics remain in `output/7353/r19/tac-insets/`: `carrier-input` had
an insufficient body width; `carrier-wide` inherited a painted page-border
reference when swapping DocInfo; `carrier-plain` preserves the original child's
separate stored-text width limitation; `space-review` has a separate vertical
text-baseline style not qualified by the shared text adapter. These were **not**
passed by weakening admission or editing their saved lines. The final diagnostic
is independently re-authored and normally saved. Original #6923 remains rejected
at its descendant stored-text boundary; the empty child cell's stored1440HU row
exceeds its1303−510−510=283HU bound content width. This discrepancy needs its own
rule investigation, not acceptance based on this diagnostic.
