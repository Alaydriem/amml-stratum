// One of eight ghost opacity steps. Step 0 is the most see-through, step 7 is solid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpacityStep(u8);

impl OpacityStep {
    pub const COUNT: u8 = 8;
    pub const VALUES: [f32; 8] = [0.30, 0.40, 0.50, 0.60, 0.70, 0.80, 0.90, 1.00];
    const LAST: u8 = Self::COUNT - 1;

    // Steps past the last one become the last one.
    pub fn new(raw: u8) -> Self {
        Self(raw.min(Self::LAST))
    }

    pub fn index(self) -> u8 {
        self.0
    }

    pub fn alpha(self) -> f32 {
        Self::VALUES[usize::from(self.0)]
    }

    // Past the last step comes the first one.
    pub fn up(self) -> Self {
        Self((self.0 + 1) % Self::COUNT)
    }

    // Before the first step comes the last one.
    pub fn down(self) -> Self {
        Self((self.0 + Self::LAST) % Self::COUNT)
    }
}
