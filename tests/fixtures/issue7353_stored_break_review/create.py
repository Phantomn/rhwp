"""Keep the original cover and body through both authored Shift+Enter paragraphs.

Only remove trailing top-level paragraphs; do not rewrite text, styles or LineSeg.
"""
from io import BytesIO
from pathlib import Path
from xml.etree import ElementTree as ET
from zipfile import ZipFile

source = Path("samples/issue6601/36331407_side_by_side_tac_tables.hwpx")
target = Path("tests/fixtures/issue7353_stored_break_review/prefix.hwpx")
with ZipFile(source) as src, ZipFile(target, "w") as dst:
    for entry in src.infolist():
        data = src.read(entry.filename)
        if entry.filename == "Contents/section0.xml":
            for _, (prefix, uri) in ET.iterparse(BytesIO(data), events=["start-ns"]):
                ET.register_namespace(prefix, uri)
            root = ET.fromstring(data)
            paragraphs = list(root)
            ns = {"hp": "http://www.hancom.co.kr/hwpml/2011/paragraph"}
            assert len(paragraphs[8].findall(".//hp:lineBreak", ns)) == 1
            assert len(paragraphs[13].findall(".//hp:lineBreak", ns)) == 1
            for paragraph in paragraphs[15:]:
                root.remove(paragraph)
            data = ET.tostring(root, encoding="utf-8", xml_declaration=True)
        dst.writestr(entry, data)
print(target)
