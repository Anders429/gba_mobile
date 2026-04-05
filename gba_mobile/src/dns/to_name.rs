pub trait ToName {
    fn to_name(&self) -> &[u8];
}

impl ToName for &str {
    fn to_name(&self) -> &[u8] {
        self.as_bytes()
    }
}
