pub mod lexing;
pub mod parser;
pub mod renderer;

pub struct metadata {
}

pub struct Result {
    pub html: String,
    pub meta: metadata
}
                                                                            
pub fn render_page(input: &str) -> Result {
    let tokens = lexing::Lexer::new().tokenise(input);
    let ast = parser::Parser::new().nodeify(&tokens);
    let html = renderer::Renderer.render(&ast);
    Result { html, meta: metadata {} }
}