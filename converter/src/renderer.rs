use crate::parser::{Node, NodeType};

pub struct Renderer;

impl Renderer {
    pub fn render(&self, nodes: &[Node]) -> String {
        self.render_children(nodes)
    }

    fn render_children(&self, children: &[Node]) -> String {
        children.iter().map(|c| self.render_node(c)).collect::<Vec<_>>().concat()
    }

    fn render_node(&self, node: &Node) -> String {
        match node.n_type {
            NodeType::Text => {
                escape_html(node.n_detail.as_deref().unwrap_or(""))
            }

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
                let level = node.n_detail.as_deref().unwrap_or("6");
                format!(
                    "<h{level} class=\"mast-heading-{level}\">{}</h{level}>",
                    self.render_children(&node.children),
                )
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

            NodeType::Linebreak => {
                String::from("<br class=\"wiki-linebreak\" />")
            }

            NodeType::Whitespace => {
                node.n_detail.as_deref().unwrap_or(" ").to_string()
            }

            NodeType::Paragraph => {
                format!(
                    "<p class=\"wiki-paragraph\">{}</p>",
                    self.render_children(&node.children)
                )
            }

            NodeType::Newline | NodeType::Pipe | NodeType::Eof => {
                String::new()
            }
        }
    }

    fn render_pseudohtml(&self, tag: &str, attrs: &str, self_closing: bool, children: &[Node]) -> String {
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
        children.iter().map(|c| {
            match c.n_type {
                NodeType::Text => escape_html(c.n_detail.as_deref().unwrap_or("")),
                NodeType::Whitespace => c.n_detail.as_deref().unwrap_or(" ").to_string(),
                _ => self.render_flat_text(&c.children),
            }
        }).collect::<Vec<_>>().concat()
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

fn extract_attr<'a>(attrs: &'a str, name: &str) -> Option<String> {
    let pattern = format!("{}=\"", name);
    let start = attrs.find(&pattern)?;
    let value_start = start + pattern.len();
    let value_end = attrs[value_start..].find('"')?;
    Some(attrs[value_start..value_start + value_end].to_string())
}
