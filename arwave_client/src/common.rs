use serde::Serialize;

#[derive(Default, Debug, Serialize)]
pub struct Tag {
    pub name: String,
    pub value: String,
}
