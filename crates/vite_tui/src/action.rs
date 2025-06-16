#[expect(unused)]
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    Task { bytes: Box<[u8]> }, // Help,
}
