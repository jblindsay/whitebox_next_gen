from pathlib import Path
import re

from reportlab.lib import colors
from reportlab.lib.pagesizes import LETTER
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import inch
from reportlab.platypus import ListFlowable, ListItem, Paragraph, SimpleDocTemplate, Spacer

ROOT = Path(__file__).resolve().parents[1]
MD_PATH = ROOT / "LICENSING_STRATEGY.md"
PDF_PATH = ROOT / "LICENSING_STRATEGY.pdf"


def build_styles():
    styles = getSampleStyleSheet()
    body = ParagraphStyle(
        "Body",
        parent=styles["BodyText"],
        fontName="Helvetica",
        fontSize=10.5,
        leading=14,
        textColor=colors.HexColor("#1f2937"),
        spaceAfter=7,
    )
    return {
        "body": body,
        "body_small": ParagraphStyle(
            "BodySmall",
            parent=body,
            fontSize=9.25,
            leading=12,
            textColor=colors.HexColor("#4b5563"),
        ),
        "hero_kicker": ParagraphStyle(
            "HeroKicker",
            parent=styles["BodyText"],
            fontName="Helvetica-Bold",
            fontSize=9,
            leading=11,
            textColor=colors.HexColor("#0f6cbd"),
            spaceAfter=6,
        ),
        "hero_title": ParagraphStyle(
            "HeroTitle",
            parent=styles["Title"],
            fontName="Helvetica-Bold",
            fontSize=24,
            leading=28,
            textColor=colors.HexColor("#0f172a"),
            spaceAfter=10,
        ),
        "hero_subtitle": ParagraphStyle(
            "HeroSubtitle",
            parent=body,
            fontSize=12,
            leading=16,
            textColor=colors.HexColor("#475569"),
            spaceAfter=16,
        ),
        "h1": ParagraphStyle(
            "H1",
            parent=styles["Heading1"],
            fontName="Helvetica-Bold",
            fontSize=16,
            leading=20,
            textColor=colors.HexColor("#0f172a"),
            spaceBefore=14,
            spaceAfter=8,
        ),
        "h2": ParagraphStyle(
            "H2",
            parent=styles["Heading2"],
            fontName="Helvetica-Bold",
            fontSize=12.5,
            leading=16,
            textColor=colors.HexColor("#111827"),
            spaceBefore=10,
            spaceAfter=6,
        ),
        "code": ParagraphStyle(
            "Code",
            parent=body,
            fontName="Courier",
            fontSize=9,
            leading=11,
            leftIndent=14,
            backColor=colors.HexColor("#f3f4f6"),
            borderColor=colors.HexColor("#d1d5db"),
            borderWidth=0.5,
            borderPadding=6,
            spaceBefore=4,
            spaceAfter=8,
        ),
        "summary": ParagraphStyle(
            "Summary",
            parent=body,
            fontName="Helvetica",
            fontSize=10.5,
            leading=14,
            textColor=colors.HexColor("#102a43"),
            backColor=colors.HexColor("#eef6ff"),
            borderColor=colors.HexColor("#bfdcff"),
            borderWidth=0.8,
            borderPadding=10,
            spaceBefore=6,
            spaceAfter=12,
        ),
    }


def escape_inline(text: str) -> str:
    text = text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
    text = re.sub(r"`([^`]+)`", r'<font name="Courier">\1</font>', text)
    text = re.sub(r"\*\*([^*]+)\*\*", r"<b>\1</b>", text)
    return text


def add_page_number(canvas, doc):
    canvas.saveState()
    canvas.setStrokeColor(colors.HexColor("#cbd5e1"))
    canvas.setLineWidth(0.5)
    canvas.line(doc.leftMargin, 0.55 * inch, LETTER[0] - doc.rightMargin, 0.55 * inch)
    canvas.setFont("Helvetica", 9)
    canvas.setFillColor(colors.HexColor("#64748b"))
    canvas.drawString(doc.leftMargin, 0.36 * inch, "Licensing Strategy for whitebox_next_gen")
    canvas.drawRightString(LETTER[0] - doc.rightMargin, 0.36 * inch, f"Page {doc.page}")
    canvas.restoreState()


def render_story(lines, styles):
    story = []
    story.append(Paragraph("Strategy Note", styles["hero_kicker"]))
    story.append(Paragraph("Licensing Strategy for whitebox_next_gen", styles["hero_title"]))
    story.append(
        Paragraph(
            "Assessment of the current placeholder licensing model, lessons from the legacy Whitebox implementation, and a practical roadmap toward a production-ready commercial licensing architecture.",
            styles["hero_subtitle"],
        )
    )
    story.append(
        Paragraph(
            "<b>Bottom line:</b> the current licensing model is incomplete, but its core architecture is sound. The right path is to preserve centralized runtime enforcement and replace the simple tier gate with a real entitlement system based on signed local licenses and optional online activation and leasing services.",
            styles["summary"],
        )
    )

    if lines and lines[0].startswith("# "):
        lines = lines[1:]

    in_code = False
    code_lines = []
    bullet_buffer = []
    num_buffer = []
    para_buffer = []

    def flush_para():
        nonlocal para_buffer
        if para_buffer:
            joined = " ".join(part.strip() for part in para_buffer if part.strip())
            if joined:
                story.append(Paragraph(escape_inline(joined), styles["body"]))
            para_buffer = []

    def flush_bullets():
        nonlocal bullet_buffer
        if bullet_buffer:
            items = [ListItem(Paragraph(escape_inline(item), styles["body"]), leftIndent=0) for item in bullet_buffer]
            story.append(ListFlowable(items, bulletType="bullet", leftIndent=18))
            story.append(Spacer(1, 4))
            bullet_buffer = []

    def flush_nums():
        nonlocal num_buffer
        if num_buffer:
            items = [ListItem(Paragraph(escape_inline(item), styles["body"]), value=index + 1) for index, item in enumerate(num_buffer)]
            story.append(ListFlowable(items, bulletType="1", leftIndent=18))
            story.append(Spacer(1, 4))
            num_buffer = []

    def flush_code():
        nonlocal code_lines
        if code_lines:
            story.append(Paragraph("<br/>".join(escape_inline(line) for line in code_lines), styles["code"]))
            code_lines = []

    for raw_line in lines:
        stripped = raw_line.strip()

        if stripped.startswith("```"):
            flush_para()
            flush_bullets()
            flush_nums()
            if in_code:
                flush_code()
                in_code = False
            else:
                in_code = True
            continue

        if in_code:
            code_lines.append(raw_line.rstrip("\n"))
            continue

        if not stripped:
            flush_para()
            flush_bullets()
            flush_nums()
            continue

        if stripped.startswith("## "):
            flush_para()
            flush_bullets()
            flush_nums()
            story.append(Spacer(1, 6))
            story.append(Paragraph(escape_inline(stripped[3:]), styles["h1"]))
            continue

        if stripped.startswith("### "):
            flush_para()
            flush_bullets()
            flush_nums()
            story.append(Paragraph(escape_inline(stripped[4:]), styles["h2"]))
            continue

        if re.match(r"-\s+", stripped):
            flush_para()
            flush_nums()
            bullet_buffer.append(re.sub(r"^-\s+", "", stripped))
            continue

        if re.match(r"\d+\.\s+", stripped):
            flush_para()
            flush_bullets()
            num_buffer.append(re.sub(r"^\d+\.\s+", "", stripped))
            continue

        para_buffer.append(stripped)

    flush_para()
    flush_bullets()
    flush_nums()
    flush_code()
    return story


def main():
    lines = MD_PATH.read_text(encoding="utf-8").splitlines()
    styles = build_styles()
    story = render_story(lines, styles)
    doc = SimpleDocTemplate(
        str(PDF_PATH),
        pagesize=LETTER,
        leftMargin=0.72 * inch,
        rightMargin=0.72 * inch,
        topMargin=0.7 * inch,
        bottomMargin=0.78 * inch,
        title="Licensing Strategy for whitebox_next_gen",
        author="GitHub Copilot",
        subject="Licensing design strategy",
    )
    doc.build(story, onFirstPage=add_page_number, onLaterPages=add_page_number)
    print(PDF_PATH)
    print(f"{PDF_PATH.stat().st_size} bytes")


if __name__ == "__main__":
    main()
