use crate::parser::{Node, NodeType};

pub struct Renderer<'a> {
    source: &'a str,
    headings: Vec<(u8, String, String)>, // (level, text, anchor_id)
    suppress_toc: bool,
    custom_toc: Option<String>,
}

impl<'a> Renderer<'a> {
    pub fn new(source: &'a str) -> Self {
        Renderer {
            source,
            headings: Vec::new(),
            suppress_toc: false,
            custom_toc: None,
        }
    }

    pub fn render(&mut self, nodes: &[Node]) -> String {
        self.render_children(nodes)
    }

    pub fn build_toc(&self) -> String {
        if self.suppress_toc || self.headings.is_empty() {
            return String::new();
        }

        if let Some(ref custom) = self.custom_toc {
            return custom.clone();
        }

        // auto-generate from self.headings
        let mut html = String::from("<ul class=\"menu menu-sm\">");
        for (level, text, anchor) in &self.headings {
            let indent = "  ".repeat((*level - 1) as usize);
            html.push_str(&format!(
                "{indent}<li><a href=\"#{anchor}\">{text}</a></li>\n"
            ));
        }
        html.push_str("</ul>");
        html
    }

    fn render_children(&mut self, children: &[Node]) -> String {
        children
            .iter()
            .map(|c| self.render_node(c))
            .collect::<Vec<_>>()
            .concat()
    }

    fn render_node(&mut self, node: &Node) -> String {
        match node.n_type {
            NodeType::Text => escape_html(&self.source[node.start..node.end]),

            NodeType::Bold => {
                format!(
                    "<strong class=\"mast-bold\">{}</strong>",
                    self.render_children(&node.children)
                )
            }

            NodeType::Italic => {
                format!(
                    "<em class=\"mast-italic\">{}</em>",
                    self.render_children(&node.children)
                )
            }

            NodeType::Underline => {
                format!(
                    "<u class=\"mast-underline\">{}</u>",
                    self.render_children(&node.children)
                )
            }

            NodeType::Heading => {
                let level = node
                    .n_detail
                    .as_deref()
                    .unwrap_or("6")
                    .parse::<u8>()
                    .unwrap_or(6);
                let text = self.render_flat_text(&node.children);
                let anchor = slugify(&text);
                self.headings.push((level, text.clone(), anchor.clone()));
                format!(
                    "<h{level} id=\"{anchor}\" class=\"mast-heading-{level}\">{}</h{level}>",
                    self.render_children(&node.children)
                )
            }

            NodeType::Macro => {
                let detail = node.n_detail.as_deref().unwrap_or("");
                match detail {
                    "NOTOC" => self.suppress_toc = true,
                    "CUSTOMTOC" => {
                        self.custom_toc = Some(self.render_children(&node.children));
                    }
                    _ => {}
                }
                String::new()
            }

            NodeType::Quote => {
                format!(
                    "<blockquote class=\"mast-blockquote\">{}</blockquote>",
                    self.render_children(&node.children)
                )
            }

            NodeType::Link => {
                let href = escape_html(node.n_detail.as_deref().unwrap_or(""));
                format!(
                    "<a href=\"{href}\" class=\"mast-link\">{}</a>",
                    self.render_children(&node.children),
                )
            }

            NodeType::Image => {
                let src = escape_html(node.n_detail.as_deref().unwrap_or(""));
                let alt = self.render_flat_text(&node.children);
                format!("<img src=\"{src}\" class=\"mast-image\" alt=\"{alt}\" />")
            }

            NodeType::Footnote => {
                format!(
                    "<sup class=\"mast-footnote\">{}</sup>",
                    self.render_children(&node.children)
                )
            }

            NodeType::Hr => String::from("<hr class=\"mast-hr\" />"),

            NodeType::PseudoHtml => {
                let detail = node.n_detail.as_deref().unwrap_or("");
                let is_self_closing = detail.trim_end().ends_with('/');
                let raw_detail = if is_self_closing {
                    detail.trim_end().trim_end_matches('/').trim_end()
                } else {
                    detail
                };
                let (tag, attrs) = split_tag_attrs(raw_detail);
                self.render_pseudohtml(tag, attrs, is_self_closing, &node.children)
            }

            NodeType::Linebreak => String::from("<br class=\"wiki-linebreak\" />"),

            NodeType::Whitespace => self.source[node.start..node.end].to_string(),

            NodeType::Paragraph => {
                format!(
                    "<p class=\"wiki-paragraph\">{}</p>",
                    self.render_children(&node.children)
                )
            }

            NodeType::Newline | NodeType::Pipe | NodeType::Eof => String::new(),
        }
    }

    fn render_pseudohtml(
        &mut self,
        tag: &str,
        attrs: &str,
        self_closing: bool,
        children: &[Node],
    ) -> String {
        match tag {
            "br" => String::from("<br class=\"wiki-br\" />"),

            "nowiki" => {
                if self_closing {
                    String::new()
                } else {
                    // raw text — no HTML processing of children
                    self.render_flat_text(children)
                }
            }

            "callout" => {
                let type_class = extract_attr(attrs, "type")
                    .map(|t| format!(" wiki-callout-{}", t))
                    .unwrap_or_default();
                let title_attr = extract_attr(attrs, "title")
                    .map(|t| format!(" data-title=\"{}\"", escape_html(&t)))
                    .unwrap_or_default();
                if self_closing {
                    format!("<div class=\"wiki-callout{type_class}\"{title_attr}></div>")
                } else {
                    format!(
                        "<div class=\"wiki-callout{type_class}\"{title_attr}>{}</div>",
                        self.render_children(children)
                    )
                }
            }

            "accordion" => {
                if self_closing {
                    String::new()
                } else {
                    format!(
                        "<details class=\"wiki-accordion\">{}</details>",
                        self.render_children(children)
                    )
                }
            }

            _ => {
                let cls = format!("wiki-{}", tag);
                if self_closing {
                    format!("<{tag} class=\"{cls}\" />")
                } else {
                    format!(
                        "<{tag} class=\"{cls}\">{}</{tag}>",
                        self.render_children(children)
                    )
                }
            }
        }
    }

    fn render_flat_text(&self, children: &[Node]) -> String {
        children
            .iter()
            .map(|c| match c.n_type {
                NodeType::Text => escape_html(&self.source[c.start..c.end]),
                NodeType::Whitespace => c.n_detail.as_deref().unwrap_or(" ").to_string(),
                _ => self.render_flat_text(&c.children),
            })
            .collect::<Vec<_>>()
            .concat()
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn split_tag_attrs(detail: &str) -> (&str, &str) {
    let detail = detail.trim();
    match detail.find(char::is_whitespace) {
        Some(pos) => (&detail[..pos], detail[pos..].trim()),
        None => (detail, ""),
    }
}

fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == ' ' {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

fn extract_attr<'a>(attrs: &'a str, name: &str) -> Option<String> {
    let pattern = format!("{}=\"", name);
    let start = attrs.find(&pattern)?;
    let value_start = start + pattern.len();
    let value_end = attrs[value_start..].find('"')?;
    Some(attrs[value_start..value_start + value_end].to_string())
}

#[cfg(test)]
mod tests {
    use crate::lexing::Lexer;
    use crate::parser::Parser;
    use crate::renderer::Renderer;

    #[test]
    fn debug_parse() {
        let input = "======heading======";
        let tokens = Lexer::new().tokenise(input);
        let ast = Parser::new().nodeify(&tokens);
        let html = Renderer::new(input).render(&ast);
        println!("{:#?}", html);
    }
}
