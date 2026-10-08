//! Name / extension masks of the multi-rename tool.
//!
//! A mask is literal text with bracketed placeholders:
//!
//! | Placeholder | Meaning |
//! |---|---|
//! | `[N]` | original name without extension |
//! | `[N2]` | 2nd character of the name |
//! | `[N2-5]` | characters 2 to 5 |
//! | `[N2-]` | from the 2nd character to the end |
//! | `[N2,3]` | 3 characters starting at the 2nd |
//! | `[E]` | original extension (same ranges as `[N]`) |
//! | `[P]` | parent folder name (same ranges as `[N]`) |
//! | `[C]` | counter |
//! | `[Y]` `[M]` `[D]` | modification year (4 digits), month, day |
//! | `[h]` `[m]` `[s]` | modification hour, minute, second |
//!
//! Character positions are 1-based. Anything that is not a valid
//! placeholder (including an unclosed `[`) is copied literally.

/// Text field a sliceable placeholder reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Name,
    Ext,
    Parent,
}

/// Part of the modification date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatePart {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
}

/// Character range of a sliced placeholder (1-based, inclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharRange {
    pub start: usize,
    /// Last character (inclusive); `None` means "to the end".
    pub end: Option<usize>,
}

impl CharRange {
    /// The characters of `text` inside the range (empty when out of bounds).
    pub fn slice(self, text: &str) -> String {
        let skip = self.start.saturating_sub(1);
        let take = match self.end {
            Some(end) if end >= self.start => end - self.start + 1,
            Some(_) => 0,
            None => usize::MAX,
        };
        text.chars().skip(skip).take(take).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Literal(String),
    Text(Field, Option<CharRange>),
    Counter,
    Date(DatePart),
}

/// Values a mask is rendered with, for one file.
#[derive(Debug, Clone, Default)]
pub struct MaskValues<'a> {
    pub name: &'a str,
    pub ext: &'a str,
    pub parent: &'a str,
    /// Already formatted counter (padding applied).
    pub counter: &'a str,
    /// Modification date, `None` when unknown (date placeholders render empty).
    pub date: Option<DateValues>,
}

/// Modification date split into its components (local time).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateValues {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl DateValues {
    /// Local-time components of `time`.
    pub fn from_system_time(time: std::time::SystemTime) -> Self {
        use chrono::{Datelike, Timelike};
        let local: chrono::DateTime<chrono::Local> = time.into();
        Self {
            year: local.year(),
            month: local.month(),
            day: local.day(),
            hour: local.hour(),
            minute: local.minute(),
            second: local.second(),
        }
    }

    fn format(self, part: DatePart) -> String {
        match part {
            DatePart::Year => format!("{:04}", self.year),
            DatePart::Month => format!("{:02}", self.month),
            DatePart::Day => format!("{:02}", self.day),
            DatePart::Hour => format!("{:02}", self.hour),
            DatePart::Minute => format!("{:02}", self.minute),
            DatePart::Second => format!("{:02}", self.second),
        }
    }
}

/// A parsed mask.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Mask {
    tokens: Vec<Token>,
}

impl Mask {
    pub fn parse(mask: &str) -> Self {
        let mut tokens = Vec::new();
        let mut literal = String::new();
        let mut rest = mask;
        while let Some(open) = rest.find('[') {
            literal.push_str(&rest[..open]);
            let after = &rest[open + 1..];
            let parsed = after
                .find(']')
                .and_then(|close| parse_placeholder(&after[..close]).map(|t| (t, close)));
            match parsed {
                Some((token, close)) => {
                    if !literal.is_empty() {
                        tokens.push(Token::Literal(std::mem::take(&mut literal)));
                    }
                    tokens.push(token);
                    rest = &after[close + 1..];
                }
                None => {
                    literal.push('[');
                    rest = after;
                }
            }
        }
        literal.push_str(rest);
        if !literal.is_empty() {
            tokens.push(Token::Literal(literal));
        }
        Self { tokens }
    }

    pub fn render(&self, values: &MaskValues) -> String {
        let mut out = String::new();
        for token in &self.tokens {
            match token {
                Token::Literal(text) => out.push_str(text),
                Token::Text(field, range) => {
                    let text = match field {
                        Field::Name => values.name,
                        Field::Ext => values.ext,
                        Field::Parent => values.parent,
                    };
                    match range {
                        Some(range) => out.push_str(&range.slice(text)),
                        None => out.push_str(text),
                    }
                }
                Token::Counter => out.push_str(values.counter),
                Token::Date(part) => {
                    if let Some(date) = values.date {
                        out.push_str(&date.format(*part));
                    }
                }
            }
        }
        out
    }
}

/// Parses the text between `[` and `]`.
fn parse_placeholder(body: &str) -> Option<Token> {
    let mut chars = body.chars();
    let head = chars.next()?;
    let spec = chars.as_str();
    let field = match head {
        'N' => Field::Name,
        'E' => Field::Ext,
        'P' => Field::Parent,
        _ if !spec.is_empty() => return None,
        'C' => return Some(Token::Counter),
        _ => return date_part(head).map(Token::Date),
    };
    if spec.is_empty() {
        return Some(Token::Text(field, None));
    }
    parse_range(spec).map(|range| Token::Text(field, Some(range)))
}

fn date_part(c: char) -> Option<DatePart> {
    Some(match c {
        'Y' => DatePart::Year,
        'M' => DatePart::Month,
        'D' => DatePart::Day,
        'h' => DatePart::Hour,
        'm' => DatePart::Minute,
        's' => DatePart::Second,
        _ => return None,
    })
}

/// `x`, `x-y`, `x-` or `x,len` (1-based).
fn parse_range(spec: &str) -> Option<CharRange> {
    let number = |s: &str| s.parse::<usize>().ok().filter(|n| *n > 0);
    if let Some((start, len)) = spec.split_once(',') {
        let start = number(start)?;
        let len = number(len)?;
        return Some(CharRange {
            start,
            end: Some(start + len - 1),
        });
    }
    if let Some((start, end)) = spec.split_once('-') {
        let start = number(start)?;
        let end = if end.is_empty() {
            None
        } else {
            Some(number(end)?)
        };
        return Some(CharRange { start, end });
    }
    let start = number(spec)?;
    Some(CharRange {
        start,
        end: Some(start),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_tokens() {
        let mask = Mask::parse("IMG_[C]_[N2-]x");
        assert_eq!(
            mask.tokens,
            [
                Token::Literal("IMG_".into()),
                Token::Counter,
                Token::Literal("_".into()),
                Token::Text(
                    Field::Name,
                    Some(CharRange {
                        start: 2,
                        end: None
                    })
                ),
                Token::Literal("x".into()),
            ]
        );
        assert_eq!(Mask::parse("").tokens, []);
    }
}
