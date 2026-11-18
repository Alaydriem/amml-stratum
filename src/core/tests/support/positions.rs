use stratum_core::structs::build::BuildStep;

// Reads the positions of build steps.
pub struct Positions;

impl Positions {
    pub fn of(steps: Vec<BuildStep>) -> Vec<[i32; 3]> {
        steps.into_iter().map(|step| step.position).collect()
    }

    // True when two positions share a face.
    pub fn touch(a: [i32; 3], b: [i32; 3]) -> bool {
        a.iter().zip(b.iter()).map(|(left, right)| (left - right).abs()).sum::<i32>() == 1
    }

    // How many steps in a row do not touch the step before.
    pub fn jumps(order: &[[i32; 3]]) -> usize {
        order.windows(2).filter(|pair| !Self::touch(pair[0], pair[1])).count()
    }
}
