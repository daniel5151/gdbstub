use core::convert::TryFrom;

#[derive(Debug)]
pub struct qSupported<'a> {
    features: core::str::Split<'a, char>,
}

impl<'a> TryFrom<&'a str> for qSupported<'a> {
    type Error = ();

    fn try_from(body: &'a str) -> Result<Self, ()> {
        if body.is_empty() {
            return Err(());
        }

        let features = body.split(';');
        Ok(qSupported { features })
    }
}

impl<'a> Iterator for qSupported<'a> {
    type Item = Feature<'a>;

    fn next(&mut self) -> Option<Feature<'a>> {
        let s = self.features.next()?;

        match s.as_bytes().last() {
            None => {
                // packet shouldn't have two ";;" in a row
                // FIXME: this should return an Error, not none!
                None
            }
            Some(&c) if c == b'+' || c == b'-' || c == b'?' => Some(Feature {
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
                Some(Feature {
                    name: parts.next().unwrap(),
                    // FIXME: this should return an Error, not none!
                    val: Some(parts.next()?),
                    status: FeatureSupported::Yes,
                })
            }
        }
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
