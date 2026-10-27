//! Archive ODG : `mimetype` en premier et non compressé, puis les parties listées dans le manifeste.

use super::xml::Xml;
use super::{NS, OdgError};
use std::io::{Seek, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

pub(crate) const MIMETYPE: &str = "application/vnd.oasis.opendocument.graphics";

pub(crate) struct Part {
    pub path: String,
    pub media_type: &'static str,
    pub bytes: Vec<u8>,
    pub compress: bool,
}

fn manifest(parts: &[Part]) -> String {
    let mut x = Xml::new();
    x.open(
        "manifest:manifest",
        &[("xmlns:manifest", "urn:oasis:names:tc:opendocument:xmlns:manifest:1.0"), ("manifest:version", "1.3")],
    );
    x.empty(
        "manifest:file-entry",
        &[("manifest:full-path", "/"), ("manifest:version", "1.3"), ("manifest:media-type", MIMETYPE)],
    );
    for p in parts {
        x.empty("manifest:file-entry", &[("manifest:full-path", &p.path), ("manifest:media-type", p.media_type)]);
    }
    x.close();
    x.finish()
}

pub(crate) fn meta_xml() -> String {
    let mut x = Xml::new();
    let mut root: Vec<(&str, &str)> = NS.to_vec();
    root.push(("office:version", "1.3"));
    x.open("office:document-meta", &root);
    x.open("office:meta", &[]);
    x.open("meta:generator", &[]);
    x.text(concat!("carnet/", env!("CARGO_PKG_VERSION")));
    x.close();
    x.close();
    x.close();
    x.finish()
}

pub(crate) fn settings_xml() -> String {
    let mut x = Xml::new();
    let mut root: Vec<(&str, &str)> = NS.to_vec();
    root.push(("office:version", "1.3"));
    x.open("office:document-settings", &root);
    x.open("office:settings", &[]);
    x.open("config:config-item-set", &[("config:name", "ooo:configuration-settings")]);
    for name in ["EmbedFonts", "EmbedOnlyUsedFonts"] {
        x.open("config:config-item", &[("config:name", name), ("config:type", "boolean")]);
        x.text("true");
        x.close();
    }
    x.close();
    x.close();
    x.close();
    x.finish()
}

pub(crate) fn write(parts: Vec<Part>, out: impl Write + Seek) -> Result<(), OdgError> {
    let mut zip = ZipWriter::new(out);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("mimetype", stored)?;
    zip.write_all(MIMETYPE.as_bytes())?;
    zip.start_file("META-INF/manifest.xml", deflated)?;
    zip.write_all(manifest(&parts).as_bytes())?;
    for p in &parts {
        zip.start_file(p.path.as_str(), if p.compress { deflated } else { stored })?;
        zip.write_all(&p.bytes)?;
    }
    zip.finish()?;
    Ok(())
}
