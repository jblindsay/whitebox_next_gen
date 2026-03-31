from pathlib import Path
import html
import sys

import mistune
from reportlab.lib import colors
from reportlab.lib.enums import TA_CENTER, TA_RIGHT
from reportlab.lib.pagesizes import letter
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import inch
from reportlab.platypus import PageBreak, Paragraph, SimpleDocTemplate, Spacer, Table, TableStyle


ACCENT = colors.HexColor("#1F4E79")
ACCENT_DARK = colors.HexColor("#0F2D46")
ACCENT_SOFT = colors.HexColor("#EAF2F8")
BODY = colors.HexColor("#222222")
MUTED = colors.HexColor("#6B7280")
GRID = colors.HexColor("#CBD5E1")
PALE = colors.HexColor("#F8FAFC")
GOLD = colors.HexColor("#C48A1D")


def flatten_text(tokens):
    parts = []
    for token in tokens or []:
        token_type = token.get("type")
        if token_type == "text":
            parts.append(html.escape(token.get("raw", "")))
        elif token_type == "codespan":
            parts.append(f'<font name="Courier">{html.escape(token.get("raw", ""))}</font>')
        elif token_type == "strong":
            parts.append(f'<b>{flatten_text(token.get("children"))}</b>')
        elif token_type == "emphasis":
            parts.append(f'<i>{flatten_text(token.get("children"))}</i>')
        elif token_type == "link":
            label = flatten_text(token.get("children"))
            parts.append(f'<font color="#1F4E79"><u>{label}</u></font>')
        elif token_type in {"softbreak", "linebreak"}:
            parts.append("<br/>")
        else:
            parts.append(flatten_text(token.get("children")))
    return "".join(parts)


def list_item_text(item_token):
    parts = []
    for child in item_token.get("children", []):
        if child.get("type") in {"block_text", "paragraph"}:
            parts.append(flatten_text(child.get("children")))
        elif child.get("type") != "list":
            parts.append(flatten_text(child.get("children")))
    return " ".join(p for p in parts if p).strip()


def table_to_data(table_token):
    rows = []
    for child in table_token.get("children", []):
        if child.get("type") == "table_head":
            rows.append([flatten_text(cell.get("children")) for cell in child.get("children", [])])
        elif child.get("type") == "table_body":
            for row in child.get("children", []):
                rows.append([flatten_text(cell.get("children")) for cell in row.get("children", [])])
        elif child.get("type") == "table_row":
            rows.append([flatten_text(cell.get("children")) for cell in child.get("children", [])])
    return rows


def compute_col_widths(rows, total_width):
    if not rows:
        return []
    num_cols = max(len(r) for r in rows)
    lengths = [1] * num_cols
    for row in rows:
        for i, cell in enumerate(row):
            text = cell.replace("<br/>", " ")
            lengths[i] = max(lengths[i], min(len(text), 40))
    total = sum(lengths)
    widths = [total_width * value / total for value in lengths]
    min_width = total_width * 0.14
    widths = [max(min_width, w) for w in widths]
    scale = total_width / sum(widths)
    return [w * scale for w in widths]


def build_styles():
    base = getSampleStyleSheet()
    styles = {}
    styles["CoverTitle"] = ParagraphStyle(
        "CoverTitle",
        parent=base["Title"],
        fontName="Helvetica-Bold",
        fontSize=28,
        leading=34,
        alignment=TA_CENTER,
        textColor=colors.white,
        spaceAfter=12,
    )
    styles["CoverSubtitle"] = ParagraphStyle(
        "CoverSubtitle",
        parent=base["BodyText"],
        fontName="Helvetica",
        fontSize=12,
        leading=17,
        alignment=TA_CENTER,
        textColor=colors.white,
        spaceAfter=8,
    )
    styles["CoverMeta"] = ParagraphStyle(
        "CoverMeta",
        parent=base["BodyText"],
        fontName="Helvetica",
        fontSize=10,
        leading=14,
        alignment=TA_CENTER,
        textColor=colors.HexColor("#D9E7F3"),
    )
    styles["SectionBar"] = ParagraphStyle(
        "SectionBar",
        parent=base["Heading2"],
        fontName="Helvetica-Bold",
        fontSize=14,
        leading=18,
        textColor=colors.white,
        alignment=TA_CENTER,
        spaceAfter=0,
    )
    styles["Heading3"] = ParagraphStyle(
        "Heading3",
        parent=base["Heading3"],
        fontName="Helvetica-Bold",
        fontSize=12,
        leading=15,
        textColor=ACCENT_DARK,
        spaceBefore=8,
        spaceAfter=4,
    )
    styles["Heading4"] = ParagraphStyle(
        "Heading4",
        parent=base["Heading4"],
        fontName="Helvetica-Bold",
        fontSize=10.8,
        leading=13,
        textColor=colors.HexColor("#35556E"),
        spaceBefore=6,
        spaceAfter=3,
    )
    styles["Body"] = ParagraphStyle(
        "Body",
        parent=base["BodyText"],
        fontName="Helvetica",
        fontSize=10.3,
        leading=14.3,
        textColor=BODY,
        spaceAfter=7,
    )
    styles["Lead"] = ParagraphStyle(
        "Lead",
        parent=styles["Body"],
        fontSize=11.2,
        leading=15.8,
        textColor=colors.HexColor("#1F2937"),
        spaceAfter=8,
    )
    styles["Bullet"] = ParagraphStyle(
        "Bullet",
        parent=styles["Body"],
        leftIndent=14,
        spaceAfter=4,
    )
    styles["TableHeader"] = ParagraphStyle(
        "TableHeader",
        parent=styles["Body"],
        fontName="Helvetica-Bold",
        fontSize=9.4,
        leading=11.5,
        textColor=colors.white,
        spaceAfter=0,
    )
    styles["TableCell"] = ParagraphStyle(
        "TableCell",
        parent=styles["Body"],
        fontSize=9.1,
        leading=11.3,
        spaceAfter=0,
    )
    styles["RatingBig"] = ParagraphStyle(
        "RatingBig",
        parent=base["Title"],
        fontName="Helvetica-Bold",
        fontSize=22,
        leading=24,
        textColor=ACCENT_DARK,
        alignment=TA_CENTER,
        spaceAfter=1,
    )
    styles["RatingLabel"] = ParagraphStyle(
        "RatingLabel",
        parent=styles["Body"],
        fontName="Helvetica-Bold",
        fontSize=8.5,
        leading=10,
        textColor=MUTED,
        alignment=TA_CENTER,
        spaceAfter=0,
    )
    return styles


def section_divider(text, styles, doc_width):
    t = Table([[Paragraph(text, styles["SectionBar"])]], colWidths=[doc_width])
    t.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, -1), ACCENT),
        ("LEFTPADDING", (0, 0), (-1, -1), 10),
        ("RIGHTPADDING", (0, 0), (-1, -1), 10),
        ("TOPPADDING", (0, 0), (-1, -1), 7),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 7),
        ("ROUNDEDCORNERS", [6, 6, 6, 6]),
    ]))
    return t


def rating_box(value, styles, doc_width):
    card = Table(
        [[Paragraph(value, styles["RatingBig"]), Paragraph("THEMATIC RATING", styles["RatingLabel"]) ]],
        colWidths=[doc_width * 0.22, doc_width * 0.28],
    )
    card.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, -1), ACCENT_SOFT),
        ("BOX", (0, 0), (-1, -1), 0.8, GRID),
        ("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
        ("LEFTPADDING", (0, 0), (-1, -1), 10),
        ("RIGHTPADDING", (0, 0), (-1, -1), 10),
        ("TOPPADDING", (0, 0), (-1, -1), 8),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 8),
        ("LINEAFTER", (0, 0), (0, 0), 0.8, GRID),
    ]))
    return card


def build_cover(title, date_line, styles, doc_width):
    story = []
    hero = Table(
        [[
            Paragraph(title, styles["CoverTitle"]),
            Paragraph("Competitive Positioning, Functional Gaps, and Pro-Tier Expansion Opportunities", styles["CoverSubtitle"]),
            Paragraph(date_line, styles["CoverMeta"]),
        ]],
        colWidths=[doc_width],
    )
    hero.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, -1), ACCENT_DARK),
        ("BOX", (0, 0), (-1, -1), 0, colors.white),
        ("LEFTPADDING", (0, 0), (-1, -1), 26),
        ("RIGHTPADDING", (0, 0), (-1, -1), 26),
        ("TOPPADDING", (0, 0), (-1, -1), 42),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 42),
        ("ROUNDEDCORNERS", [10, 10, 10, 10]),
    ]))
    strap = Table(
        [[Paragraph("Prepared from the current Whitebox Next Gen tool inventory and qualitative comparison with ArcGIS Pro, QGIS/GRASS/SAGA, TauDEM, PDAL, ENVI/ERDAS/SNAP, and scientific Python/R workflows.", ParagraphStyle("strap", parent=styles["Body"], textColor=BODY, alignment=TA_CENTER, fontSize=10.6, leading=15))]],
        colWidths=[doc_width * 0.86],
    )
    strap.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, -1), colors.white),
        ("BOX", (0, 0), (-1, -1), 0.7, GRID),
        ("LEFTPADDING", (0, 0), (-1, -1), 16),
        ("RIGHTPADDING", (0, 0), (-1, -1), 16),
        ("TOPPADDING", (0, 0), (-1, -1), 14),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 14),
        ("ALIGN", (0, 0), (-1, -1), "CENTER"),
    ]))
    story.append(Spacer(1, 1.35 * inch))
    story.append(hero)
    story.append(Spacer(1, 0.38 * inch))
    story.append(strap)
    story.append(Spacer(1, 0.35 * inch))
    ribbon = Table([[Paragraph("Executive view: Whitebox is strongest in terrain, geomorphometry, hydrology, LiDAR-terrain workflows, and scientific raster analysis. The clearest strategic opportunity is deeper domain-specialized Pro tools, not generic GIS feature parity.", ParagraphStyle("ribbon", parent=styles["Body"], textColor=colors.white, alignment=TA_CENTER, fontSize=10.2, leading=14.2))]], colWidths=[doc_width * 0.9])
    ribbon.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, -1), GOLD),
        ("LEFTPADDING", (0, 0), (-1, -1), 14),
        ("RIGHTPADDING", (0, 0), (-1, -1), 14),
        ("TOPPADDING", (0, 0), (-1, -1), 10),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 10),
    ]))
    story.append(ribbon)
    story.append(PageBreak())
    return story


def build_story(ast, styles, doc_width):
    story = []
    idx = 0
    if ast and ast[0].get("type") == "heading":
        title = flatten_text(ast[0].get("children"))
        date_line = ""
        for tok in ast[1:6]:
            if tok.get("type") == "paragraph":
                date_line = flatten_text(tok.get("children"))
                break
        story.extend(build_cover(title, date_line, styles, doc_width))
        idx = 1
        while idx < len(ast) and ast[idx].get("type") != "heading":
            idx += 1

    while idx < len(ast):
        token = ast[idx]
        token_type = token.get("type")
        if token_type == "blank_line":
            idx += 1
            continue
        if token_type == "heading":
            level = token.get("attrs", {}).get("level", 1)
            text = flatten_text(token.get("children"))
            if level == 1:
                idx += 1
                continue
            if level == 2:
                story.append(Spacer(1, 8))
                story.append(section_divider(text, styles, doc_width))
                story.append(Spacer(1, 10))
                idx += 1
                continue
            if level == 3:
                story.append(Paragraph(text, styles["Heading3"]))
                if text.strip().lower() == "rating":
                    j = idx + 1
                    while j < len(ast) and ast[j].get("type") == "blank_line":
                        j += 1
                    if j < len(ast) and ast[j].get("type") == "list":
                        items = ast[j].get("children", [])
                        if items:
                            value = list_item_text(items[0])
                            story.append(rating_box(value, styles, doc_width))
                            story.append(Spacer(1, 8))
                            idx = j + 1
                            continue
                idx += 1
                continue
            story.append(Paragraph(text, styles["Heading4"]))
            idx += 1
            continue
        if token_type == "paragraph":
            text = flatten_text(token.get("children"))
            story.append(Paragraph(text, styles["Lead"] if not story or (len(story) and isinstance(story[-1], Table)) else styles["Body"]))
            idx += 1
            continue
        if token_type == "list":
            depth = token.get("attrs", {}).get("depth", 0)
            ordered = token.get("attrs", {}).get("ordered", False)
            for n, item in enumerate(token.get("children", []), start=1):
                bullet = f"{n}." if ordered else "•"
                text = list_item_text(item)
                if text:
                    style = styles["Bullet"].clone(f"BulletDepth{depth}_{n}")
                    style.leftIndent = styles["Bullet"].leftIndent + depth * 16
                    story.append(Paragraph(f'<font color="#1F1F1F">{bullet}</font> {text}', style))
                for child in item.get("children", []):
                    if child.get("type") == "list":
                        story.extend(build_story([child], styles, doc_width))
            story.append(Spacer(1, 3))
            idx += 1
            continue
        if token_type == "table":
            rows = table_to_data(token)
            if rows:
                rendered = []
                for r_idx, row in enumerate(rows):
                    style = styles["TableHeader"] if r_idx == 0 else styles["TableCell"]
                    rendered.append([Paragraph(cell or "&nbsp;", style) for cell in row])
                table = Table(rendered, colWidths=compute_col_widths(rows, doc_width), repeatRows=1)
                table.setStyle(TableStyle([
                    ("BACKGROUND", (0, 0), (-1, 0), ACCENT),
                    ("TEXTCOLOR", (0, 0), (-1, 0), colors.white),
                    ("ROWBACKGROUNDS", (0, 1), (-1, -1), [PALE, colors.white]),
                    ("GRID", (0, 0), (-1, -1), 0.45, GRID),
                    ("VALIGN", (0, 0), (-1, -1), "TOP"),
                    ("LEFTPADDING", (0, 0), (-1, -1), 6),
                    ("RIGHTPADDING", (0, 0), (-1, -1), 6),
                    ("TOPPADDING", (0, 0), (-1, -1), 6),
                    ("BOTTOMPADDING", (0, 0), (-1, -1), 6),
                ]))
                story.append(table)
                story.append(Spacer(1, 10))
            idx += 1
            continue
        idx += 1
    return story


def add_page_chrome(canvas, doc):
    canvas.saveState()
    canvas.setStrokeColor(GRID)
    canvas.setLineWidth(0.6)
    canvas.line(doc.leftMargin, letter[1] - 38, letter[0] - doc.rightMargin, letter[1] - 38)
    canvas.line(doc.leftMargin, 34, letter[0] - doc.rightMargin, 34)
    canvas.setFont("Helvetica", 8)
    canvas.setFillColor(MUTED)
    canvas.drawString(doc.leftMargin, 20, "Whitebox Competitive Assessment")
    canvas.drawRightString(letter[0] - doc.rightMargin, 20, f"Page {canvas.getPageNumber()}")
    canvas.restoreState()


def add_cover_chrome(canvas, doc):
    canvas.saveState()
    canvas.setFillColor(colors.HexColor("#F3F7FA"))
    canvas.rect(0, 0, letter[0], letter[1], fill=1, stroke=0)
    canvas.setFillColor(ACCENT_DARK)
    canvas.rect(0, letter[1] - 140, letter[0], 140, fill=1, stroke=0)
    canvas.setFillColor(colors.HexColor("#D3E1EE"))
    canvas.circle(letter[0] - 70, letter[1] - 70, 90, fill=1, stroke=0)
    canvas.setFillColor(colors.HexColor("#E8EEF3"))
    canvas.circle(70, 70, 110, fill=1, stroke=0)
    canvas.restoreState()


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: md_to_presentation_pdf.py <input.md> <output.pdf>")
    md_path = Path(sys.argv[1])
    pdf_path = Path(sys.argv[2])
    parser = mistune.create_markdown(renderer="ast", plugins=["table"])
    ast = parser(md_path.read_text(encoding="utf-8"))
    styles = build_styles()
    doc = SimpleDocTemplate(
        str(pdf_path),
        pagesize=letter,
        leftMargin=0.78 * inch,
        rightMargin=0.78 * inch,
        topMargin=0.72 * inch,
        bottomMargin=0.62 * inch,
        title="Whitebox Competitive Assessment",
        author="GitHub Copilot",
    )
    story = build_story(ast, styles, doc.width)
    doc.build(story, onFirstPage=add_cover_chrome, onLaterPages=add_page_chrome)
    print(pdf_path)


if __name__ == "__main__":
    main()
