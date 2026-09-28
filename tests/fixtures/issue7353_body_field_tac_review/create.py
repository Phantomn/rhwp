"""Keep original cover and the first two body tables, including field begin."""
import argparse
from io import BytesIO
from pathlib import Path
from xml.etree import ElementTree as ET
from zipfile import ZipFile

parser = argparse.ArgumentParser()
parser.add_argument("--output", default="tests/fixtures/issue7353_body_field_tac_review/prefix.hwpx")
target = Path(parser.parse_args().output)
source = Path("samples/issue6601/36331407_side_by_side_tac_tables.hwpx")
with ZipFile(source) as src, ZipFile(target, "w") as dst:
    for entry in src.infolist():
        data = src.read(entry.filename)
        if entry.filename == "Contents/section0.xml":
            for _, (prefix, uri) in ET.iterparse(BytesIO(data), events=["start-ns"]):
                ET.register_namespace(prefix, uri)
            root = ET.fromstring(data)
            paragraphs = list(root)
            assert paragraphs[2].attrib["pageBreak"] == "1"
            for p in paragraphs[4:]:
                root.remove(p)
            data = ET.tostring(root, encoding="utf-8", xml_declaration=True)
        dst.writestr(entry, data)
print(target)
