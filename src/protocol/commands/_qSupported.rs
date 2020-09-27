use super::prelude::*;

#[derive(Debug)]
pub struct qSupported<'a> {
    pub features: Features<'a>,
}

impl<'a> ParseCommand<'a> for qSupported<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        if body.is_empty() {
            return None;
        }

        Some(qSupported {
            features: Features(body),
        })
    }
}

#[derive(Debug)]
pub struct Features<'a>(&'a [u8]);

impl<'a> Features<'a> {
    pub fn into_iter(self) -> impl Iterator<Item = Option<Feature<'a>>> + 'a {
        self.0.split(|b| *b == b';').map(|s| match s.last() {
            None => None,
            Some(&c) if c == b'+' || c == b'-' || c == b'?' => Some(Feature {
                name: s[..s.len() - 1].into(),
                val: None,
                status: match c {
                    b'+' => FeatureSupported::Yes,
                    b'-' => FeatureSupported::No,
                    b'?' => FeatureSupported::Maybe,
                    _ => return None,
                },
            }),
            Some(_) => {
                let mut parts = s.split(|b| *b == b'=');
                Some(Feature {
                    name: parts.next()?.into(),
                    val: Some(parts.next()?.into()),
                    status: FeatureSupported::Yes,
                })
            }
        })
    }
}

#[derive(Debug)]
pub enum FeatureSupported {
    Yes,
    No,
    Maybe,
}

#[derive(Debug)]
pub struct Feature<'a> {
    name: Bstr<'a>,
    val: Option<Bstr<'a>>,
    status: FeatureSupported,
}
