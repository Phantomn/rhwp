use rhwp::model::{control::Control,paragraph::Paragraph};
fn fresh(p:&mut Paragraph){p.line_segs.clear();for c in &mut p.controls{if let Control::Table(t)=c{for cell in &mut t.cells{for p in &mut cell.paragraphs{fresh(p);}}}}}
fn main(){
 let mut d=rhwp::parse_document(&std::fs::read("tests/fixtures/issue7353_tac_space_review/space-input.hwpx").unwrap()).unwrap();
 let original=rhwp::parse_document(&std::fs::read("tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp").unwrap()).unwrap();
 let Control::Table(w)=&original.sections[0].paragraphs[5].controls[0] else {panic!()};
 let mut carrier=w.cells[0].paragraphs[69].clone();
 // Restore original IDs; append the base fixture's plain carrier style.
 let plain=d.doc_info.para_shapes[d.sections[0].paragraphs[0].para_shape_id as usize].clone();
 d.doc_info=original.doc_info.clone();
 let id=d.doc_info.para_shapes.len() as u16;d.doc_info.para_shapes.push(plain);
 d.sections[0].paragraphs[0].para_shape_id=id;d.sections[0].paragraphs[1].para_shape_id=id;
 if std::env::args().any(|a|a=="--simple") {
  let Control::Table(child)=&mut carrier.controls[0] else {panic!()};
  let mut cell=child.cells[0].clone();
  cell.col=0;cell.row=0;cell.col_span=1;cell.row_span=1;
  cell.width=child.common.width;cell.height=child.common.height;
  let mut label=d.sections[0].paragraphs[1].clone();
  label.text="TABLE ON SECOND LINE".into();label.char_count=21;label.char_offsets=(0..20).collect();
  cell.paragraphs=vec![label];
  child.row_count=1;child.col_count=1;child.row_sizes=vec![child.common.height as i16];
  child.cells=vec![cell];child.cell_grid.clear();child.zones.clear();
 }
 let head=&mut d.sections[0].paragraphs[0];
 let Control::Table(t)=head.controls.iter_mut().find(|c|matches!(c,Control::Table(_))).unwrap() else {panic!()};
 t.common.height=26000;t.row_sizes=vec![26000];t.cells[0].height=26000;
 t.cells[0].paragraphs=vec![carrier];
 fresh(head);fresh(&mut d.sections[0].paragraphs[1]);
 let name=if std::env::args().any(|a|a=="--simple"){"space-row-input.hwpx"}else{"carrier-input.hwpx"};
 std::fs::write(format!("output/7353/r19/tac-next/{name}"),rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap()).unwrap();
}
