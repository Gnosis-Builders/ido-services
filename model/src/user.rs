use primitive_types::H160;

#[derive(Eq, PartialEq, Clone, Debug, Copy, Default)]
pub struct User {
    pub address: H160,
    pub user_id: u64,
}
