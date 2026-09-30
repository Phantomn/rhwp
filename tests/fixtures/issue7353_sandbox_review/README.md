# Original sandbox document reference

- Input: `samples/issue4090/156492236_규제샌드박스_min.hwpx`, unchanged.
- Input SHA256: `d6f4d431b9a4d934b3b4e4330546ef61768c953c2e1328010d2f75440fefa070`.
- `sandbox-original-2020.pdf`: direct Windows Hancom PDF, not a re-saved input.
- PDF SHA256: `667052d8cc5b29d4a2a7271de272f7cb13d6c7845267eeab0e69af45ca3c9411`.
- Job: `2e11ab6c-00a7-41dd-9a4a-c3b8ec05c17c`, 2026-09-28;
  engine2020, Hancom11.0.0.9136, `hwp-managed-direct-dll-host`,
  `input_preprocess=none`, 17pages, 545118bytes.

Independent observations: page3 continues page2's main story; pages6/8/16
contain the final line from pages5/7/15 respectively. Page17 ends with
`감소 등 친환경 소비도 확대될 것으로 기대되고 있습니다.`
The intentionally empty picture placeholders appear in both outputs.

Page5 title-frame path centers (pt, top-down): x58.049..542.468;
y58.017..100.570 and482.832..546.003. The PDF uses a600dpi device grid
with observed text transformation(.119935,.119869)pt per device unit,
not exactly(.12,.12). Coordinate contracts apply this independent printer
scale and allow two device dots(.24pt) rounding, without altering production
coordinates or the comparison images.

`mutool draw -q -F trace -o page5-trace.xml sandbox-original-2020.pdf 5`
reproduces these observations. Text extraction is an auxiliary check, not
evidence of layout fidelity by itself.

Formal contracts: `tests/cases/issue_7353_sandbox_document.rs`. The two tests
protect supported original-document output; they are not a new defect's
before/after claim. Full17page Native/freshWASM compare/overlay/review and
direct visual findings are recorded in `mydocs/working/task_m100_7353_stage19.md`
under the original-document review. Font outlines, weights and symbol widths
are not asserted identical to Hancom.
