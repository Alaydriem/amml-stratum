use super::{AimKind, FaceRule, Half};

// What `aim` leaves for `decide`. The strings are reused, so a call allocates nothing once warm.
#[derive(Debug, Clone)]
pub(crate) struct Aim {
    pub kind: AimKind,
    pub spot: [i32; 3],
    pub palette_id: u16,
    pub rule: FaceRule,
    pub half: Half,
    // The item name the spot needs.
    pub item: String,
    // The ghost block name that shows the placement worked.
    pub expected: String,
    // For a double slab, the single slab block name that also shows a placement worked. Empty for
    // any other block.
    pub slab: String,
}

impl Aim {
    const NAME_CAPACITY: usize = 64;

    pub fn new() -> Self {
        Self {
            kind: AimKind::Off,
            spot: [0; 3],
            palette_id: 0,
            rule: FaceRule::Any,
            half: Half::Any,
            item: String::with_capacity(Self::NAME_CAPACITY),
            expected: String::with_capacity(Self::NAME_CAPACITY),
            slab: String::with_capacity(Self::NAME_CAPACITY),
        }
    }

    pub fn set_target(&mut self, spot: [i32; 3], palette_id: u16, rule: FaceRule, half: Half) {
        self.spot = spot;
        self.palette_id = palette_id;
        self.rule = rule;
        self.half = half;
    }

    pub fn set_names(&mut self, item: &str, expected: &str, slab: &str) {
        self.item.clear();
        self.item.push_str(item);
        self.expected.clear();
        self.expected.push_str(expected);
        self.slab.clear();
        self.slab.push_str(slab);
    }
}

impl Default for Aim {
    fn default() -> Self {
        Self::new()
    }
}
