use docx_rs::Docx;

pub fn get_global_styles(docx: &Docx) {
    println!("{:?}", docx.styles)
}
