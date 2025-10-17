use pulldown_cmark::{Parser, Event, Tag, TagEnd};

pub fn markdown_to_pango(markdown: &str) -> String {
    let parser = Parser::new(markdown);
    let mut output = String::new();
    let mut in_code_block = false;
    
    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Strong => {
                    output.push_str("<b>");
                }
                Tag::Emphasis => {
                    output.push_str("<i>");
                }
                Tag::CodeBlock(_) => {
                    if !output.is_empty() && !output.ends_with("\n\n") {
                        output.push_str("\n");
                    }
                    output.push_str("<tt>");
                    in_code_block = true;
                }
                Tag::Heading { .. } => {
                    if !output.is_empty() && !output.ends_with("\n\n") {
                        output.push_str("\n");
                    }
                    output.push_str("<b><big>");
                }
                Tag::List(_) => {
                    if !output.is_empty() && !output.ends_with("\n\n") {
                        output.push_str("\n");
                    }
                }
                Tag::Item => {
                    output.push_str("• ");
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Strong => {
                    output.push_str("</b>");
                }
                TagEnd::Emphasis => {
                    output.push_str("</i>");
                }
                TagEnd::CodeBlock => {
                    output.push_str("</tt>\n");
                    in_code_block = false;
                }
                TagEnd::Heading { .. } => {
                    output.push_str("</big></b>\n");
                }
                TagEnd::Paragraph => {
                    output.push_str("\n");
                }
                TagEnd::Item => {
                    output.push_str("\n");
                }
                TagEnd::List(_) => {
                    output.push_str("\n");
                }
                _ => {}
            },
            Event::Text(text) => {
                let escaped = glib::markup_escape_text(&text);
                output.push_str(&escaped);
            }
            Event::Code(text) => {
                let escaped = glib::markup_escape_text(&text);
                output.push_str("<tt>");
                output.push_str(&escaped);
                output.push_str("</tt>");
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_code_block {
                    output.push('\n');
                } else {
                    output.push(' ');
                }
            }
            _ => {}
        }
    }
    
    output
}
