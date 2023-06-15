#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Attachment {
    pub filename: Option<String>,
    pub mime: String,
    pub body: Vec<u8>,
}
