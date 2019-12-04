#[derive(PartialEq, Eq, Debug)]
pub struct QSupported<'a>(Vec<Feature<'a>>);

impl<'a> QSupported<'a> {
    pub fn from_cmd_body(s: Option<&'a str>) -> Result<QSupported<'a>, ()> {
        let s = s.ok_or(())?; // can't have empty body

        let features = s
            .split(';')
            .map(|s| match s.as_bytes().last() {
                None => {
                    // packet shouldn't have two ";;" in a row
                    Err(())
                }
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
                        val: Some(parts.next().ok_or(())?),
                        status: FeatureSupported::Yes,
                    })
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(QSupported(features))
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
