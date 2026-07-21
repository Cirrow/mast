pub mod lexing;
pub mod parser;
pub mod renderer;
pub struct PageResult {
    pub html: String,
    pub toc: String,
}

pub fn render_page(input: &str) -> PageResult {
    let tokens = lexing::Lexer::new().tokenise(input);
    let ast = parser::Parser::new().nodeify(&tokens);
    let mut renderer = renderer::Renderer::new(input);
    let html = renderer.render(&ast);
    let toc = renderer.build_toc();
    PageResult { html, toc }
}
