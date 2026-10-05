use rhwp::wasm_api::HwpDocument;
use std::{fs, path::Path};
const TEXT: &str = "1) 가나다라마바사아자차카타파하 가나다라마바사아자차카타파하 가나다라마바사아자차카타파하 가나다라마바사아자차카타파하 가나다라마바사아자차카타파하";
fn open(p: &str) -> HwpDocument {
    HwpDocument::from_bytes(&fs::read(p).unwrap()).unwrap()
}
fn save(doc: &HwpDocument, name: &str, out: &Path) {
    let p = out.join(name);
    fs::create_dir_all(&p).unwrap();
    fs::write(
        p.join(format!("{name}.hwp")),
        doc.export_hwp_native().unwrap(),
    )
    .unwrap();
    for page in 0..doc.page_count() {
        fs::write(
            p.join(format!("page_{:03}.svg", page + 1)),
            doc.render_page_svg_native(page as u32).unwrap(),
        )
        .unwrap();
        fs::write(
            p.join(format!("page_{:03}.json", page + 1)),
            doc.get_page_layer_tree_native(page as u32).unwrap(),
        )
        .unwrap();
    }
    println!("{name}: {} pages", doc.page_count());
}
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let out = Path::new(&out);
    for (name, indent) in [
        ("typed-flat", 0),
        ("typed-first", 3000),
        ("typed-hanging", -3000),
    ] {
        let mut d = HwpDocument::create_empty();
        d.create_blank_document_native().unwrap();
        d.insert_text_native(0, 0, 0, TEXT).unwrap();
        d.apply_para_format_native(0, 0, &format!("{{\"indent\":{indent}}}"))
            .unwrap();
        save(&d, name, out);
    }
    let mut d = open("samples/issue6190/center_align_first_line_indent.hwp");
    d.insert_text_native(0, 4, 7, "가").unwrap();
    d.insert_text_native(0, 7, 0, "가").unwrap();
    save(&d, "6190-edited", out);
    let mut d = open("samples/biz_plan.hwp");
    d.insert_text_native(0, 51, 67, "가").unwrap();
    save(&d, "biz-edited", out);
}
