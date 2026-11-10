use super::MaterialLine;

// Items to gather, sorted by count descending, then item id ascending.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MaterialList {
    lines: Vec<MaterialLine>,
}

impl MaterialList {
    // Adds the counts of equal items, then sorts the lines.
    pub fn new(mut lines: Vec<MaterialLine>) -> Self {
        lines.sort_by(|a, b| a.item.cmp(&b.item));
        let mut folded: Vec<MaterialLine> = Vec::with_capacity(lines.len());
        for line in lines {
            match folded.last_mut() {
                Some(last) if last.item == line.item => last.count = last.count.saturating_add(line.count),
                _ => folded.push(line),
            }
        }
        folded.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.item.cmp(&b.item)));
        Self { lines: folded }
    }

    pub fn lines(&self) -> &[MaterialLine] {
        &self.lines
    }

    pub fn total(&self) -> u64 {
        self.lines.iter().fold(0u64, |sum, line| sum.saturating_add(line.count))
    }

    // Adds the counts of equal items across lists.
    pub fn merge(lists: &[&MaterialList]) -> MaterialList {
        MaterialList::new(lists.iter().flat_map(|list| list.lines.iter().cloned()).collect())
    }
}
