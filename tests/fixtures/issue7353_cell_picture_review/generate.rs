fn main() {
    let args: Vec<_> = std::env::args().collect();
    let input = args.get(1).map(String::as_str).unwrap_or("samples/issue4090/156492236_규제샌드박스_min.hwpx");
    let mut d = rhwp::parse_document(&std::fs::read(input).unwrap()).unwrap();
    if args.len() == 1 {
        d.sections[0].paragraphs = d.sections[0].paragraphs[60..64].to_vec();
        d.sections[0].paragraphs[0].column_type = rhwp::model::paragraph::ColumnBreakType::None;
        d.sections[0].paragraphs[0].raw_break_type = 0;
        std::fs::write("output/7353/r19/cell-floating-picture/picture-input.hwpx", rhwp::serializer::serialize_hwpx(&d).unwrap()).unwrap();
        // Explicit visual control: change only the sanitized white image bytes.
        // Keep source size, crop, offsets, host, cell and paragraph properties.
        let (w,h)=(160u32,120u32);
        let size=54+w*h*3;
        let mut bmp=Vec::new();
        bmp.extend_from_slice(b"BM"); bmp.extend_from_slice(&size.to_le_bytes());
        bmp.extend_from_slice(&[0;4]); bmp.extend_from_slice(&54u32.to_le_bytes());
        bmp.extend_from_slice(&40u32.to_le_bytes()); bmp.extend_from_slice(&w.to_le_bytes());
        bmp.extend_from_slice(&h.to_le_bytes()); bmp.extend_from_slice(&1u16.to_le_bytes());
        bmp.extend_from_slice(&24u16.to_le_bytes()); bmp.extend_from_slice(&[0;24]);
        for y in 0..h { for x in 0..w {
            let bgr=if x<4 || y<4 || x>=w-4 || y>=h-4 { [0,0,0] }
                else if x<w/2 && y<h/2 { [30,160,240] }
                else if x<w/2 { [220,100,30] }
                else if y<h/2 { [60,190,60] } else { [180,40,180] };
            bmp.extend_from_slice(&bgr);
        }}
        d.bin_data_content[5].data=rhwp::model::bin_data::BinDataBytes::from_shared(bmp);
        std::fs::write("output/7353/r19/cell-floating-picture/visible-input.hwpx", rhwp::serializer::serialize_hwpx(&d).unwrap()).unwrap();
    }
    for (pi,p) in d.sections[0].paragraphs.iter().enumerate() {
        println!("p{pi}: text={:?} lines={:?}", p.text, p.line_segs);
        for c in &p.controls {
            if let rhwp::model::control::Control::Table(t) = c {
                println!("table={:?}", t.common);
                for cell in &t.cells {
                    for p in &cell.paragraphs {
                        if p.controls.iter().any(|c| matches!(c,rhwp::model::control::Control::Picture(_))) {
                            println!("picture host={p:?}");
                            println!("style={:?}", d.doc_info.para_shapes[p.para_shape_id as usize]);
                        }
                    }
                }
            }
        }
    }
}
