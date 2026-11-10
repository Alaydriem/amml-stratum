// Writes a number with a comma between each group of three digits, as the Paper artboards show counts.
pub struct DigitGroups;

impl DigitGroups {
    pub fn of(value: u64) -> String {
        let digits = value.to_string();
        let mut out = String::with_capacity(digits.len() + digits.len() / 3);
        for (index, digit) in digits.chars().enumerate() {
            if index > 0 && (digits.len() - index).is_multiple_of(3) {
                out.push(',');
            }
            out.push(digit);
        }
        out
    }
}
