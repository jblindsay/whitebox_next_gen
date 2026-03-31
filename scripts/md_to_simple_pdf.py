from pathlib import Path
import html
import sys

import mistune
from reportlab.lib import colors
from reportlab.lib.enums import TA_CENTER, TA_RIGHT
from reportlab.lib.pagesizes import letter
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import inch
from reportlab.platypus import Paragraph, SimpleDocTemplate, Spacer, Table, TableStyle


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
            href = html.escape(token.get("attrs", {}).get("url", ""))
            label = flatten_text(token.get("children")) or href
            parts.append(f'<u><font color="#1F4E79">{label}</font></u>')
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
        elif child.get("type") == "list":
            continue
        else:
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
            lengths[i] = max(lengths[i], min(len(text), 48))
    total = sum(lengths)
    widths = [total_width * value / total for value in lengths]
    min_width = total_width * 0.12
    widths = [max(min_width, w) for w in widths]
    scale = total_width / sum(widths)
    return [w * scale for w in widths]


def build_story(ast, styles, doc_width):
    story = []
    for token in ast:
        token_type = token.get("type")
        if token_type == "blank_line":
            continue
        if token_type == "heading":
            level = token.get("attrs", {}).get("level", 1)
            text = flatten_text(token.get("children"))
            style_name = {1: "TitleCustom", 2: "Heading2Custom", 3: "Heading3Custom", 4: "Heading4Custom"}.get(level, "Heading4Custom")
            story.append(Paragraph(text, styles[style_name]))
            continue
        if token_type == "paragraph":
            text = flatten_text(token.get("children"))
            story.append(Paragraph(text, styles["BodyCustom"]))
            continue
        if token_type == "list":
            depth = token.get("attrs", {}).get("depth", 0)
            ordered = token.get("attrs", {}).get("ordered", False)
            for idx, item in enumerate(token.get("children", []), start=1):
                bullet = f"{idx}." if ordered else "•"
                text = list_item_text(item)
                if text:
                    style = styles["BulletCustom"].clone(f"BulletDepth{depth}")
                    style.leftIndent = styles["BulletCustom"].leftIndent + depth * 16
                    style.firstLineIndent = 0
                    story.append(Paragraph(f'<font color="#1F1F1F">{bullet}</font> {text}', style))
                for child in item.get("children", []):
                    if child.get("type") == "list":
                        story.extend(build_story([child], styles, doc_width))
            story.append(Spacer(1, 4))
            continue
        if token_type == "table":
            rows = table_to_data(token)
            if rows:
                table_rows = []
                for r_index, row in enumerate(rows):
                    row_style = styles["TableHeader"] if r_index == 0 else styles["TableCell"]
                    table_rows.append([Paragraph(cell or "&nbsp;", row_style) for cell in row])
                table = Table(table_rows, colWidths=compute_col_widths(rows, doc_width), repeatRows=1)
                table.setStyle(TableStyle([
                    ("BACKGROUND", (0, 0), (-1, 0), colors.HexColor("#1F4E79")),
                    ("TEXTCOLOR", (0, 0), (-1, 0), colors.white),
                    ("BACKGROUND", (0, 1), (-1, -1), colors.HexColor("#F8FAFC")),
                    ("ROWBACKGROUNDS", (0, 1), (-1, -1), [colors.HexColor("#F8FAFC"), colors.white]),
                    ("GRID", (0, 0), (-1, -1), 0.4, colors.HexColor("#C9D4E3")),
                    ("VALIGN", (0, 0), (-1, -1), "TOP"),
                    ("LEFTPADDING", (0, 0), (-1, -1), 6),
                    ("RIGHTPADDING", (0, 0), (-1, -1), 6),
                    ("TOPPADDING", (0, 0), (-1, -1), 6),
                    ("BOTTOMPADDING", (0, 0), (-1, -1), 6),
                ]))
                story.append(table)
                story.append(Spacer(1, 10))
            continue
    return story


def build_styles():
    base = getSampleStyleSheet()
    styles = {}
    styles["TitleCustom"] = ParagraphStyle(
        "TitleCustom",
        parent=base["Title"],
        fontName="Helvetica-Bold",
        fontSize=22,
        leading=27,
        textColor=colors.HexColor("#0F2D46"),
        spaceAfter=14,
        alignment=TA_CENTER,
    )
    styles["Heading2Custom"] = ParagraphStyle(
        "Heading2Custom",
        parent=base["Heading2"],
        fontName="Helvetica-Bold",
        fontSize=15,
        leading=19,
        textColor=colors.HexColor("#1F4E79"),
        spaceBefore=12,
        spaceAfter=6,
    )
    styles["Heading3Custom"] = ParagraphStyle(
        "Heading3Custom",
        parent=base["Heading3"],
        fontName="Helvetica-Bold",
        fontSize=12,
        leading=15,
        textColor=colors.HexColor("#264653"),
        spaceBefore=8,
        spaceAfter=4,
    )
    styles["Heading4Custom"] = ParagraphStyle(
        "Heading4Custom",
        parent=base["Heading4"],
        fontName="Helvetica-Bold",
        fontSize=11,
        leading=13,
        textColor=colors.HexColor("#2F4858"),
        spaceBefore=6,
        spaceAfter=2,
    )
    styles["BodyCustom"] = ParagraphStyle(
        "BodyCustom",
        parent=base["BodyText"],
        fontName="Helvetica",
        fontSize=10.5,
        leading=14,
        spaceAfter=7,
        textColor=colors.HexColor("#222222"),
    )
    styles["BulletCustom"] = ParagraphStyle(
        "BulletCustom",
        parent=styles["BodyCustom"],
        leftIndent=14,
        spaceAfter=4,
    )
    styles["TableHeader"] = ParagraphStyle(
        "TableHeader",
        parent=styles["BodyCustom"],
        fontName="Helvetica-Bold",
        fontSize=9.5,
        leading=12,
        textColor=colors.white,
        spaceAfter=0,
    )
    styles["TableCell"] = ParagraphStyle(
        "TableCell",
        parent=styles["BodyCustom"],
        fontSize=9.2,
        leading=11.5,
        spaceAfter=0,
    )
    styles["Footer"] = ParagraphStyle(
        "Footer",
        parent=styles["BodyCustom"],
        alignment=TA_RIGHT,
        fontSize=8,
        textColor=colors.HexColor("#666666"),
    )
    return styles


def add_page_chrome(canvas, doc):
    canvas.saveState()
    canvas.setStrokeColor(colors.HexColor("#C9D4E3"))
    canvas.setLineWidth(0.6)
    canvas.line(doc.leftMargin, letter[1] - 42, letter[0] - doc.rightMargin, letter[1] - 42)
    canvas.line(doc.leftMargin, 34, letter[0] - doc.rightMargin, 34)
    canvas.setFont("Helvetica", 8)
    canvas.setFillColor(colors.HexColor("#666666"))
    canvas.drawString(doc.leftMargin, 20, "Whitebox Competitive Assessment")
    canvas.drawRightString(letter[0] - doc.rightMargin, 20, f"Page {canvas.getPageNumber()}")
    canvas.restoreState()


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: md_to_simple_pdf.py <input.md> <output.pdf>")
    md_path = Path(sys.argv[1])
    pdf_path = Path(sys.argv[2])
    parser = mistune.create_markdown(renderer="ast", plugins=["table"])
    ast = parser(md_path.read_text(encoding="utf-8"))
    styles = build_styles()

    doc = SimpleDocTemplate(
        str(pdf_path),
        pagesize=letter,
        leftMargin=0.8 * inch,
        rightMargin=0.8 * inch,
        topMargin=0.75 * inch,
        bottomMargin=0.6 * inch,
        title="Whitebox Competitive Assessment",
        author="GitHub Copilot",
    )
    story = build_story(ast, styles, doc.width)
    doc.build(story, onFirstPage=add_page_chrome, onLaterPages=add_page_chrome)
    print(pdf_path)


if __name__ == "__main__":
    main()
