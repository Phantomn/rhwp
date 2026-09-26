# Saved following-host anchor review

Independent normal Hancom saves, not the full #6923 source document.
`create.rs` reads `issue6923/148738070_wrapper_table_stored_page_frame.hwp`.
It retains p5/t0/c0/p37's TopAndBottom Para/Top anchor, zero vertical offset,
horizontal1620HU and four283HU outer margins. It replaces the child with a
1x1,5000HU-high labelled table, clears source LineSegs and adds labelled
before/after paragraphs plus an authored empty paragraph. The wrapper is TAC.
The second input replaces the literal spaces with `HOST TEXT`, retaining the
same paragraph properties. Thus neither generated input is a source fidelity claim.

Generation: compile `create.rs` against rhwp, run from repository root with
`output/7353/r19/next-anchor2/` present. Serialize HWPX, normal-save to HWP using
Hancom2020, then create the PDF from that exact saved HWP. No source LineSeg
or PDF coordinates are manually authored. HWP saves use HOffice11.0.0.9136,
32-bit managed direct DLL, preprocessing none; PDF one-up print method0.

| Input | HWP job | PDF job |
| --- | --- | --- |
| Literal spaces | ddb7ee31-3542-4eda-8e91-60bb8d9e81f2 | 603162a4-4451-4a13-8d62-d69efd31cffb |
| Visible host | 6d02eeea-af58-4060-9379-83e7faa9cb76 | ef95df01-d355-4d3f-ade7-a6ae11f68e48 |

SHA-256:

- anchor-saved.hwp: `8fac76ee1e9ae6051e3510d1839d5d5331593b4b167b04f95dfa3beb02df0a3d`
- anchor-2020.pdf: `e673661c21a6765c64f0b68f8058157bd4368936e9b562813752077041db13ed`
- visible-saved.hwp: `5dd0fb018e0e693e6326d501844b6c3a760bd40dda209b965c4c48c318c13751`
- visible-2020.pdf: `73b2c6508ba161342f6f3cc63b1c40485aef1c6c39f84ff161ed78ad116fd014`

Both PDFs have one physical page. Saved host vpos6766HU follows the1200HU
before line and child exclusion283+5000+283. Host height1400 and gap-280
produce the following empty paragraph origin7886; its1200HU height produces
AFTER HOST at9086. Parent padding141HU top/bottom and last1200HU line yield
outer height10568HU. The PDF independently shows these relative positions.
The visible host and spaces occupy the same line geometry. Neither is removed.

`tests/cases/issue_7353_table_v2_document_flow.rs` checks the final rendered
origins, parent/child boxes, following body and preserved content. Separate
synthetic cut tests change only page-break policy, alignment and padding to
exercise20/30/40px fragment budgets; these are ownership contracts, not claims
about Hancom's automatic pagination. Fully empty positive-width hosts,
multi-line anchors and nonzero vertical anchor offsets remain unqualified.
