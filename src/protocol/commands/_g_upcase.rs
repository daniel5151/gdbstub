#[derive(PartialEq, Eq, Debug)]
pub struct G {
    pub vals: Vec<u8>,
}

impl G {
    pub fn parse(body: &str) -> Result<Self, ()> {
        if body.len() % 2 != 0 || !body.is_ascii() {
            return Err(());
        }

        let vals = body
            .as_bytes()
            .chunks_exact(2)
            .map(|c| unsafe { core::str::from_utf8_unchecked(c) })
            .map(|c| u8::from_str_radix(c, 16))
            .collect::<Result<Vec<_>, _>>()
            .map_err(drop)?;

        Ok(G { vals })
    }
}
