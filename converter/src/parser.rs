use crate::lexing::{Token, TokenType};

#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
pub enum NodeType {
    #[default] Text,
    Paragraph,
    Hr,
    Bold,
    Italic,
    Underline,
    Heading,
    Quote,
    Link,
    Image,
    Pipe,
    Footnote,
    PseudoHtml,
    Linebreak,
    Newline,
    Whitespace,
    Eof,
}

#[derive(Debug, Default)]
pub struct Node {
    pub n_type: NodeType,
    pub children: Vec<Node>,
    pub n_detail: Option<String>,
    pub start: usize,
    pub end: usize
}

type Stack = Vec<(NodeType, Vec<Node>, Option<String>)>;

pub struct Parser;

impl Parser {
    pub fn new() -> Self {
        Parser
    }

    pub fn nodeify(&self, tokens: &[Token]) -> Vec<Node> {
        let mut blocks: Vec<Node> = Vec::new();
        let mut i: usize = 0;

        while i < tokens.len() {
            let t: &Token = &tokens[i];

            match t.t_type {
                TokenType::Eof => break,
                TokenType::Newline => {
                    i += 1;
                }

                TokenType::Heading => {
                    let end: usize = self.line_end(tokens, i + 1);
                    let content: Vec<Node> = self.inline_parse(&tokens[i + 1..end]);
                    blocks.push(Node {
                        n_type: NodeType::Heading,
                        n_detail: t.t_detail.clone(),
                        children: content,
                        start: t.start,
                        end: t.end
                    });
                    i = end + 1;
                }

                TokenType::Hr => {
                    blocks.push(Node {
                        n_type: NodeType::Hr,
                        start: t.start,
                        end: t.end,
                        ..Default::default()
                    });
                    let end = self.line_end(tokens, i + 1);
                    i = end + 1;
                }

                TokenType::Quote => {
                    let end: usize = self.line_end(tokens, i + 1);
                    let content: Vec<Node> = self.inline_parse(&tokens[i + 1..end]);
                    blocks.push(Node {
                        n_type: NodeType::Quote,
                        n_detail: t.t_detail.clone(),
                        children: content,
                        start: t.start,
                        end: t.end
                    });
                    i = end + 1;
                }

                _ => {
                    let end: usize = self.para_end(tokens, i);
                    let content: Vec<Node> = self.inline_parse(&tokens[i..end]);
                    if !content.is_empty() {
                        blocks.push(Node {
                            n_type: NodeType::Paragraph,
                            children: content,
                            start: tokens[i].start,
                            ..Default::default()
                        });
                    }
                    i = end;
                }
            }
        }

        blocks
    }

    fn inline_parse(&self, tokens: &[Token]) -> Vec<Node> {
        let mut stack: Stack = Vec::new();
        let mut result: Vec<Node> = Vec::new();
        let mut i: usize = 0;

        let count_in_stack = |stack: &Stack, nt: NodeType| -> usize {
            stack.iter().filter(|(n, _, _)| *n == nt).count()
        };

        while i < tokens.len() {
            let t: &Token = &tokens[i];

            match t.t_type {

                TokenType::Text | TokenType::Whitespace | TokenType::Linebreak | TokenType::Newline => {
                    let node: Node = Self::leaf(t);
                    Self::emit(&mut stack, &mut result, node);
                    i += 1;
                }

                TokenType::Bold | TokenType::Italic | TokenType::Underline => {
                    let nt: NodeType = Self::to_nt(t.t_type);
                    let count: usize = count_in_stack(&stack, nt);
                    if count % 2 == 0 {
                        if i + 1 < tokens.len() && tokens[i + 1].t_type == t.t_type {
                            Self::emit(&mut stack, &mut result, Node {
                                start: t.start,
                                end: tokens[i + 1].end,
                                ..Default::default()
                            });
                            i += 2;
                            continue;
                        }
                        stack.push((nt, vec![], None));

                    } else {
                        let top_matches: bool = stack
                            .last()
                            .map(|(top_nt, _, _)| *top_nt == nt)
                            .unwrap_or(false);

                        if top_matches {
                            let (_, children, _) = stack.pop().unwrap();
                            Self::emit(
                                &mut stack,
                                &mut result,
                                Node { n_type: nt, children, start: t.start, ..Default::default() },
                            );
                        } else {
                            panic!(
                                "Overlapping formatting: {:?} opened inside {:?} at pos {}",
                                nt,
                                stack.last().map(|(n, _, _)| n).unwrap(),
                                t.start
                            );
                        }
                    }
                    i += 1;
                }

                TokenType::LinkOpen => {
                    stack.push((NodeType::Link, vec![], None));
                    i += 1;
                }

                TokenType::Pipe => {
                    let inside_link_or_image: bool = stack
                        .last()
                        .map(|(nt, _, _)| *nt == NodeType::Link || *nt == NodeType::Image)
                        .unwrap_or(false);

                    if inside_link_or_image {
                        Self::emit(
                            &mut stack,
                            &mut result,
                            Node { n_type: NodeType::Pipe, ..Default::default() },
                        );

                    } else {
                        Self::emit(
                            &mut stack,
                            &mut result,
                            Node { n_type: NodeType::Text, n_detail: Some("|".to_string()), ..Default::default() },
                        );
                    }
                    i += 1;
                }

                TokenType::LinkClose => {

                    let top_is_link = stack
                        .last()
                        .map(|(nt, _, _)| *nt == NodeType::Link)
                        .unwrap_or(false);
                    
                    if top_is_link {
                        let (_, mut children, _) = stack.pop().unwrap();
                        let pipe_pos = children.iter().position(|n| n.n_type == NodeType::Pipe);
                        let target = match pipe_pos {
                            Some(pos) => {
                                let display = children.drain(pos + 1..).collect();
                                let text = children[..pos]
                                    .iter()
                                    .filter_map(|n| n.n_detail.as_deref())
                                    .collect::<Vec<_>>()
                                    .concat();
                                children = display;
                                Some(text)
                            }
                            None => {
                                let text = children
                                    .iter()
                                    .filter_map(|n| n.n_detail.as_deref())
                                    .collect::<Vec<_>>()
                                    .concat();
                                Some(text)
                            }
                        };
                        Self::emit(
                            &mut stack,
                            &mut result,
                            Node {  n_type: NodeType::Link, children, n_detail: target, start: t.start, end: t.end },
                        );
                    } else {
                        panic!("Unmatched LinkClose ]] at pos {}", t.start);
                    }
                    i += 1;
                }

                TokenType::ImageOpen => {
                    stack.push((NodeType::Image, vec![], None));
                    i += 1;
                }

                TokenType::ImageClose => {
                    let top_is_image = stack
                        .last()
                        .map(|(nt, _, _)| *nt == NodeType::Image)
                        .unwrap_or(false);
                    if top_is_image {
                        let (_, mut children, _) = stack.pop().unwrap();
                        let pipe_pos = children.iter().position(|n| n.n_type == NodeType::Pipe);
                        let target = match pipe_pos {
                            Some(pos) => {
                                let display = children.drain(pos + 1..).collect();
                                let text = children[..pos]
                                    .iter()
                                    .filter_map(|n| n.n_detail.as_deref())
                                    .collect::<Vec<_>>()
                                    .concat();
                                children = display;
                                Some(text)
                            }
                            None => {
                                let text = children
                                    .iter()
                                    .filter_map(|n| n.n_detail.as_deref())
                                    .collect::<Vec<_>>()
                                    .concat();
                                Some(text)
                            }
                        };
                        Self::emit(
                            &mut stack,
                            &mut result,
                            Node {
                                n_type: NodeType::Image,
                                children,
                                n_detail: target,
                                start: t.start,
                                end: t.end
                            },
                        );
                    } else {
                        panic!("Unmatched ImageClose }} at pos {}", t.start);
                    }
                    i += 1;
                }

                TokenType::FootnoteOpen => {
                    stack.push((NodeType::Footnote, vec![], None));
                    i += 1;
                }

                TokenType::FootnoteClose => {
                    let top_is_footnote = stack
                        .last()
                        .map(|(nt, _, _)| *nt == NodeType::Footnote)
                        .unwrap_or(false);
                    if top_is_footnote {
                        let (_, children, _) = stack.pop().unwrap();
                        Self::emit(
                            &mut stack,
                            &mut result,
                            Node {
                                n_type: NodeType::Footnote,
                                children,
                                start: t.start,
                                ..Default::default()
                            },
                        );
                    } else {
                        panic!("Unmatched FootnoteClose )) at pos {}", t.start);
                    }
                    i += 1;
                }

                TokenType::PseudoHtml => {
                    let detail = t.t_detail.as_deref().unwrap_or("");
                    if detail.starts_with('/') {
                        let close_tag =
                            detail[1..].split_whitespace().next().unwrap_or("");
                        let is_match = stack
                            .last()
                            .map(|(nt, _, tag)| {
                                *nt == NodeType::PseudoHtml
                                    && tag.as_deref().and_then(|t| t.split_whitespace().next())
                                        == Some(close_tag)
                            })
                            .unwrap_or(false);
                        if is_match {
                            let (_, children, _) = stack.pop().unwrap();
                            Self::emit(
                                &mut stack,
                                &mut result,
                                Node {
                                    n_type: NodeType::PseudoHtml,
                                    children,
                                    n_detail: t.t_detail.clone(),
                                    start: t.start,
                                    end: t.end
                                },
                            );
                        } else {
                            panic!(
                                "Unmatched closing </{}> at pos {}",
                                close_tag, t.start
                            );
                        }
                    } else if detail.trim_end().ends_with('/') {
                        Self::emit(
                            &mut stack,
                            &mut result,
                            Node {
                                n_type: NodeType::PseudoHtml,
                                n_detail: t.t_detail.clone(),
                                start: t.start,
                                ..Default::default()
                            },
                        );
                    } else {
                        stack.push((NodeType::PseudoHtml, vec![], t.t_detail.clone()));
                    }
                    i += 1;
                }

                TokenType::QMark => {
                    Self::emit(
                        &mut stack,
                        &mut result,
                        Node {
                            n_type: NodeType::Text,
                            n_detail: Some("?".to_string()),
                            ..Default::default()
                        },
                    );
                    i += 1;
                }

                TokenType::Eof => {
                    if !stack.is_empty() {
                        let names: Vec<String> = stack
                            .iter()
                            .map(|(nt, _, tag)| {
                                tag.clone()
                                    .unwrap_or_else(|| format!("{:?}", nt))
                            })
                            .collect();
                        eprintln!("warning: unclosed formatting at EOF: {}", names.join(", "));
                    }
                    break;
                }

                _ => {
                    panic!(
                        "Unexpected token {:?} at pos {}",
                        t.t_type, t.start
                    );
                }
            }
        }

        if !stack.is_empty() {
            let names: Vec<String> = stack.iter().map(|(nt, _, tag)| {
                tag.clone().unwrap_or_else(|| format!("{:?}", nt))
            }).collect();
            eprintln!("warning: unclosed formatting: {}", names.join(", "));
        }

        result
    }

    fn line_end(&self, tokens: &[Token], start: usize) -> usize {
        tokens[start..]
            .iter()
            .position(|t: &Token| matches!(t.t_type, TokenType::Newline | TokenType::Eof))
            .map(|p: usize| start + p)
            .unwrap_or(tokens.len())
    }

    fn para_end(&self, tokens: &[Token], start: usize) -> usize {
        let mut saw_newline = false;
        for (offset, t) in tokens[start..].iter().enumerate() {
            match t.t_type {
                TokenType::Eof => return start + offset,
                TokenType::Newline if saw_newline => return start + offset,
                TokenType::Newline => saw_newline = true,
                _ => saw_newline = false,
            }
        }
        tokens.len()
    }

    fn leaf(t: &Token) -> Node {
        Node {
            n_type: match t.t_type {
                TokenType::Text => NodeType::Text,
                TokenType::Whitespace => NodeType::Whitespace,
                TokenType::Linebreak => NodeType::Linebreak,
                TokenType::Newline => NodeType::Newline,
                _ => unreachable!(),
            },
            n_detail: t.t_detail.clone(),
            start: t.start,
            end: t.end,
            children: vec![],
        }
    }

    fn emit(stack: &mut Stack, result: &mut Vec<Node>, node: Node) {
        if let Some(top) = stack.last_mut() {
            top.1.push(node);
        } else {
            result.push(node);
        }
    }

    fn to_nt(tt: TokenType) -> NodeType {
        match tt {
            TokenType::Bold => NodeType::Bold,
            TokenType::Italic => NodeType::Italic,
            TokenType::Underline => NodeType::Underline,
            _ => unreachable!(),
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::lexing::Lexer;
    use super::Parser;

    #[test]
    fn debug_parse() {
        let input = "****bold**** and //italic//";
        let tokens = Lexer::new().tokenise(input);
        let ast = Parser::new().nodeify(&tokens);
        println!("{:#?}", ast);
    }
}