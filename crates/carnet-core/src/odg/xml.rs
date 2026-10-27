//! Écriture XML minimale, avec échappement des attributs et du texte.

use quick_xml::escape::escape;

pub(crate) struct Xml {
    buf: String,
    stack: Vec<&'static str>,
}

impl Xml {
    pub fn new() -> Xml {
        Xml { buf: String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"), stack: vec![] }
    }

    /// Fragment sans déclaration XML, à insérer dans un autre document.
    pub fn fragment() -> Xml {
        Xml { buf: String::new(), stack: vec![] }
    }

    fn start(&mut self, name: &str, attrs: &[(&str, &str)]) {
        self.buf.push('<');
        self.buf.push_str(name);
        for (k, v) in attrs {
            self.buf.push(' ');
            self.buf.push_str(k);
            self.buf.push_str("=\"");
            self.buf.push_str(&escape(*v));
            self.buf.push('"');
        }
    }

    pub fn open(&mut self, name: &'static str, attrs: &[(&str, &str)]) {
        self.start(name, attrs);
        self.buf.push('>');
        self.stack.push(name);
    }

    pub fn empty(&mut self, name: &str, attrs: &[(&str, &str)]) {
        self.start(name, attrs);
        self.buf.push_str("/>");
    }

    pub fn text(&mut self, t: &str) {
        self.buf.push_str(&escape(t));
    }

    pub fn raw(&mut self, xml: &str) {
        self.buf.push_str(xml);
    }

    pub fn close(&mut self) {
        let name = self.stack.pop().expect("close sans open");
        self.buf.push_str("</");
        self.buf.push_str(name);
        self.buf.push('>');
    }

    pub fn finish(self) -> String {
        assert!(self.stack.is_empty(), "éléments non fermés : {:?}", self.stack);
        self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_attributes_and_text() {
        let mut x = Xml::fragment();
        x.open("a", &[("t", "\"<&>\"")]);
        x.text("1 < 2 & 3");
        x.empty("b", &[]);
        x.close();
        assert_eq!(x.finish(), "<a t=\"&quot;&lt;&amp;&gt;&quot;\">1 &lt; 2 &amp; 3<b/></a>");
    }
}
