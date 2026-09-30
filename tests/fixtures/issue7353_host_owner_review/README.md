# Full-width floating table owner line

Generated HWPX, then normal Hancom2020 save (`11.0.0.9136`) and PDF of that
same saved HWP. No stored LineSeg was manually supplied or changed.

- HWP save job: `dcefe94d-be07-403c-992d-ca1ac7c8556b`.
- PDF job: `5cf08ea8-49bc-4331-97b1-dfb8d3fa4e06`.
- HWP SHA256: `2738c34a6123168e1865ed82a9cb6a797c3d65bea90adf9df4a9f9c1fcfb921c`.
- PDF SHA256: `6138f3a977d668a35b07f99eeeb81ea5de16959582d5ff3b638ed6d57e6b3407`.

24000×30000HU portrait paper,1500HU margins. BEFORE, explicit page break on a
textless TopAndBottom table owner, a21000HU-wide1×1 table containing ROW01–25,
then AFTER. Table policy is RowBreak (file value2 / HWPX CELL / split inside cells).
The supplied HWPX is the reproducible generator output; raw strings are only labels.

Independent PDF: p1 BEFORE; p2 ROW01–19; p3 ROW20–25 then AFTER. At72dpi,
p3 outer bottom is96.759pt, AFTER baseline104.442pt. The saved owner has width0,
height900HU, baseline765HU, following gap450HU. It is an occluded anchor line,
not a new full-width blank inserted after the table. AFTER vpos8175HU plus1500HU
top margin gives129px at96dpi. Cell final paragraph gap is omitted explicitly by
the existing OmitFinalParagraphGap validation policy, not a changed default.

The earlier invalid wide/portrait and CellBreak variants remain in task output
only and are not fidelity fixtures. See stage19 shared-driver evidence for their
rejection, generation history and retained failure output.
