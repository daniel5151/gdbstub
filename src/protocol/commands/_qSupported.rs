use super::prelude::*;

#[derive(Debug)]
pub struct qSupported<'a> {
    pub features: Features<'a>,
}

impl<'a> ParseCommand<'a> for qSupported<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        if body.is_empty() {
            return None;
        }

        Some(qSupported {
            features: Features(body),
        })
    }
}

#[derive(Debug)]
pub struct Features<'a>(&'a str);

impl<'a> Features<'a> {
    pub fn into_iter(self) -> impl Iterator<Item = Result<Feature<'a>, &'static str>> + 'a {
        self.0.split(';').map(|s| match s.as_bytes().last() {
            None => Err("packet shouldn't have two ';' in a row"),
            Some(&c) if c == b'+' || c == b'-' || c == b'?' => Ok(Feature {
                name: &s[..s.len() - 1],
                val: None,
                status: match c {
                    b'+' => FeatureSupported::Yes,
                    b'-' => FeatureSupported::No,
                    b'?' => FeatureSupported::Maybe,
                    _ => unreachable!(),
                },
            }),
            Some(_) => {
                let mut parts = s.split('=');
                Ok(Feature {
                    name: parts.next().unwrap(),
                    val: Some(parts.next().ok_or("missing feature val")?),
                    status: FeatureSupported::Yes,
                })
            }
        })
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum FeatureSupported {
    Yes,
    No,
    Maybe,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Feature<'a> {
    name: &'a str,
    val: Option<&'a str>,
    status: FeatureSupported,
}
