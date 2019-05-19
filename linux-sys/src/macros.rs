macro_rules! check {
    ($e:expr) => {
        match $e as isize {
            e if e < 0 && e > -256 => Err(Error { code: -e as u8 }),
            x => Ok(x),
        }
    };
}
