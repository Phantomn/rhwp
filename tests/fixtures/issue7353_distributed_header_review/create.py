"""표지·본문 앞 두 표와 대상 표의 앞뒤 문단만 남긴다. 각 문단은 변경하지 않는다."""
from io import BytesIO
from pathlib import Path
from xml.etree import ElementTree as ET
from zipfile import ZipFile
import argparse

source = Path("samples/issue6601/36331407_side_by_side_tac_tables.hwpx")
parser = argparse.ArgumentParser()
parser.add_argument("--prefix", action="store_true", help="진단용 앞25문단 전체 발췌")
prefix = parser.parse_args().prefix
target = Path("tests/fixtures/issue7353_distributed_header_review") / ("prefix.hwpx" if prefix else "table.hwpx")
with ZipFile(source) as src, ZipFile(target, "w") as dst:
    for entry in src.infolist():
        data = src.read(entry.filename)
        if entry.filename == "Contents/section0.xml":
            for _, (namespace_prefix, uri) in ET.iterparse(BytesIO(data), events=["start-ns"]):
                ET.register_namespace(namespace_prefix, uri)
            root = ET.fromstring(data)
            paragraphs = list(root)
            assert len(paragraphs) > 25
            for i, paragraph in enumerate(paragraphs):
                if (prefix and i >= 25) or (not prefix and i not in [0, 1, 2, 3, 22, 23, 24]):
                    root.remove(paragraph)
            data = ET.tostring(root, encoding="utf-8", xml_declaration=True)
        dst.writestr(entry, data)
print(target)
