use primitive_types::H160;

#[derive(Eq, PartialEq, Clone, Debug, Copy, Default)]
pub struct User {
    pub address: H160,
    pub user_id: u64,
}

impl User {
    #[allow(dead_code)]
    pub fn show_full_address(&self) -> String {
        let mut bytes = [0u8; 2 + 20 * 2];
        bytes[..2].copy_from_slice(b"0x");
        // Can only fail if the buffer size does not match but we know it is correct.
        hex::encode_to_slice(self.address, &mut bytes[2..]).unwrap();
        // Hex encoding is always valid utf8.
        let s = std::str::from_utf8(&bytes).unwrap();
        (*s).to_string()
    }
}
