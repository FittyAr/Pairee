//! `ListObjectsV2` answers, read leniently: S3-compatible stores leave out
//! fields AWS always sends (ETag, owner, storage class), so only the key,
//! size, time, common prefixes and continuation token are looked at.

use crate::fs::remote::xml::reference_text;
use quick_xml::events::Event;

/// One object of a listing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Object {
    pub key: String,
    pub size: u64,
    pub last_modified: String,
}

/// One page of a listing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Page {
    pub objects: Vec<Object>,
    pub prefixes: Vec<String>,
    pub next_token: Option<String>,
}

/// The element a text node belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    None,
    Key,
    Size,
    Modified,
    Prefix,
    Token,
}

/// The page in the `ListBucketResult` document `xml`.
pub fn parse(xml: &str) -> Result<Page, quick_xml::Error> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut page = Page::default();
    let mut object: Option<Object> = None;
    let mut in_prefixes = false;
    let mut field = Field::None;
    let mut text = String::new();
    loop {
        match reader.read_event()? {
            Event::Start(e) => {
                text.clear();
                field = match e.local_name().as_ref() {
                    "Contents" => {
                        object = Some(Object::default());
                        Field::None
                    }
                    "CommonPrefixes" => {
                        in_prefixes = true;
                        Field::None
                    }
                    "Key" => Field::Key,
                    "Size" => Field::Size,
                    "LastModified" => Field::Modified,
                    "Prefix" if in_prefixes => Field::Prefix,
                    "NextContinuationToken" => Field::Token,
                    _ => Field::None,
                };
            }
            Event::Text(t) => text.push_str(&t.xml10_content()),
            Event::GeneralRef(r) => text.push_str(&reference_text(&r)?),
            Event::End(e) => {
                let value = std::mem::take(&mut text);
                store(&mut page, object.as_mut(), field, value);
                field = Field::None;
                match e.local_name().as_ref() {
                    "Contents" => page.objects.extend(object.take()),
                    "CommonPrefixes" => in_prefixes = false,
                    _ => {}
                }
            }
            Event::Eof => return Ok(page),
            _ => {}
        }
    }
}

fn store(page: &mut Page, object: Option<&mut Object>, field: Field, value: String) {
    match (field, object) {
        (Field::Key, Some(o)) => o.key = value,
        (Field::Size, Some(o)) => o.size = value.trim().parse().unwrap_or(0),
        (Field::Modified, Some(o)) => o.last_modified = value,
        (Field::Prefix, _) => page.prefixes.push(value),
        (Field::Token, _) => page.next_token = Some(value).filter(|t| !t.is_empty()),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_objects_prefixes_and_the_next_token() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Name>bucket</Name><Prefix>dir/</Prefix>
  <Contents><Key>dir/a&amp;b.txt</Key><LastModified>2009-10-12T17:50:30.000Z</LastModified><Size>5</Size></Contents>
  <CommonPrefixes><Prefix>dir/sub/</Prefix></CommonPrefixes>
  <NextContinuationToken>abc</NextContinuationToken>
</ListBucketResult>"#;
        let page = parse(xml).unwrap();
        assert_eq!(
            page.objects,
            [Object {
                key: "dir/a&b.txt".into(),
                size: 5,
                last_modified: "2009-10-12T17:50:30.000Z".into(),
            }]
        );
        assert_eq!(page.prefixes, ["dir/sub/"]);
        assert_eq!(page.next_token.as_deref(), Some("abc"));
    }
}
