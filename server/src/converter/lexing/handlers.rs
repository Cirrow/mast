use super::lexer::{Cursor, Token, TokenType};

pub trait Handler {
    fn trigger(&self)                 -> Option<char>;
    fn priority(&self)                -> u16;
    fn maybe(&self, c: char)          -> usize;
    fn confirm(&self, s: &str)        -> bool;
    fn requires_line_start(&self)     -> bool { false }
    fn handle(&self, cursor: &Cursor) -> Token;
}


pub struct WhitespaceHandler;
pub struct HeadingHandler;
pub struct EofHandler;
pub struct LinebreakHandler;

pub struct NewlineHandler;
pub struct BoldHandler;
pub struct ItalicHandler;
pub struct UnderlineHandler;
pub struct LinkOpenHandler;
pub struct PipeHandler;
pub struct LinkCloseHandler;
pub struct PseudoHTMLHandler;
pub struct ImageOpenHandler;
pub struct ImageCloseHandler;
pub struct QMarkHandler;
pub struct FootnoteOpenHandler;
pub struct FootnoteCloseHandler;
pub struct QuoteHandler;


impl Handler for WhitespaceHandler {
    fn trigger(&self)           -> Option<char> { Some(' ') }
    fn priority(&self)          -> u16 { 5 }
    fn maybe(&self, c: char)   -> usize { if c == ' ' { 1 } else { 0 } }
    fn confirm(&self, _s: &str) -> bool { true }

    fn handle(&self, cursor: &Cursor) -> Token {
        let remaining = &cursor.input[cursor.pos..];
        let end = cursor.pos + remaining.find(|c| c != ' ').unwrap_or(remaining.len());
        Token { t_type: TokenType::Whitespace, start: cursor.pos, end: end, ..Default::default() }
    }
}

impl Handler for BoldHandler {
    fn trigger(&self)           -> Option<char> { Some('*') }
    fn priority(&self)          -> u16 { 200 }
    fn maybe(&self, c: char)   -> usize { if c == '*' { 2 } else { 0 } }
    fn confirm(&self, s: &str) -> bool { s == "**" }

    fn handle(&self, cursor: &Cursor) -> Token {
        Token { t_type: TokenType::Bold, start: cursor.pos, end: cursor.pos + 2, ..Default::default() }
    }
}

impl Handler for ItalicHandler {
    fn trigger(&self)           -> Option<char> { Some('/') }
    fn priority(&self)          -> u16 { 500 }
    fn maybe(&self, c: char)   -> usize { if c == '/' { 2 } else { 0 } }
    fn confirm(&self, s: &str) -> bool { s == "//" }

    fn handle(&self, cursor: &Cursor) -> Token {
        Token { t_type: TokenType::Italic, start: cursor.pos, end: cursor.pos + 2, ..Default::default() }
    }
}

impl Handler for UnderlineHandler {
    fn trigger(&self)           -> Option<char> { Some('_') }
    fn priority(&self)          -> u16 { 450 }
    fn maybe(&self, c: char)   -> usize { if c == '_' { 2 } else { 0 } }
    fn confirm(&self, s: &str) -> bool { s == "__" }

    fn handle(&self, cursor: &Cursor) -> Token {
        Token { t_type: TokenType::Underline, start: cursor.pos, end: cursor.pos + 2, ..Default::default() }
    }
}

impl Handler for LinkOpenHandler {
    fn trigger(&self)           -> Option<char> { Some('[') }
    fn priority(&self)          -> u16 { 11 }
    fn maybe(&self, c: char)   -> usize { if c == '[' { 2 } else { 0 } }
    fn confirm(&self, s: &str) -> bool { s == "[[" }

    fn handle(&self, cursor: &Cursor) -> Token {
        Token { t_type: TokenType::LinkOpen, start: cursor.pos, end: cursor.pos + 2, ..Default::default() }
    }
}

impl Handler for PseudoHTMLHandler {
    fn trigger(&self)  -> Option<char> { Some('<') }
    fn priority(&self) -> u16 { 15 }
    fn maybe(&self, c: char) -> usize { if c == '<' { 2048 } else { 0 } }

    fn confirm(&self, s: &str) -> bool {
        let bytes = s.as_bytes();
        
        if bytes.len() < 3 || bytes[0] != b'<' || bytes[1] == b'<' { 
            return false; 
        }

        match bytes[1] {
            b'/' => {
                if !bytes[2].is_ascii_alphabetic() { return false; }
            }
            b if b.is_ascii_alphabetic() => {}
            _ => return false,
        }

        let (mut dq, mut sq) = (false, false);
        for &b in &bytes[1..] {
            match b {
                b'"' if !sq => dq = !dq,
                b'\'' if !dq => sq = !sq,
                b'>' if !dq && !sq => return true,
                _ => {}
            }
        }
        false
    }

    fn handle(&self, cursor: &Cursor) -> Token {
        let bytes = cursor.input[cursor.pos..].as_bytes();
        
        let (mut dq, mut sq) = (false, false);
        
        for (i, &b) in bytes.iter().enumerate().skip(1) {
            match b {
                b'"' if !sq => dq = !dq,
                b'\'' if !dq => sq = !sq,
                b'>' if !dq && !sq => {
                    return Token {
                        t_type: TokenType::PseudoHtml,
                        // using direct slicing is safe here because we know it is ascii.
                        t_detail: Some(cursor.input[cursor.pos + 1..cursor.pos + i].to_string()),
                        start: cursor.pos,
                        end: cursor.pos + i + 1,
                    };
                }
                _ => {}
            }
        }
        unreachable!("handle() called but matching closing bracket was not found")
    }
}

impl Handler for PipeHandler {
    fn trigger(&self)           -> Option<char> { Some('|') }
    fn priority(&self)          -> u16 { 50 }
    fn maybe(&self, c: char)   -> usize { if c == '|' { 1 } else { 0 } }
    fn confirm(&self, _s: &str) -> bool { true }

    fn handle(&self, cursor: &Cursor) -> Token {
        Token { t_type: TokenType::LinkOpen, start: cursor.pos, end: cursor.pos + 1, ..Default::default() }
    }
}

impl Handler for LinkCloseHandler {
    fn trigger(&self)           -> Option<char> { Some(']') }
    fn priority(&self)          -> u16 { 11 }
    fn maybe(&self, c: char)   -> usize { if c == '[' { 2 } else { 0 } }
    fn confirm(&self, s: &str) -> bool { s == "]]" }

    fn handle(&self, cursor: &Cursor) -> Token {
        Token { t_type: TokenType::LinkOpen, start: cursor.pos, end: cursor.pos + 2, ..Default::default() }
    }
}

impl Handler for FootnoteOpenHandler {
    fn trigger(&self)           -> Option<char> { Some('(') }
    fn priority(&self)          -> u16 { 7 }
    fn maybe(&self, c: char)   -> usize { if c == '(' { 2 } else { 0 } }
    fn confirm(&self, s: &str) -> bool { s == "((" }

    fn handle(&self, cursor: &Cursor) -> Token {
        Token { t_type: TokenType::FootnoteOpen, start: cursor.pos, end: cursor.pos + 2, ..Default::default() }
    }
}

impl Handler for FootnoteCloseHandler {
    fn trigger(&self)           -> Option<char> { Some(')') }
    fn priority(&self)          -> u16 { 7 }
    fn maybe(&self, c: char)   -> usize { if c == ')' { 2 } else { 0 } }
    fn confirm(&self, s: &str) -> bool { s == "))" }

    fn handle(&self, cursor: &Cursor) -> Token {
        Token { t_type: TokenType::ImageOpen, start: cursor.pos, end: cursor.pos + 2, ..Default::default() }
    }
}

impl Handler for ImageOpenHandler {
    fn trigger(&self)           -> Option<char> { Some('{') }
    fn priority(&self)          -> u16 { 130 }
    fn maybe(&self, c: char)   -> usize { if c == '{' { 2 } else { 0 } }
    fn confirm(&self, s: &str) -> bool { s == "{{" }

    fn handle(&self, cursor: &Cursor) -> Token {
        Token { t_type: TokenType::ImageOpen, start: cursor.pos, end: cursor.pos + 2, ..Default::default() }
    }
}

impl Handler for ImageCloseHandler {
    fn trigger(&self)           -> Option<char> { Some('}') }
    fn priority(&self)          -> u16 { 170 }
    fn maybe(&self, c: char)   -> usize { if c == '}' { 2 } else { 0 } }
    fn confirm(&self, s: &str) -> bool { s == "}}" }

    fn handle(&self, cursor: &Cursor) -> Token {
        Token { t_type: TokenType::ImageClose, start: cursor.pos, end: cursor.pos + 2, ..Default::default() }
    }
}


impl Handler for QMarkHandler {
    fn trigger(&self)          -> Option<char> { Some('?') }
    fn priority(&self)         -> u16 { 3 }
    fn maybe(&self, c: char)   -> usize { if c == '?' { 1 } else { 0 }}
    fn confirm(&self, _s: &str) -> bool { true }

    fn handle(&self, cursor: &Cursor) -> Token {
        Token {t_type: TokenType::QMark, start: cursor.pos, end: cursor.pos, ..Default::default()}
    }
}



impl Handler for HeadingHandler {
    fn trigger(&self)                 -> Option<char> { Some('=') }
    fn priority(&self)                -> u16 { 10 }
    fn maybe(&self, c: char)          -> usize { if c == '=' { 7 } else { 0 } }
    fn confirm(&self, s: &str) -> bool {
        let equals = s.bytes().take_while(|b| *b == b'=').count();
        equals >= 2 && (s.as_bytes().get(equals) == Some(&b' ') || s.ends_with('='))
    }
    fn requires_line_start(&self)     -> bool { true }

    fn handle(&self, cursor: &Cursor) -> Token { 
        let remaining = &cursor.input[cursor.pos..];
        let equal_count: usize = remaining.find(|c| c != '=').unwrap_or(remaining.len());
        let heading_level: String = (7 - equal_count).to_string();   
        
        Token {t_type: TokenType::Heading, t_detail: Some(heading_level), start: cursor.pos, end: cursor.pos + equal_count}
    }
}

impl Handler for QuoteHandler {
    fn trigger(&self)                 -> Option<char> { Some('>') }
    fn priority(&self)                -> u16 { 20 }
    fn maybe(&self, c: char)          -> usize { if c == '>' { 7 } else { 0 } } // max number of allowed quote level = 7
    fn confirm(&self, s: &str) -> bool {  true  }
    fn requires_line_start(&self)     -> bool { true }

    fn handle(&self, cursor: &Cursor) -> Token { 
        let remaining = &cursor.input[cursor.pos..];
        let level_count: usize = remaining.find(|c| c != '>').unwrap_or(remaining.len());   
        
        Token {t_type: TokenType::Quote, t_detail: Some((level_count).to_string()), start: cursor.pos, end: cursor.pos + level_count}
    }
}

impl Handler for EofHandler {
    fn trigger(&self)          -> Option<char> { Some('\0') }
    fn priority(&self)         -> u16 { 1 }
    fn maybe(&self, c: char)   -> usize { if c == '\0' { 1 } else { 0 }}
    fn confirm(&self, _s: &str) -> bool { true }

    fn handle(&self, cursor: &Cursor) -> Token {
        Token {t_type: TokenType::Eof, start: cursor.pos, end: cursor.pos, ..Default::default()}
    }
}

impl Handler for NewlineHandler {
    fn trigger(&self)          -> Option<char> { Some('\n') }
    fn priority(&self)         -> u16 { 100 }
    fn maybe(&self, c: char)   -> usize { if c == '\n' { 1 } else { 0 }}
    fn confirm(&self, _s: &str) -> bool { true }

    fn handle(&self, cursor: &Cursor) -> Token {
        Token {t_type: TokenType::Newline, start: cursor.pos, end: cursor.pos + 1, ..Default::default()}
    }
}

impl Handler for LinebreakHandler {
    fn trigger(&self)          -> Option<char> { Some('\\') }
    fn priority(&self)         -> u16 { 150 }
    fn maybe(&self, c: char)   -> usize { if c == '\\' { 3 } else { 0 }}
    fn confirm(&self, s: &str) -> bool { s == "\\\\ " }

    fn handle(&self, cursor: &Cursor) -> Token {
        Token {t_type: TokenType::Linebreak, start: cursor.pos, end: cursor.pos + 3, ..Default::default()}
    }
}

pub fn builtin_handlers() -> Vec<Box<dyn Handler>> {
    vec![
        Box::new(WhitespaceHandler),
        Box::new(HeadingHandler),
        Box::new(EofHandler),
        Box::new(LinebreakHandler),
        Box::new(NewlineHandler),
        Box::new(BoldHandler),
        Box::new(ItalicHandler),
        Box::new(UnderlineHandler),
        Box::new(LinkOpenHandler),
        Box::new(LinkCloseHandler),
        Box::new(PipeHandler),
        Box::new(QMarkHandler),
        Box::new(ImageCloseHandler),
        Box::new(ImageOpenHandler),
        Box::new(FootnoteOpenHandler),
        Box::new(FootnoteCloseHandler),
        Box::new(QuoteHandler),
        Box::new(PseudoHTMLHandler)
    ]
}