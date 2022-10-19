pub fn parse_md() -> std::io::Result<()> {
    let mdtext = std::fs::read_to_string("index.en.md").unwrap();

    // pre-process shortcode

    // parse body as comrak AST
    let arena = comrak::Arena::new();
    let mdroot = comrak::parse_document(&arena, &mdtext, &comrak_options());

    if let minidom::Node::Element(xml) = xml_from_ast(&mdroot) {
        xml.write_to(&mut std::io::stdout()).unwrap();

        let new_arena = comrak::Arena::new();
        let new_root = ast_from_xml(&new_arena, &xml);

        comrak::format_commonmark(new_root, &comrak_options(), &mut std::io::stdout()).unwrap();
    }
    Ok(())
}

fn xml_from_ast<'a>(ast_node: &'a comrak::nodes::AstNode<'a>) -> minidom::node::Node {
    use comrak::nodes::{ListType::*, NodeValue::*};
    use minidom::node::Node;
    use minidom::Element;
    use std::str::from_utf8;
    const NS: &str = "markdown";
    let ast = &ast_node.data.borrow();

    // Convert Markdown AST to XML nodes
    let xml = match &ast.value {
        Document => Node::Element(Element::bare("body", NS)),
        FrontMatter(t) => Node::Element(
            Element::builder("header", NS)
                .append(std::str::from_utf8(t).unwrap())
                .build(),
        ),
        BlockQuote => Node::Element(Element::bare("blockquote", NS)),
        List(nl) => {
            use comrak::nodes::{ListDelimType::*, ListType::*};
            let elm = Element::builder("ul", NS)
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
                .attr("literal", from_utf8(&hb.literal).unwrap())
                .build(),
        ),
        Paragraph => Node::Element(Element::bare("p", NS)),
        Heading(hd) => Node::Element(Element::builder("h1", NS).attr("level", hd.level).build()),
        ThematicBreak => Node::Element(Element::bare("hr", NS)),
        FootnoteDefinition(t) => Node::Element(
            Element::builder("footer", NS)
                .attr("name", from_utf8(t).unwrap())
                .build(),
        ),
        Table(_) => Node::Element(Element::bare("table", NS)),
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
                .append(from_utf8(&t.literal).unwrap())
                .build(),
        ),
        HtmlInline(t) => Node::Element(
            Element::builder("inline", NS)
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
                .attr("href", from_utf8(&url.url).unwrap())
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
    if let Node::Element(mut xml) = xml {
        for ast_child in ast_node.children() {
            let xml_child = xml_from_ast(ast_child);
            xml.append_node(xml_child);
        }
        Node::Element(xml)
    } else {
        xml
    }
}

fn ast_from_xml<'a>(
    arena: &'a comrak::Arena<comrak::nodes::AstNode<'a>>,
    node: &minidom::Element,
) -> &'a comrak::nodes::AstNode<'a> {
    use comrak::nodes::NodeValue::*;

    let nodeval = match node.name() {
        "body" => Document,
        "header" => FrontMatter(node.text().into_bytes()),
        "blockquote" => BlockQuote,
        "ul" | "ol" => List(node_list_from_xml(&node)),
        "li" => Item(node_list_from_xml(&node)),
        "dl" => DescriptionList,
        "di" => DescriptionItem(comrak::nodes::NodeDescriptionItem {
            marker_offset: node.attr("offset").map_or(0, |v| v.parse().unwrap_or(0)),
            padding: node.attr("padding").map_or(0, |v| v.parse().unwrap_or(0)),
        }),
        "dt" => DescriptionTerm,
        "dd" => DescriptionDetails,
        "pre" => CodeBlock(comrak::nodes::NodeCodeBlock {
            fenced: true,
            fence_char: '`' as u8,
            fence_length: 3,
            fence_offset: 0,
            info: Vec::from(node.attr("info").unwrap_or("")),
            literal: node.text().into(),
        }),
        "object" => HtmlBlock(comrak::nodes::NodeHtmlBlock {
            block_type: '<' as u8,
            literal: Vec::from(node.attr("literal").unwrap_or("")),
        }),
        "p" => Paragraph,
        "h1" => Heading(comrak::nodes::NodeHeading {
            level: node.attr("level").map_or(1, |v| v.parse().unwrap_or(1)),
            setext: false,
        }),
        "hr" => ThematicBreak,
        "footer" => FootnoteDefinition(Vec::from(node.attr("name").unwrap_or(""))),
        "table" => Table(vec![]),
        "th" => TableRow(true),
        "tr" => TableRow(false),
        "td" => TableCell,
        "input" => TaskItem(node.attr("checked") == Some("1")),
        "wbr" => SoftBreak,
        "br" => LineBreak,
        "code" => Code(comrak::nodes::NodeCode {
            num_backticks: 1,
            literal: Vec::from(node.attr("literal").unwrap_or("")),
        }),
        "inline" => HtmlInline(Vec::from(node.attr("literal").unwrap_or(""))),
        "em" => Emph,
        "strong" => Strong,
        "del" => Strikethrough,
        "sup" => Superscript,
        "a" => Link(comrak::nodes::NodeLink {
            url: Vec::from(node.attr("href").unwrap_or("")),
            title: Vec::from(node.attr("title").unwrap_or("")),
        }),
        "img" => Image(comrak::nodes::NodeLink {
            url: Vec::from(node.attr("href").unwrap_or("")),
            title: Vec::from(node.attr("title").unwrap_or("")),
        }),
        "sub" => FootnoteReference(Vec::from(node.attr("name").unwrap_or(""))),
        _ => Text(vec![]), // empty text for unknown XML element
    };

    let ast_node = arena.alloc(comrak::nodes::AstNode::from(nodeval));

    match node.name() {
        "header" | "pre" => {
            // Already parsed child texts
        }
        _ => {
            // Add children nodes
            for xml_child in node.nodes() {
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

fn node_list_from_xml(node: &minidom::Element) -> comrak::nodes::NodeList {
    use comrak::nodes::ListType::*;
    comrak::nodes::NodeList {
        list_type: if Some("o") == node.attr("type") {
            Ordered
        } else {
            Bullet
        },
        marker_offset: node.attr("offset").map_or(0, |v| v.parse().unwrap_or(0)),
        padding: node.attr("padding").map_or(0, |v| v.parse().unwrap_or(0)),
        start: node.attr("start").map_or(0, |v| v.parse().unwrap_or(0)),
        delimiter: comrak::nodes::ListDelimType::Period,
        bullet_char: '-' as u8,
        tight: Some("1") == node.attr("tight"),
    }
}

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
