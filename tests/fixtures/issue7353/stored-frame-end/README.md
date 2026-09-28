# Saved frame-end controls

Created for #7353, 2026-09-27. These are authored controls, not a substitute for
the original #6923 nested-table fidelity judgment.

Each `*-input.hwpx` has 65 single-line 14pt cell paragraphs and no hand-written
LineSeg cache. Hancom saved it as `*-saved.hwp`; that exact saved HWP was printed
as `*-2020.pdf`. Saved LineSeg records were not patched. Profile 2020 used Hancom
11.0.0.9136, 32-bit managed host, no input preprocessing, one-up print method 0.
The host paragraph is 100% spacing; the table vertical offset is 2000HU.

| Control | Difference | First frame stored height (HU) | First frame lines |
| --- | --- | ---: | ---: |
| base2 | 100%, cell top/bottom 141HU | 67482 | 48 |
| blank2 | paragraph41 is empty | 67482 | 48 |
| blankend3 | paragraph48 is empty | 67482 | 48 |
| padding2 | cell bottom750HU | 66691 | 47 |
| outer2 | table outer bottom750HU | 66082 | 47 |
| gap3 | cell paragraphs160% | 66642 | 30 |

Expected frame height comes from the independent Hancom save, not rhwp:
last saved line origin + line height + cell top/bottom padding. At96dpi divide
HU by75. A final empty paragraph has a real1400HU line box. The final840HU gap
in gap3 advances a following line and is not part of this frame's bottom.
PDF vector heights are roughly1px shorter than the saved dimensions; no image
translation/scaling is used to hide that difference.

Original conversion jobs and hashes are recorded in the stage19 working log
and local `output/7353/r19/frame-extent/*-{hwp,pdf}-{job,status,download}.json`.
Base HWP job: `9b442820-11f0-44de-8bf3-5026c0698b61`.
Base PDF job: `ca903f95-8fd4-42ee-bcce-b3af31703ca0`.
The fixture bytes, paired PDF and normal authoring inputs are retained here so
CI and future reviews do not depend on the ignored output directory.
