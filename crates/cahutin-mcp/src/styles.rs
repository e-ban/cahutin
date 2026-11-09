//! Styles de texte nommés et formats de page, tels que l'interface les connaît.

use carnet_core::{Align, Document, NamedTextStyle, PageSize};

/// Les styles de Cahutin. Copie de `BUILTIN_STYLES` dans `ui/src/lib/namedStyles.ts` : les garder pareils.
pub fn builtin() -> Vec<NamedTextStyle> {
    let s = |id: &str, name: &str, font: &str, size_pt: f64, color: &str, align: Align| NamedTextStyle {
        id: id.into(),
        name: name.into(),
        font: font.into(),
        size_pt,
        color: color.into(),
        align,
        outline: None,
    };
    vec![
        s("titre", "Titre", "Playwrite FR Moderne", 28.0, "#c0392b", Align::Center),
        s("consigne", "Consigne", "Andika", 16.0, "#1f2c4a", Align::Left),
        s("texte", "Texte", "Andika", 14.0, "#222222", Align::Left),
        s("prenom", "Prénom", "Playwrite FR Moderne", 32.0, "#1f2c4a", Align::Center),
    ]
}

/// Les styles du document : ceux de Cahutin (tels que le document les a redéfinis), puis les siens, comme
/// `stylesOf` dans l'interface.
pub fn of(doc: &Document) -> Vec<(NamedTextStyle, bool)> {
    let builtin = builtin();
    let mut out: Vec<(NamedTextStyle, bool)> = builtin.iter().map(|b| (doc.text_styles.iter().find(|s| s.id == b.id).unwrap_or(b).clone(), true)).collect();
    out.extend(doc.text_styles.iter().filter(|s| !builtin.iter().any(|b| b.id == s.id)).map(|s| (s.clone(), false)));
    out
}

/// Petit et grand côté des formats A, en mm (comme `FORMATS` dans `ui/src/lib/pageFormat.ts`).
pub const FORMATS: [(&str, f64, f64); 3] = [("A5", 148.0, 210.0), ("A4", 210.0, 297.0), ("A3", 297.0, 420.0)];

/// Le format de la page, s'il est connu (en portrait ou en paysage).
pub fn format_of(size: PageSize) -> Option<&'static str> {
    let (short, long) = (size.width_mm.min(size.height_mm), size.width_mm.max(size.height_mm));
    FORMATS.iter().find(|(_, s, l)| (s - short).abs() < 0.5 && (l - long).abs() < 0.5).map(|f| f.0)
}
