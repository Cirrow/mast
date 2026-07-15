use super::handlers::{Handler, builtin_handlers};
use std::cmp::Reverse;
use std::collections::HashSet;

#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
pub enum TokenType {
    #[default]
    Text,
    Hr,

    Bold,
    Italic,
    Underline,

    Heading,

    LinkOpen,
    Pipe,
    LinkClose,
    ImageOpen,
    ImageClose,
    QMark, // ?

    FootnoteOpen,
    FootnoteClose,

    Quote,

    PseudoHtml,

    Linebreak,
    Newline,
    Whitespace,

    Eof,
}

#[derive(Debug, Default, Clone)]
pub struct Token {
    pub t_type: TokenType,
    pub t_detail: Option<String>,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug)]
pub struct Cursor<'a> {
    pub input: &'a str,
    pub pos: usize,
}

pub struct Lexer {
    handlers: Vec<Box<dyn Handler>>,
    special_chars: HashSet<u8>,
}

impl Lexer {
    pub fn new() -> Self {
        let mut handlers = builtin_handlers();
        handlers.sort_by_key(|h| Reverse(h.priority()));

        let special_chars = Self::build_special_chars(&handlers);

        Lexer {
            handlers,
            special_chars,
        }
    }

    pub fn tokenise(&self, input: &str) -> Vec<Token> {
        let mut cursor = Cursor { input, pos: 0 };
        let mut tokens = Vec::new();
        let mut start_of_line = true;

        while cursor.pos < cursor.input.len() {
            let remaining = &cursor.input[cursor.pos..];

            if let Some(offset) = remaining
                .bytes()
                .position(|b| self.special_chars.contains(&b))
            {
                if offset > 0 {
                    tokens.push(Token {
                        start: cursor.pos,
                        end: cursor.pos + offset,
                        ..Default::default()
                    });
                    cursor.pos += offset;
                } else {
                    let mut handled = false;
                    let byte = input.as_bytes()[cursor.pos];

                    for handler in &self.handlers {
                        let Some(trigger) = handler.trigger() else {
                            continue;
                        };
                        if byte != trigger as u8 {
                            continue;
                        }

                        let lookahead = handler.maybe(byte as char);
                        if lookahead == 0 {
                            continue;
                        }

                        let slice_end = std::cmp::min(cursor.pos + lookahead, input.len());

                        if !handler.confirm(&input[cursor.pos..slice_end]) {
                            continue;
                        }
                        if handler.requires_line_start() && !start_of_line {
                            continue;
                        }

                        let token: Token = handler.handle(&cursor);
                        cursor.pos = token.end;

                        if matches!(token.t_type, TokenType::Linebreak | TokenType::Newline) {
                            start_of_line = true;
                        } else {
                            start_of_line = false;
                        }

                        tokens.push(token);
                        handled = true;
                        break;
                    }

                    if !handled {
                        start_of_line = false;
                        // collect consecutive same byte special chars
                        let mut end = cursor.pos + 1;
                        while end < input.len() && input.as_bytes()[end] == byte {
                            end += 1;
                        }
                        tokens.push(Token {
                            t_type: TokenType::Text,
                            t_detail: Some(input[cursor.pos..end].to_string()),
                            start: cursor.pos,
                            end,
                        });
                        cursor.pos = end;
                    }
                }
            } else {
                tokens.push(Token {
                    start: cursor.pos,
                    end: input.len(),
                    ..Default::default()
                });

                break;
            }
        }

        tokens.push(Token {
            t_type: TokenType::Eof,
            start: input.len(),
            end: input.len(),
            ..Default::default()
        });

        tokens
    }

    fn build_special_chars(handlers: &[Box<dyn Handler>]) -> HashSet<u8> {
        let mut chars = HashSet::new();

        for handler in handlers {
            if let Some(c) = handler.trigger() {
                chars.insert(c as u8);
            }
        }

        chars
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] // cargo test lex_debug -- --nocapture
    fn lex_debug() {
        let lexer = Lexer::new();
        let input = "====== heading1 ======";
        let tokens = lexer.tokenise(&input);
        for t in &tokens {
            println!("{:?}", t);
        }
        println!("{}", &input);
    }
}
