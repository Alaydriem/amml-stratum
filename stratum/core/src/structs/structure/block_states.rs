use super::StateValue;

// Block state pairs in the order they appear in the file.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct BlockStates {
    pairs: Vec<(String, StateValue)>,
}

impl BlockStates {
    pub fn new(pairs: Vec<(String, StateValue)>) -> Self {
        Self { pairs }
    }

    // First pair with this name.
    pub fn get(&self, name: &str) -> Option<&StateValue> {
        self.pairs.iter().find(|(key, _)| key == name).map(|(_, value)| value)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &StateValue)> {
        self.pairs.iter().map(|(key, value)| (key.as_str(), value))
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}
