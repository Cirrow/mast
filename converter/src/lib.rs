pub mod lexing;
pub mod parser;
pub mod renderer;
pub struct PageResult {
    pub html: String,
    pub toc: String,
}

pub fn render_page(input: &str) -> PageResult {
    eprintln!("[conv] lexing start");
    let tokens = lexing::Lexer::new().tokenise(input);
    eprintln!("[conv] lexing done ({} tokens)", tokens.len());
    let ast = parser::Parser::new().nodeify(&tokens);
    eprintln!("[conv] parsing done ({} nodes)", ast.len());
    let mut renderer = renderer::Renderer::new(input);
    let html = renderer.render(&ast);
    eprintln!("[conv] rendering done");
    let toc = renderer.build_toc();
    eprintln!("[conv] toc done");
    PageResult { html, toc }
} 
