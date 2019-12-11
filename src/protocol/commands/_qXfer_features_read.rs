#[derive(PartialEq, Eq, Debug)]
pub struct qXferFeaturesRead<'a> {
    pub annex: &'a str,
    pub offset: usize,
    pub len: usize,
}

impl<'a> qXferFeaturesRead<'a> {
    pub fn parse(body: &'a str) -> Result<Self, ()> {
        // body should be ":<target>:<offset>,<len>b"
        log::debug!("{}", body);
        if body.is_empty() {
            return Err(());
        }

        let mut body = body.split(':').skip(1);
        let annex = body.next().ok_or(())?;

        let mut body = body.next().ok_or(())?.split(',');
        let offset = usize::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;
        let len = usize::from_str_radix(body.next().ok_or(())?, 16).map_err(drop)?;

        Ok(qXferFeaturesRead { annex, offset, len })
    }
}
