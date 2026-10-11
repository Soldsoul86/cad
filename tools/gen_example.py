"""The built-in examples (millimetres, $INSUNITS = 4).

Usage: tools/.venv/bin/python tools/gen_example.py crates/lightable/assets

- bracket-rev-b.dxf / bracket-rev-c.dxf (the default example): a steel
  mounting bracket fabrication drawing with a title block. Rev C has
  exactly 4 design changes: one hole moved 5 mm, one hole Ø12 → Ø14, the
  thread note M12 → M16 and a 10 × 10 chamfer on one corner, plus revision
  markup: a revision cloud around the chamfer and the title block's
  revision letter.
- plan-rev-a.dxf / plan-rev-b.dxf: two revisions of a small apartment
  plan showing each kind of change the diff reports: a moved door, a
  removed wall, a new kitchen island, a renamed room, a changed floor
  hatch, an edited block definition (the chair, used by several
  references), a layer colour change and a new dimension.

Everything in them is generated here with ezdxf (MIT), so the files are
ours to ship under the project's licence. Each later revision is the same
document edited, so unchanged entities keep their handles, as when a CAD
program saves a new revision.
"""

import sys
from pathlib import Path

import ezdxf
from ezdxf.enums import TextEntityAlignment

OUT = Path(sys.argv[1])
OUT.mkdir(parents=True, exist_ok=True)


def build():
    """Revision A; returns the document and the entities B edits."""
    doc = ezdxf.new("R2018", setup=True)
    doc.header["$INSUNITS"] = 4
    for name, color in [("WALLS", 7), ("DOORS", 3), ("WINDOWS", 4), ("FURNITURE", 8), ("TEXT", 2),
                        ("FLOOR", 9), ("DIMENSIONS", 1)]:
        doc.layers.add(name, color=color)
    chair = doc.blocks.new("CHAIR")
    chair.add_lwpolyline([(-220, -220), (220, -220), (220, 220), (-220, 220)], close=True)
    chair.add_line((-220, 160), (220, 160))
    doc.blocks.new("TABLE").add_circle((0, 0), 500)
    msp = doc.modelspace()
    e = {}
    walls = {"layer": "WALLS"}
    # Outer walls (10 m × 7 m), inner and outer faces.
    msp.add_lwpolyline([(0, 0), (10000, 0), (10000, 7000), (0, 7000)], close=True, dxfattribs=walls)
    msp.add_lwpolyline([(200, 200), (9800, 200), (9800, 6800), (200, 6800)], close=True, dxfattribs=walls)
    msp.add_line((6000, 200), (6000, 3000), dxfattribs=walls)
    msp.add_line((6000, 3900), (6000, 6800), dxfattribs=walls)
    msp.add_line((6000, 4500), (9800, 4500), dxfattribs=walls)
    e["kitchen_wall"] = msp.add_line((200, 4200), (2600, 4200), dxfattribs=walls)
    doors = {"layer": "DOORS"}
    msp.add_arc((4000, 200), 900, 0, 90, dxfattribs=doors)
    msp.add_line((4000, 200), (4000, 1100), dxfattribs=doors)
    e["bed_door_arc"] = msp.add_arc((6000, 3000), 900, 90, 180, dxfattribs=doors)
    e["bed_door_leaf"] = msp.add_line((6000, 3000), (5100, 3000), dxfattribs=doors)
    for x in (1000, 3000, 7500):
        msp.add_lwpolyline([(x, 6800), (x + 1200, 6800), (x + 1200, 7000), (x, 7000)], close=True,
                           dxfattribs={"layer": "WINDOWS"})
    msp.add_blockref("TABLE", (3000, 2500), dxfattribs={"layer": "FURNITURE"})
    for x, y, rot in [(3000, 1750, 0), (3000, 3250, 180), (2250, 2500, 270), (3750, 2500, 90)]:
        msp.add_blockref("CHAIR", (x, y), dxfattribs={"layer": "FURNITURE", "rotation": rot})
    e["bath_floor"] = h = msp.add_hatch(color=256, dxfattribs={"layer": "FLOOR"})
    h.paths.add_polyline_path([(6200, 4700), (9600, 4700), (9600, 6600), (6200, 6600)], is_closed=True)

    def label(text, x, y):
        t = msp.add_text(text, height=200, dxfattribs={"layer": "TEXT"})
        t.set_placement((x, y), align=TextEntityAlignment.MIDDLE_CENTER)
        return t

    label("LIVING / DINING", 3000, 3800)
    e["kitchen_label"] = label("KITCHEN", 1400, 4600)
    e["bedroom_label"] = label("BEDROOM", 7900, 2400)
    label("BATH", 7900, 5600)
    msp.add_linear_dim(base=(0, -600), p1=(0, 0), p2=(10000, 0), dimstyle="EZDXF",
                       dxfattribs={"layer": "DIMENSIONS"}).render()
    return doc, e


def revise(doc, e):
    """Revision B: the same document edited, so unchanged entities keep
    their handles (as when a CAD program saves a new revision)."""
    msp = doc.modelspace()
    doc.layers.get("FURNITURE").color = 6
    chair = doc.blocks.get("CHAIR")
    chair.add_lwpolyline([(-260, -180), (-220, -180), (-220, 180), (-260, 180)])
    chair.add_lwpolyline([(260, -180), (220, -180), (220, 180), (260, 180)])
    msp.delete_entity(e["kitchen_wall"])  # open-plan kitchen
    e["bed_door_arc"].translate(0, 600, 0)
    e["bed_door_leaf"].translate(0, 600, 0)
    msp.add_lwpolyline([(900, 5000), (2700, 5000), (2700, 5800), (900, 5800)], close=True,
                       dxfattribs={"layer": "FURNITURE"})  # kitchen island
    e["bath_floor"].set_pattern_fill("ANSI37", scale=20, color=256)
    e["kitchen_label"].translate(0, 1600, 0)
    e["bedroom_label"].dxf.text = "MASTER BEDROOM"
    msp.add_linear_dim(base=(10600, 0), p1=(10000, 0), p2=(10000, 7000), angle=90, dimstyle="EZDXF",
                       dxfattribs={"layer": "DIMENSIONS"}).render()


def bracket():
    """Rev B of the bracket; returns the document and what Rev C edits."""
    doc = ezdxf.new("R2018", setup=True)
    doc.header["$INSUNITS"] = 4
    for name, color in [("OUTLINE", 7), ("HOLES", 4), ("NOTES", 2), ("DIMS", 3), ("BORDER", 8),
                        ("TITLE", 8), ("REVISION", 1)]:
        doc.layers.add(name, color=color)
    msp = doc.modelspace()
    e = {}
    # Sheet border and title block (revision markup lives here).
    msp.add_lwpolyline([(0, 0), (297, 0), (297, 210), (0, 210)], close=True, dxfattribs={"layer": "BORDER"})
    msp.add_lwpolyline([(187, 5), (292, 5), (292, 45), (187, 45)], close=True, dxfattribs={"layer": "TITLE"})
    for y in (15, 25, 35):
        msp.add_line((187, y), (292, y), dxfattribs={"layer": "TITLE"})
    msp.add_line((262, 5), (262, 15), dxfattribs={"layer": "TITLE"})

    LEFT = TextEntityAlignment.LEFT

    def text(s, x, y, align, h, layer):
        t = msp.add_text(s, height=h, dxfattribs={"layer": layer})
        t.set_placement((x, y), align=align)
        return t

    # Text on a baseline inside each row (35-45, 25-35, 15-25, 5-15).
    text("STEEL MOUNTING BRACKET", 190, 38.5, LEFT, 3.5, "TITLE")
    text("MATERIAL: S235 STEEL, 6 mm PLATE", 190, 29, LEFT, 2.5, "TITLE")
    text("DRAWING No. LT-0042    SCALE 1:1    mm", 190, 19, LEFT, 2.5, "TITLE")
    text("LIGHTABLE EXAMPLE", 190, 9, LEFT, 2.5, "TITLE")
    e["rev"] = text("REV B", 266, 8.5, LEFT, 3.5, "TITLE")
    # The part: a 160 × 100 plate with four mounting holes and a tapped hole.
    e["outline"] = msp.add_lwpolyline([(40, 60), (200, 60), (200, 160), (40, 160)], close=True,
                                      dxfattribs={"layer": "OUTLINE"})
    e["holes"] = [msp.add_circle(c, 6, dxfattribs={"layer": "HOLES"})
                  for c in [(55, 75), (185, 75), (185, 145), (55, 145)]]
    msp.add_circle((120, 110), 5.1, dxfattribs={"layer": "HOLES"})
    e["thread"] = text("M12", 127, 116, TextEntityAlignment.LEFT, 3.5, "NOTES")
    text("4x Ø12 THRU", 46, 165, LEFT, 3, "NOTES")
    # Readable at 1:1: 3.5 mm text, 3 mm arrows, lengths as drawn (the
    # EZDXF style otherwise scales them by 100).
    style = {"dimtxt": 3.5, "dimasz": 3, "dimexe": 2, "dimexo": 2, "dimgap": 1, "dimlfac": 1}
    msp.add_linear_dim(base=(40, 50), p1=(40, 60), p2=(200, 60), dimstyle="EZDXF", override=style,
                       dxfattribs={"layer": "DIMS"}).render()
    msp.add_linear_dim(base=(22, 60), p1=(40, 60), p2=(40, 160), angle=90, dimstyle="EZDXF", override=style,
                       dxfattribs={"layer": "DIMS"}).render()
    return doc, e


def bracket_rev_c(doc, e):
    """Rev C: exactly 4 design changes, a revision cloud around the
    chamfer, and the new revision letter."""
    msp = doc.modelspace()
    e["holes"][1].dxf.center = (190, 75)       # hole moved 5 mm right
    e["holes"][3].dxf.radius = 7               # Ø12 → Ø14
    e["thread"].dxf.text = "M16"               # thread note M12 → M16
    # 10 × 10 chamfer on the top right corner (200, 160).
    e["outline"].set_points([(40, 60), (200, 60), (200, 150), (190, 160), (40, 160)])
    e["outline"].close(True)
    e["rev"].dxf.text = "REV C"
    # Revision cloud: a closed polyline of arcs around the chamfer.
    x0, y0, x1, y1 = 180, 140, 212, 172
    step = 6.4
    pts = []
    for (ax, ay), (bx, by) in [((x0, y0), (x1, y0)), ((x1, y0), (x1, y1)), ((x1, y1), (x0, y1)),
                               ((x0, y1), (x0, y0))]:
        n = round(((bx - ax) ** 2 + (by - ay) ** 2) ** 0.5 / step)
        for k in range(n):
            pts.append((ax + (bx - ax) * k / n, ay + (by - ay) * k / n, -0.55))
    msp.add_lwpolyline(pts, format="xyb", close=True, dxfattribs={"layer": "REVISION"})


def main():
    doc, e = bracket()
    doc.saveas(OUT / "bracket-rev-b.dxf")
    bracket_rev_c(doc, e)
    doc.saveas(OUT / "bracket-rev-c.dxf")
    doc, e = build()
    doc.saveas(OUT / "plan-rev-a.dxf")
    revise(doc, e)
    doc.saveas(OUT / "plan-rev-b.dxf")


if __name__ == "__main__":
    main()
