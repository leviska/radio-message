#[derive(Debug, Copy, Clone)]
pub struct RequestMessage {
    pub from: u32,
    pub to: u32,
    pub start: u32,
    pub id: u32,
}

#[derive(Debug)]
pub enum MessageType<T> {
    Request(RequestMessage),
    Comm(T),
}

#[derive(Debug)]
pub struct Message<T> {
    pub data: MessageType<T>,
    pub from: u32,
    pub to: u32,
}
