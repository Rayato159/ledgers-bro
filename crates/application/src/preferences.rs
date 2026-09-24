/// Display preferences belong to the local ledger, never to monetary values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserPreferences {
    pub english: bool,
    pub dark: bool,
    pub primary_color: u32,
    pub gradient: bool,
}
impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            english: false,
            dark: false,
            primary_color: 0xbda0ff,
            gradient: true,
        }
    }
}
