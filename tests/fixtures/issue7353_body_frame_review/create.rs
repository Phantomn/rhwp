use rhwp::model::{paragraph::{Paragraph,CharShapeRef},style::{ParaShape,LineSpacingType},page::PageDef};
fn p(text:&str)->Paragraph { Paragraph { text:text.into(),char_count:text.encode_utf16().count() as u32+1,char_offsets:(0..text.len() as u32).collect(),char_shapes:vec![CharShapeRef{start_pos:0,char_shape_id:0}],..Default::default() } }
fn main(){
 let mut d=rhwp::parse_document(&std::fs::read("tests/fixtures/issue7353_picture_space_review/picture-saved.hwp").unwrap()).unwrap();
 d.doc_info.para_shapes=vec![ParaShape{line_spacing_type:LineSpacingType::Fixed,line_spacing:3600,..Default::default()}];
 d.doc_info.char_shapes[0].base_size=1200;
 let page=PageDef{width:18000,height:30000,margin_left:1500,margin_right:1500,margin_top:1500,margin_header:0,margin_bottom:19500,margin_footer:0,..Default::default()};
 d.sections[0].section_def.page_def=page;
 d.sections[0].section_def.flags=0;
 d.sections[0].section_def.page_border_fill=Default::default();
 d.sections[0].section_def.extra_page_border_fills.clear();
 d.sections[0].paragraphs=vec![p("BEFORE"),p("ALPHA\nBRAVO\n\nCHARLIE\nDELTA\nECHO"),p("AFTER")];
 std::fs::write("output/7353/r19/body-reset/portrait-input.hwpx",rhwp::serializer::serialize_hwpx(&d).unwrap()).unwrap();
}
