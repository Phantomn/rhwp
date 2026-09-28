"""원본 본문 누름틀과 마지막 빈 줄을 보존하고, 가시 후속 문단을 추가한다."""
from copy import deepcopy
from io import BytesIO
from pathlib import Path
from xml.etree import ElementTree as ET
from zipfile import ZipFile
import argparse

parser = argparse.ArgumentParser()
parser.add_argument("--bounded", action="store_true")
parser.add_argument("--before", action="store_true")
args = parser.parse_args()

source = Path("samples/issue6601/36331407_side_by_side_tac_tables.hwpx")
target = Path("tests/fixtures/issue7353_body_end_review/after.hwpx")
if args.bounded:
    target = target.with_name("bounded.hwpx")
elif args.before:
    target = target.with_name("before.hwpx")
hp = "http://www.hancom.co.kr/hwpml/2011/paragraph"
with ZipFile(source) as src, ZipFile(target, "w") as dst:
    for entry in src.infolist():
        data = src.read(entry.filename)
        if entry.filename == "Contents/section0.xml":
            for _, (prefix, uri) in ET.iterparse(BytesIO(data), events=["start-ns"]):
                ET.register_namespace(prefix, uri)
            root = ET.fromstring(data)
            # Copy formatting only. Do NOT fabricate saved line coordinates.
            after = deepcopy(list(root)[33])
            after.set("id", "2147483647")
            after.remove(after.find(f"{{{hp}}}linesegarray"))
            after.find(f".//{{{hp}}}t").text = "후속 문단: 누름틀 종료 빈 줄 다음"
            root.append(after)
            if args.bounded:
                # Keep intact source paragraphs; exclude unrelated middle pages.
                for i, paragraph in enumerate(list(root)):
                    if i not in (0, 1, 2, 3, 32, 33, 34, 35):
                        root.remove(paragraph)
            elif args.before:
                # Diagnostic prefix accepted by the previous V2 implementation.
                for paragraph in list(root)[34:]:
                    root.remove(paragraph)
            data = ET.tostring(root, encoding="utf-8", xml_declaration=True)
        dst.writestr(entry, data)
print(target)
