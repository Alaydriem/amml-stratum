use stratum_core::structs::nbt::{NbtCompound, NbtTag};

use super::NbtWriter;

// Builds `.mcstructure` bytes for test fixtures.
pub struct StructureFixture {
    size: [i32; 3],
    origin: Option<[i32; 3]>,
    // Palette entries: name and states in file order.
    palette: Vec<(String, Vec<(String, NbtTag)>)>,
    primary: Vec<i32>,
    secondary: Vec<i32>,
    // When set, each layer is `count` copies of one value (primary, secondary), written straight
    // to bytes instead of held as two integer vectors.
    fill: Option<(usize, i32, i32)>,
    // Writes block_indices as a list of two Int Arrays instead of two Int lists, as some exporters do.
    array_layers: bool,
    // Writes size and structure_world_origin as Int Arrays instead of Int lists.
    array_header: bool,
}

impl StructureFixture {
    // An empty box: every cell is "no block".
    pub fn new(size: [i32; 3]) -> Self {
        let volume = size.iter().fold(1usize, |acc, axis| acc.saturating_mul((*axis).max(0) as usize));
        // A box too big to allocate starts with empty layers; use raw_indices to fill them.
        let cells = if volume > 1 << 24 { 0 } else { volume };
        Self {
            size,
            origin: None,
            palette: Vec::new(),
            primary: vec![-1; cells],
            secondary: vec![-1; cells],
            fill: None,
            array_layers: false,
            array_header: false,
        }
    }

    // A box where every cell of the primary layer is one stone block. Cells cannot be changed after.
    pub fn full(size: [i32; 3]) -> Self {
        let volume = size.iter().map(|axis| *axis as usize).product::<usize>();
        let mut fixture = Self::new([1, 1, 1]).block(0, 0, 0, "minecraft:stone");
        fixture.size = size;
        fixture.primary = Vec::new();
        fixture.secondary = Vec::new();
        fixture.fill = Some((volume, 0, -1));
        fixture
    }

    // A box where every cell is stone in the primary layer and water in the secondary layer.
    pub fn full_waterlogged(size: [i32; 3]) -> Self {
        let mut fixture = Self::full(size);
        fixture.palette.push(("minecraft:water".to_string(), Vec::new()));
        fixture.fill = fixture.fill.map(|(count, primary, _)| (count, primary, 1));
        fixture
    }

    // Puts a new palette entry in one cell of the primary layer.
    pub fn block(mut self, x: usize, y: usize, z: usize, name: &str) -> Self {
        let index = self.cell(x, y, z);
        self.palette.push((name.to_string(), Vec::new()));
        self.primary[index] = (self.palette.len() - 1) as i32;
        self
    }

    // Puts a new palette entry in one cell of the secondary (waterlog) layer.
    pub fn waterlog(mut self, x: usize, y: usize, z: usize, name: &str) -> Self {
        let index = self.cell(x, y, z);
        self.palette.push((name.to_string(), Vec::new()));
        self.secondary[index] = (self.palette.len() - 1) as i32;
        self
    }

    // Adds `count` palette entries that no cell uses.
    pub fn unused_entries(mut self, count: usize) -> Self {
        self.palette.extend((0..count).map(|n| (format!("test:unused_{n}"), Vec::new())));
        self
    }

    // Adds a state to the most recent block.
    pub fn state(mut self, key: &str, value: NbtTag) -> Self {
        if let Some((_, states)) = self.palette.last_mut() {
            states.push((key.to_string(), value));
        }
        self
    }

    pub fn origin(mut self, origin: [i32; 3]) -> Self {
        self.origin = Some(origin);
        self
    }

    pub fn int_array_layers(mut self) -> Self {
        self.array_layers = true;
        self
    }

    pub fn int_array_header(mut self) -> Self {
        self.array_header = true;
        self
    }

    // Replaces both index layers as written, whatever the size says.
    pub fn raw_indices(mut self, primary: Vec<i32>, secondary: Vec<i32>) -> Self {
        self.primary = primary;
        self.secondary = secondary;
        self.fill = None;
        self
    }

    pub fn bytes(&self) -> Vec<u8> {
        let block_palette = NbtTag::List(
            self.palette
                .iter()
                .map(|(name, states)| {
                    NbtTag::Compound(NbtCompound::new(vec![
                        ("name".to_string(), NbtTag::String(name.clone())),
                        ("states".to_string(), NbtTag::Compound(NbtCompound::new(states.clone()))),
                        ("version".to_string(), NbtTag::Int(18_153_475)),
                    ]))
                })
                .collect(),
        );
        let default = NbtCompound::new(vec![
            ("block_palette".to_string(), block_palette),
            ("block_position_data".to_string(), NbtTag::Compound(NbtCompound::default())),
        ]);
        let palette = NbtCompound::new(vec![("default".to_string(), NbtTag::Compound(default))]);

        let cells = self.fill.map_or(self.primary.len() + self.secondary.len(), |(count, _, _)| count * 2);
        let mut out = Vec::with_capacity(cells * 4 + 4096);
        out.push(10);
        NbtWriter::write_string(&mut out, "").expect("fixture NBT is writable");
        NbtWriter::write_entry(&mut out, "format_version", &NbtTag::Int(1)).expect("fixture NBT is writable");
        NbtWriter::write_entry(&mut out, "size", &self.triple(self.size)).expect("fixture NBT is writable");

        out.push(10);
        NbtWriter::write_string(&mut out, "structure").expect("fixture NBT is writable");
        // block_indices: a list of two Int lists, or of two Int Arrays.
        out.push(9);
        NbtWriter::write_string(&mut out, "block_indices").expect("fixture NBT is writable");
        out.push(if self.array_layers { 11 } else { 9 });
        out.extend_from_slice(&2u32.to_le_bytes());
        match self.fill {
            Some((count, primary, secondary)) => {
                self.write_fill(&mut out, primary, count);
                self.write_fill(&mut out, secondary, count);
            }
            None => {
                self.write_layer(&mut out, &self.primary);
                self.write_layer(&mut out, &self.secondary);
            }
        }
        NbtWriter::write_entry(&mut out, "entities", &NbtTag::List(Vec::new())).expect("fixture NBT is writable");
        NbtWriter::write_entry(&mut out, "palette", &NbtTag::Compound(palette)).expect("fixture NBT is writable");
        out.push(0);

        if let Some(origin) = self.origin {
            NbtWriter::write_entry(&mut out, "structure_world_origin", &self.triple(origin))
                .expect("fixture NBT is writable");
        }
        out.push(0);
        out
    }

    fn cell(&self, x: usize, y: usize, z: usize) -> usize {
        (x * self.size[1] as usize + y) * self.size[2] as usize + z
    }

    fn triple(&self, values: [i32; 3]) -> NbtTag {
        if self.array_header { NbtTag::IntArray(values.to_vec()) } else { NbtTag::IntList(values.to_vec()) }
    }

    // One layer as a list element: an Int Array payload has no element id, an Int list has one.
    fn write_layer(&self, out: &mut Vec<u8>, values: &[i32]) {
        if !self.array_layers {
            out.push(3);
        }
        out.extend_from_slice(&(values.len() as u32).to_le_bytes());
        out.extend(values.iter().flat_map(|value| value.to_le_bytes()));
    }

    fn write_fill(&self, out: &mut Vec<u8>, value: i32, count: usize) {
        if self.array_layers {
            NbtWriter::write_int_fill(out, value, count).expect("fixture NBT is writable");
        } else {
            NbtWriter::write_int_list_fill(out, value, count).expect("fixture NBT is writable");
        }
    }
}
