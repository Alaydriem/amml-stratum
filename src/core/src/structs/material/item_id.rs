// A namespaced item id such as `minecraft:oak_sign`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(pub String);

impl ItemId {
    // The id without its namespace, for example `stone_bricks`.
    pub fn short_name(&self) -> &str {
        self.0.split_once(':').map_or(self.0.as_str(), |(_, name)| name)
    }

    // A display name: `minecraft:stone_bricks` becomes `Stone Bricks`.
    pub fn readable(&self) -> String {
        let mut out = String::with_capacity(self.0.len());
        for (position, word) in self.short_name().split('_').filter(|word| !word.is_empty()).enumerate() {
            if position > 0 {
                out.push(' ');
            }
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                out.extend(first.to_uppercase());
                out.push_str(chars.as_str());
            }
        }
        out
    }
}
