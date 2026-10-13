//! `PROPFIND` multistatus answers read into entries. Servers differ in
//! namespace prefixes (`D:`, `d:`, `lp1:`...), so elements are matched by
//! their local name.

use crate::fs::remote::xml::reference_text;
use quick_xml::events::Event;
use std::time::SystemTime;

/// What `PROPFIND` asks for.
pub const REQUEST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:"><D:prop>
<D:resourcetype/><D:getcontentlength/><D:getlastmodified/>
</D:prop></D:propfind>"#;

/// One `<response>` of a multistatus answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DavItem {
    /// The `href`, percent-encoded as sent.
    pub href: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

/// The element a text node belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    None,
    Href,
    Length,
    Modified,
}

/// The items of the multistatus document `xml`.
pub fn parse(xml: &str) -> Result<Vec<DavItem>, quick_xml::Error> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut items = Vec::new();
    let mut item: Option<DavItem> = None;
    let mut field = Field::None;
    loop {
        match reader.read_event()? {
            Event::Start(e) => match e.local_name().as_ref() {
                "response" => item = Some(DavItem::default()),
                "collection" => set_dir(&mut item),
                "href" => field = Field::Href,
                "getcontentlength" => field = Field::Length,
                "getlastmodified" => field = Field::Modified,
                _ => {}
            },
            Event::Empty(e) if e.local_name().as_ref() == "collection" => set_dir(&mut item),
            Event::Text(t) => fill(item.as_mut(), field, t.xml10_content().trim()),
            Event::GeneralRef(r) => fill(item.as_mut(), field, &reference_text(&r)?),
            Event::End(e) => {
                field = Field::None;
                if e.local_name().as_ref() == "response"
                    && let Some(done) = item.take()
                {
                    items.push(done);
                }
            }
            Event::Eof => return Ok(items),
            _ => {}
        }
    }
}

fn set_dir(item: &mut Option<DavItem>) {
    if let Some(item) = item {
        item.is_dir = true;
    }
}

fn fill(item: Option<&mut DavItem>, field: Field, text: &str) {
    let Some(item) = item else {
        return;
    };
    match field {
        Field::Href => item.href.push_str(text),
        Field::Length => item.size = text.parse().unwrap_or(0),
        Field::Modified => {
            item.modified = chrono::DateTime::parse_from_rfc2822(text)
                .ok()
                .map(SystemTime::from);
        }
        Field::None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_folders_files_sizes_and_times() {
        let xml = r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:" xmlns:lp1="DAV:">
 <d:response><d:href>/dav/dir/</d:href><d:propstat><d:prop>
  <lp1:resourcetype><d:collection/></lp1:resourcetype>
 </d:prop></d:propstat></d:response>
 <d:response><d:href>/dav/dir/a%20b.txt</d:href><d:propstat><d:prop>
  <lp1:resourcetype/><lp1:getcontentlength>12</lp1:getcontentlength>
  <lp1:getlastmodified>Sun, 06 Nov 1994 08:49:37 GMT</lp1:getlastmodified>
 </d:prop></d:propstat></d:response>
</d:multistatus>"#;
        let items = parse(xml).unwrap();
        assert_eq!(items.len(), 2);
        assert!(items[0].is_dir);
        assert_eq!(items[1].href, "/dav/dir/a%20b.txt");
        assert_eq!(items[1].size, 12);
        let when = items[1].modified.unwrap();
        let secs = when
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(secs, 784_111_777);
    }
}
