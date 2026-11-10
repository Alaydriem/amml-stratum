#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StateValue {
    Bool(bool),
    Int(i32),
    Text(String),
}
