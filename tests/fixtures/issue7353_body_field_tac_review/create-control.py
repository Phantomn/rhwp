"""Independent control: original cover + original field prefix, then Hancom save."""
from copy import deepcopy
from io import BytesIO
from pathlib import Path
from xml.etree import ElementTree as ET
from zipfile import ZipFile

ns = {'hp': 'http://www.hancom.co.kr/hwpml/2011/paragraph'}
target = Path('tests/fixtures/issue7353_body_field_tac_review/control-input.hwpx')
with ZipFile('samples/issue6601/36331407_side_by_side_tac_tables.hwpx') as src, ZipFile(target, 'w') as dst:
    for entry in src.infolist():
        data = src.read(entry.filename)
        if entry.filename == 'Contents/section0.xml':
            for _, (prefix, uri) in ET.iterparse(BytesIO(data), events=['start-ns']):
                ET.register_namespace(prefix, uri)
            root = ET.fromstring(data)
            paragraphs = list(root)
            field = deepcopy(paragraphs[2].find('.//hp:ctrl[hp:fieldBegin]', ns))
            assert field is not None
            host = paragraphs[1].find('hp:run', ns)
            assert host.find('hp:tbl', ns) is not None
            host.insert(0, field)
            for p in paragraphs[2:]: root.remove(p)
            for parent in root.iter():
                for child in list(parent):
                    if child.tag.endswith('}linesegarray'): parent.remove(child)
            data = ET.tostring(root, encoding='utf-8', xml_declaration=True)
        dst.writestr(entry, data)
print(target)
