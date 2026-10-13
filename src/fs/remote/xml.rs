//! What the XML answers of WebDAV and S3 share: entity references (`&amp;`,
//! `&#38;`...) arrive as events of their own and are resolved here.

use quick_xml::events::BytesRef;

/// The text an entity or character reference stands for (unknown entities
/// are dropped).
pub fn reference_text(reference: &BytesRef<'_>) -> Result<String, quick_xml::Error> {
    if reference.is_char_ref() {
        return Ok(reference
            .resolve_char_ref()?
            .map(String::from)
            .unwrap_or_default());
    }
    Ok(quick_xml::escape::unescape(&format!("&{};", &**reference))
        .map(|t| t.into_owned())
        .unwrap_or_default())
}
