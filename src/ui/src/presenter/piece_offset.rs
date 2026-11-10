// Writes the offset of a piece from the build origin as the Paper artboard shows it: "+48, +0, +0".
pub struct PieceOffset;

impl PieceOffset {
    pub fn of(offset: [i32; 3]) -> String {
        format!("{:+}, {:+}, {:+}", offset[0], offset[1], offset[2])
    }
}
