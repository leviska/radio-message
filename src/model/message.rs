#[derive(Debug)]
pub enum MessageType {
    Request(String),
    Comm(String),
}

#[derive(Debug)]
pub struct Message {
    pub data: MessageType,
    pub from: u32,
    pub to: u32,
}
