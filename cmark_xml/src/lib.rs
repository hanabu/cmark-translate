// SPDX-License-Identifier: MIT
//!
//! Convert CommonMark <=> XML
//!

/// XML namespace
const NS: &str = "markdown";

/// Read CommonMark with frontmatter
///
/// Returns tuple, (CommonMark body, frontmatter)
pub fn read_cmark_with_frontmatter<R: std::io::Read>(
    reader: &mut R,
) -> std::io::Result<(String, Option<String>)> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;

    if buf.starts_with("+++") {
        // TOML frontmatter
        split_frontmatter(&buf, "+++")
    } else if buf.starts_with("---") {
        // YAML frontmatter
        split_frontmatter(&buf, "---")
    } else {
        // No frontmatter, only CommonMark body
        Ok((buf, None))
    }
}

/// Split frontmatter and CommonMark body
fn split_frontmatter(filebody: &str, delimiter: &str) -> std::io::Result<(String, Option<String>)> {
    let mut iter = filebody.splitn(3, delimiter);
    let _ = iter.next(); // should empty
    let frontmatter = iter.next();
    let cmark_body = iter.next();

    if let (Some(frontmatter), Some(cmark_body)) = (frontmatter, cmark_body) {
        Ok((cmark_body.to_string(), Some(frontmatter.to_string())))
    } else {
        // second delimiter can not be found.
        Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof))
    }
}

/// Convert CommonMark text to XML string
///
/// If CommonMark text contains Jinja style shortcode {{ ... }} used in Hugo, Zora, etc.,
/// set escape_shortcode to true.
pub fn xml_from_cmark(cmark_text: &str, escape_shortcode: bool) -> String {
    let mut buf = Vec::<u8>::new();

    let xml_root = xmldom_from_cmark(cmark_text, escape_shortcode);
    xml_root.write_to(&mut buf).unwrap();

    String::from_utf8(buf).unwrap()
}

/// Convert CommonMark text to XML DOM
///
/// If CommonMark text contains Jinja style shortcode {{ ... }} used in Hugo, Zora, etc.,
/// set escape_shortcode to true.
pub fn xmldom_from_cmark(cmark_text: &str, escape_shortcode: bool) -> minidom::Element {
    // parse body as comrak AST
    let arena = comrak::Arena::new();

    let ast_root = if escape_shortcode {
        // pre-process shortcodes
        let escaped = escape_all_shortcodes(&cmark_text);
        log::trace!("escape_shortcode: {:?}\n", escaped);
        comrak::parse_document(&arena, &escaped, &comrak_options())
    } else {
        // no escape
        comrak::parse_document(&arena, cmark_text, &comrak_options())
    };

    if let minidom::Node::Element(xml) = xml_from_ast(&ast_root) {
        xml
    } else {
        // incase of no element, returns empty <body/>
        minidom::Element::bare("body", NS)
    }
}

/// Convert XML text back to CommonMark text
///
/// If XML contains escaped shortcode, set escape_shortcode to true.
pub fn cmark_from_xml(xml_str: &str, escape_shortcode: bool) -> minidom::Result<String> {
    let xml_root: minidom::Element = xml_str.parse()?;
    Ok(cmark_from_xmldom(&xml_root, escape_shortcode))
}

/// Convert XML DOM back to CommonMark text
///
/// If XML contains escaped shortcode, set escape_shortcode to true.
pub fn cmark_from_xmldom(xml_root: &minidom::Element, escape_shortcode: bool) -> String {
    // Convert XML to Comrak AST
    let arena = comrak::Arena::new();
    let ast_root = ast_from_xml(&arena, &xml_root);

    // AST to plain CommonMark
    let mut buf = Vec::<u8>::new();
    comrak::format_commonmark(ast_root, &comrak_options(), &mut buf).unwrap();
    let cmark_text = String::from_utf8(buf).unwrap();
    if escape_shortcode {
        unescape_all_shortcodes(&cmark_text)
    } else {
        cmark_text
    }
}

/// Escape shortcode {{...}} with <!--{{...}}-->, {%...%} with <!--{%...%}-->
fn escape_all_shortcodes(cmark_text: &str) -> String {
    let mut substr = cmark_text;
    let mut escaped1 = String::new();

    // escape {{ ... }}
    while let Some((left, right)) = substr.split_once("{{") {
        escaped1 += left;
        if let Some((shortcode, rest)) = right.split_once("}}") {
            escaped1 += &format!("<!--{{{{{}}}}}-->", shortcode);
            substr = rest;
        } else {
            // No "}}" found, assume all string upto EOF are shortcode
            escaped1 += &format!("<!--{{{{{}}}}}-->", right);
            substr = "";
            break;
        }
    }
    escaped1 += substr;

    // escape {% ... %}
    substr = &escaped1;
    let mut escaped2 = String::new();
    while let Some((left, right)) = substr.split_once("{%") {
        escaped2 += left;
        if let Some((shortcode, rest)) = right.split_once("%}") {
            escaped2 += &format!("<!--{{%{}%}}-->", shortcode);
            substr = rest;
        } else {
            // No "%}" found, assume all string upto EOF are shortcode
            escaped2 += &format!("<!--{{%{}%}}-->", right);
            substr = "";
            break;
        }
    }
    escaped2 += substr;

    escaped2
}

/// Restore escaped shortcodes  <!--{{...}}--> with {{...}} and <!--{%...%}--> with {%...%}
fn unescape_all_shortcodes(escaped: &str) -> String {
    let mut substr = escaped;
    let mut restored1 = String::new();

    // restore {% ... %}
    while let Some((left, right)) = substr.split_once("<!--{%") {
        restored1 += left;
        if let Some((shortcode, rest)) = right.split_once("%}-->") {
            restored1 += &format!("{{%{}%}}", shortcode);
            substr = rest;
        } else {
            // No "}}" found, assume all string upto EOF are shortcode
            restored1 += &format!("{{%{}%}}", right);
            substr = "";
            break;
        }
    }
    restored1 += substr;

    // restore {{ ... }}
    substr = &restored1;
    let mut restored2 = String::new();
    while let Some((left, right)) = substr.split_once("<!--{{") {
        restored2 += left;
        if let Some((shortcode, rest)) = right.split_once("}}-->") {
            restored2 += &format!("{{{{{}}}}}", shortcode);
            substr = rest;
        } else {
            // No "}}" found, assume all string upto EOF are shortcode
            restored2 += &format!("{{{{{}}}}}", right);
            substr = "";
            break;
        }
    }
    restored2 += substr;

    restored2
}

/// Create XML DOM from Comrak AST
fn xml_from_ast<'a>(ast_node: &'a comrak::nodes::AstNode<'a>) -> minidom::node::Node {
    use comrak::nodes::{ListType::*, NodeValue::*};
    use minidom::node::Node;
    use minidom::Element;
    use std::str::from_utf8;
    let ast = &ast_node.data.borrow();

    // Convert Markdown AST to XML nodes
    let xml_node = match &ast.value {
        Document => Node::Element(Element::bare("body", NS)),
        FrontMatter(t) => Node::Element(
            Element::builder("header", NS)
                .append(std::str::from_utf8(t).unwrap())
                .build(),
        ),
        BlockQuote => Node::Element(Element::bare("blockquote", NS)),
        List(nl) => {
            use comrak::nodes::{ListDelimType::*, ListType::*};
            let name = if nl.list_type == Ordered { "ol" } else { "ul" };
            let elm = Element::builder(name, NS)
                .attr("type", if nl.list_type == Ordered { "o" } else { "u" })
                .attr("offset", nl.marker_offset)
                .attr("padding", nl.padding)
                .attr("start", nl.start)
                .attr("delimiter", if nl.delimiter == Period { "." } else { ")" })
                .attr("tight", nl.tight as i32)
                .build();
            Node::Element(elm)
        }
        Item(nl) => {
            use comrak::nodes::ListDelimType::*;
            let elm = Element::builder("li", NS)
                .attr("type", if nl.list_type == Ordered { "o" } else { "u" })
                .attr("offset", nl.marker_offset)
                .attr("padding", nl.padding)
                .attr("start", nl.start)
                .attr("delimiter", if nl.delimiter == Period { "." } else { ")" })
                .attr("tight", nl.tight as i32)
                .build();
            Node::Element(elm)
        }
        DescriptionList => Node::Element(Element::bare("dl", NS)),
        DescriptionItem(nd) => Node::Element(
            Element::builder("di", NS)
                .attr("offset", nd.marker_offset)
                .attr("padding", nd.padding)
                .build(),
        ),
        DescriptionTerm => Node::Element(Element::bare("dt", NS)),
        DescriptionDetails => Node::Element(Element::bare("dd", NS)),
        CodeBlock(cb) => Node::Element(
            Element::builder("pre", NS)
                .attr("info", from_utf8(&cb.info).unwrap())
                .append(from_utf8(&cb.literal).unwrap())
                .build(),
        ),
        HtmlBlock(hb) => Node::Element(
            Element::builder("object", NS)
                .attr("type", hb.block_type as i32)
                .attr("literal", from_utf8(&hb.literal).unwrap())
                .build(),
        ),
        Paragraph => Node::Element(Element::bare("p", NS)),
        Heading(hd) => Node::Element(
            Element::builder(format!("h{}", hd.level), NS)
                .attr("level", hd.level)
                .build(),
        ),
        ThematicBreak => Node::Element(Element::bare("hr", NS)),
        FootnoteDefinition(t) => Node::Element(
            Element::builder("footer", NS)
                .attr("name", from_utf8(t).unwrap())
                .build(),
        ),
        Table(align) => {
            use comrak::nodes::TableAlignment::*;
            let align_str = align
                .iter()
                .map(|v| match v {
                    None => '-',
                    Left => 'l',
                    Center => 'c',
                    Right => 'r',
                })
                .collect::<String>();
            Node::Element(
                Element::builder("table", NS)
                    .attr("align", align_str)
                    .build(),
            )
        }
        TableRow(hdr) => {
            let elm = if *hdr {
                Element::bare("th", NS)
            } else {
                Element::bare("tr", NS)
            };
            Node::Element(elm)
        }
        TableCell => Node::Element(Element::bare("td", NS)),
        Text(t) => Node::Text(String::from_utf8(t.clone()).unwrap()),
        TaskItem(checked) => Node::Element(
            Element::builder("input", NS)
                .attr("checked", *checked as i32)
                .build(),
        ),
        SoftBreak => Node::Element(Element::bare("wbr", NS)),
        LineBreak => Node::Element(Element::bare("br", NS)),
        Code(t) => Node::Element(
            Element::builder("code", NS)
                .attr("literal", from_utf8(&t.literal).unwrap())
                .build(),
        ),
        HtmlInline(t) => Node::Element(
            Element::builder("embed", NS)
                .attr("literal", from_utf8(t).unwrap())
                .build(),
        ),
        Emph => Node::Element(Element::bare("em", NS)),
        Strong => Node::Element(Element::bare("strong", NS)),
        Strikethrough => Node::Element(Element::bare("del", NS)),
        Superscript => Node::Element(Element::bare("sup", NS)),
        Link(url) => Node::Element(
            Element::builder("a", NS)
                .attr("href", from_utf8(&url.url).unwrap())
                .attr("title", from_utf8(&url.title).unwrap())
                .build(),
        ),
        Image(url) => Node::Element(
            Element::builder("img", NS)
                .attr("src", from_utf8(&url.url).unwrap())
                .attr("title", from_utf8(&url.title).unwrap())
                .build(),
        ),
        FootnoteReference(t) => Node::Element(
            Element::builder("sub", NS)
                .attr("name", from_utf8(t).unwrap())
                .build(),
        ),
    };

    // Append child nodes
    if let Node::Element(mut xml_elm) = xml_node {
        for ast_child in ast_node.children() {
            let xml_child = xml_from_ast(ast_child);
            xml_elm.append_node(xml_child);
        }
        Node::Element(xml_elm)
    } else {
        xml_node
    }
}

/// Create Comrak AST from XML DOM
fn ast_from_xml<'a>(
    arena: &'a comrak::Arena<comrak::nodes::AstNode<'a>>,
    xml_elm: &minidom::Element,
) -> &'a comrak::nodes::AstNode<'a> {
    use comrak::nodes::NodeValue::*;

    let nodeval = match xml_elm.name() {
        "body" => Document,
        "header" => FrontMatter(xml_elm.text().into_bytes()),
        "blockquote" => BlockQuote,
        "ul" | "ol" => List(node_list_from_xml(&xml_elm)),
        "li" => Item(node_list_from_xml(&xml_elm)),
        "dl" => DescriptionList,
        "di" => DescriptionItem(comrak::nodes::NodeDescriptionItem {
            marker_offset: xml_elm.attr("offset").map_or(0, |v| v.parse().unwrap_or(0)),
            padding: xml_elm
                .attr("padding")
                .map_or(0, |v| v.parse().unwrap_or(0)),
        }),
        "dt" => DescriptionTerm,
        "dd" => DescriptionDetails,
        "pre" => CodeBlock(comrak::nodes::NodeCodeBlock {
            fenced: true,
            fence_char: '`' as u8,
            fence_length: 3,
            fence_offset: 0,
            info: Vec::from(xml_elm.attr("info").unwrap_or("")),
            literal: xml_elm.text().into(),
        }),
        "object" => HtmlBlock(comrak::nodes::NodeHtmlBlock {
            block_type: xml_elm.attr("type").map_or(0, |v| v.parse().unwrap_or(0)),
            literal: Vec::from(xml_elm.attr("literal").unwrap_or("")),
        }),
        "p" => Paragraph,
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Heading(comrak::nodes::NodeHeading {
            level: xml_elm.attr("level").map_or(1, |v| v.parse().unwrap_or(1)),
            setext: false,
        }),
        "hr" => ThematicBreak,
        "footer" => FootnoteDefinition(Vec::from(xml_elm.attr("name").unwrap_or(""))),
        "table" => {
            use comrak::nodes::TableAlignment::*;
            let align = xml_elm
                .attr("align")
                .unwrap()
                .chars()
                .map(|c| match c {
                    'l' => Left,
                    'c' => Center,
                    'r' => Right,
                    _ => None,
                })
                .collect::<Vec<_>>();
            Table(align)
        }
        "th" => TableRow(true),
        "tr" => TableRow(false),
        "td" => TableCell,
        "input" => TaskItem(xml_elm.attr("checked") == Some("1")),
        "wbr" => SoftBreak,
        "br" => LineBreak,
        "code" => Code(comrak::nodes::NodeCode {
            num_backticks: 1,
            literal: Vec::from(xml_elm.attr("literal").unwrap_or("")),
        }),
        "embed" => HtmlInline(Vec::from(xml_elm.attr("literal").unwrap_or(""))),
        "em" => Emph,
        "strong" => Strong,
        "del" => Strikethrough,
        "sup" => Superscript,
        "a" => Link(comrak::nodes::NodeLink {
            url: Vec::from(xml_elm.attr("href").unwrap_or("")),
            title: Vec::from(xml_elm.attr("title").unwrap_or("")),
        }),
        "img" => Image(comrak::nodes::NodeLink {
            url: Vec::from(xml_elm.attr("src").unwrap_or("")),
            title: Vec::from(xml_elm.attr("title").unwrap_or("")),
        }),
        "sub" => FootnoteReference(Vec::from(xml_elm.attr("name").unwrap_or(""))),
        _ => Text(vec![]), // empty text for unknown XML element
    };

    let ast_node = arena.alloc(comrak::nodes::AstNode::from(nodeval));

    match xml_elm.name() {
        "header" | "pre" => {
            // Already parsed child texts
        }
        _ => {
            // Add children nodes
            for xml_child in xml_elm.nodes() {
                match xml_child {
                    minidom::Node::Element(element) => {
                        // recursively parse elements
                        let ast_child = ast_from_xml(arena, element);
                        ast_node.append(ast_child);
                    }
                    minidom::Node::Text(text) => {
                        let ast_child_text = arena.alloc(comrak::nodes::AstNode::from(Text(
                            text.clone().into_bytes(),
                        )));
                        ast_node.append(ast_child_text);
                    }
                }
            }
        }
    }

    ast_node
}

/// Comrak AST NodeList from XML element
fn node_list_from_xml(xml_elm: &minidom::Element) -> comrak::nodes::NodeList {
    use comrak::nodes::ListType::*;
    comrak::nodes::NodeList {
        list_type: if Some("o") == xml_elm.attr("type") {
            Ordered
        } else {
            Bullet
        },
        marker_offset: xml_elm.attr("offset").map_or(0, |v| v.parse().unwrap_or(0)),
        padding: xml_elm
            .attr("padding")
            .map_or(0, |v| v.parse().unwrap_or(0)),
        start: xml_elm.attr("start").map_or(0, |v| v.parse().unwrap_or(0)),
        delimiter: comrak::nodes::ListDelimType::Period,
        bullet_char: '-' as u8,
        tight: Some("1") == xml_elm.attr("tight"),
    }
}

/// Comrak options for CommonMark-XML conversion
fn comrak_options() -> comrak::ComrakOptions {
    comrak::ComrakOptions {
        extension: comrak::ComrakExtensionOptions {
            strikethrough: true,
            tagfilter: false,
            table: true,
            autolink: false,
            tasklist: false,
            superscript: false,
            header_ids: None,
            footnotes: false,
            description_lists: false,
            front_matter_delimiter: Some(String::from("+++")),
        },
        parse: comrak::ComrakParseOptions {
            smart: false,
            default_info_string: None,
        },
        render: comrak::ComrakRenderOptions {
            hardbreaks: false,
            github_pre_lang: false,
            width: 0,
            unsafe_: true,
            escape: false,
            ..Default::default()
        },
    }
}
