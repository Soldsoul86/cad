# shaft: expected-change inventory

Drive shaft LT-0020 and the same drawing re-saved by another program: every handle renumbered, the entities in another order, saved as DXF R2000 instead of R2018, the outline polyline split into separate lines. One real change: the surface finish note, Ra 1.6 -> Ra 0.8.

Written by tools/gen_usecases.py from the edits it makes, before the files are compared.

## Design changes (1)

| # | Type | Layer | Expected words |
|---|---|---|---|
| E1 | TEXT | NOTES | Text changed from SURFACE FINISH Ra 1.6 to SURFACE FINISH Ra 0.8 |

## Not changes

- handles renumbered, entity order shuffled, file version R2018 -> R2000
- the outline polyline (16 vertices) written as 16 lines: drawn the same (redrawn)
