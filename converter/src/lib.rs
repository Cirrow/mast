pub mod lexing;
pub mod parser;
pub mod renderer;
pub struct Result {
    pub html: String,
}

pub fn render_page(input: &str) -> Result {
    let tokens = lexing::Lexer::new().tokenise(input);
    let ast = parser::Parser::new().nodeify(&tokens);
    let html = renderer::Renderer::new(input).render(&ast);
    Result { html }
}
