//! A small XML reader for the XML openNURBS keeps in 3dm files (`ON_XMLNode`): the RDK document
//! (render content, sun, ground plane and the other rendering settings) and the XML user data
//! of objects (decals, mesh modifiers). Elements with their attributes, text and children;
//! comments, declarations and processing instructions are skipped, the five named entities and
//! character references are decoded. Depth and element count are limited, so that a damaged or
//! hostile file cannot exhaust the stack or memory.

use threedm_core::{err, R};

/// How deep elements may nest.
const MAX_DEPTH: usize = 128;
/// How many elements one document may have.
const MAX_ELEMENTS: usize = 200_000;

/// An element: its tag, its attributes in order, its text (the parts between its children,
/// joined) and its children.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Node {
    pub tag: String,
    pub attributes: Vec<(String, String)>,
    pub text: String,
    pub children: Vec<Node>,
}

impl Node {
    /// The value of an attribute.
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }

    /// The first child with this tag.
    pub fn child(&self, tag: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.tag == tag)
    }

    /// The node at a path of tags separated by `/`, from this one.
    pub fn path(&self, path: &str) -> Option<&Node> {
        path.split('/').filter(|p| !p.is_empty()).try_fold(self, |n, tag| n.child(tag))
    }
}

/// Parses a document: the element that holds its top-level elements (with an empty tag).
pub fn parse(s: &str) -> R<Node> {
    let mut p = Parser { s: s.as_bytes(), pos: 0, elements: 0 };
    let mut root = Node::default();
    p.content(&mut root, 0)?;
    Ok(root)
}

struct Parser<'a> {
    s: &'a [u8],
    pos: usize,
    elements: usize,
}

impl Parser<'_> {
    fn starts(&self, what: &str) -> bool {
        self.s[self.pos..].starts_with(what.as_bytes())
    }

    /// Moves past `what`, which must follow somewhere.
    fn skip_past(&mut self, what: &str) -> R<()> {
        let rest = &self.s[self.pos..];
        match rest.windows(what.len()).position(|w| w == what.as_bytes()) {
            Some(i) => {
                self.pos += i + what.len();
                Ok(())
            }
            None => err("XML: unterminated markup"),
        }
    }

    fn spaces(&mut self) {
        while self.pos < self.s.len() && self.s[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn name(&mut self) -> R<String> {
        let start = self.pos;
        while self.pos < self.s.len() && !self.s[self.pos].is_ascii_whitespace() && !b"/>=<\"'".contains(&self.s[self.pos]) {
            self.pos += 1;
        }
        if start == self.pos {
            return err("XML: a name expected");
        }
        Ok(String::from_utf8_lossy(&self.s[start..self.pos]).into_owned())
    }

    /// The children and text of `node` up to its end tag (or the end of the document at depth 0).
    fn content(&mut self, node: &mut Node, depth: usize) -> R<()> {
        loop {
            let start = self.pos;
            while self.pos < self.s.len() && self.s[self.pos] != b'<' {
                self.pos += 1;
            }
            if self.pos > start {
                node.text.push_str(&decode(&String::from_utf8_lossy(&self.s[start..self.pos])));
            }
            if self.pos >= self.s.len() {
                return if depth == 0 { Ok(()) } else { err("XML: an element without its end") };
            }
            if self.starts("<!--") {
                self.skip_past("-->")?;
            } else if self.starts("<![CDATA[") {
                self.pos += 9;
                let start = self.pos;
                self.skip_past("]]>")?;
                node.text.push_str(&String::from_utf8_lossy(&self.s[start..self.pos - 3]));
            } else if self.starts("<?") || self.starts("<!") {
                self.skip_past(">")?;
            } else if self.starts("</") {
                self.pos += 2;
                let tag = self.name()?;
                self.skip_past(">")?;
                if depth == 0 || tag != node.tag {
                    return err("XML: an end tag that does not match");
                }
                return Ok(());
            } else {
                self.pos += 1;
                if depth >= MAX_DEPTH {
                    return err("XML: elements nested too deep");
                }
                self.elements += 1;
                if self.elements > MAX_ELEMENTS {
                    return err("XML: too many elements");
                }
                let mut child = Node { tag: self.name()?, ..Node::default() };
                loop {
                    self.spaces();
                    if self.starts("/>") {
                        self.pos += 2;
                        break;
                    }
                    if self.starts(">") {
                        self.pos += 1;
                        self.content(&mut child, depth + 1)?;
                        break;
                    }
                    if self.pos >= self.s.len() {
                        return err("XML: an unterminated tag");
                    }
                    let name = self.name()?;
                    self.spaces();
                    let mut value = String::new();
                    if self.starts("=") {
                        self.pos += 1;
                        self.spaces();
                        let quote = *self.s.get(self.pos).ok_or("XML: an attribute without its value")?;
                        if quote != b'"' && quote != b'\'' {
                            return err("XML: an unquoted attribute value");
                        }
                        self.pos += 1;
                        let start = self.pos;
                        while self.pos < self.s.len() && self.s[self.pos] != quote {
                            self.pos += 1;
                        }
                        if self.pos >= self.s.len() {
                            return err("XML: an unterminated attribute value");
                        }
                        value = decode(&String::from_utf8_lossy(&self.s[start..self.pos]));
                        self.pos += 1;
                    }
                    child.attributes.push((name, value));
                }
                node.children.push(child);
            }
        }
    }
}

/// Text with its entities and character references decoded.
fn decode(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let end = rest.find(';').filter(|&e| e <= 12);
        let decoded = end.and_then(|e| {
            let entity = &rest[1..e];
            let c = match entity {
                "lt" => Some('<'),
                "gt" => Some('>'),
                "amp" => Some('&'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ if entity.starts_with("#x") || entity.starts_with("#X") => u32::from_str_radix(&entity[2..], 16).ok().and_then(char::from_u32),
                _ if entity.starts_with('#') => entity[1..].parse().ok().and_then(char::from_u32),
                _ => None,
            };
            c.map(|c| (c, e))
        });
        match decoded {
            Some((c, e)) => {
                out.push(c);
                rest = &rest[e + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elements_attributes_text_and_entities() {
        let doc = parse("<?xml version=\"1.0\"?><!-- c --><a x=\"1 &amp; 2\"><b type='bool'>true</b><c/>t&lt;&#65;&#x42;</a>").unwrap();
        let a = doc.child("a").unwrap();
        assert_eq!(a.attribute("x"), Some("1 & 2"));
        assert_eq!(a.path("b").unwrap().text, "true");
        assert_eq!(a.path("b").unwrap().attribute("type"), Some("bool"));
        assert!(a.child("c").is_some());
        assert_eq!(a.text, "t<AB");
    }

    #[test]
    fn damaged_documents_are_errors() {
        assert!(parse("<a><b></a>").is_err());
        assert!(parse("<a x=\"1></a>").is_err());
        assert!(parse(&"<a>".repeat(MAX_DEPTH + 2)).is_err());
        assert!(parse("<a>").is_err());
    }
}
