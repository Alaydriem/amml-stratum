use std::collections::VecDeque;

use crate::errors::BuildError;

// The commands a build wants sent, until the mod takes them. Bound: `capacity` commands; a push past it is
// refused, nothing is dropped.
#[derive(Debug)]
pub struct BuildOutbox {
    capacity: usize,
    commands: VecDeque<String>,
}

impl BuildOutbox {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self { capacity, commands: VecDeque::with_capacity(capacity) }
    }

    pub fn push(&mut self, command: String) -> Result<(), BuildError> {
        if self.commands.len() >= self.capacity {
            return Err(BuildError::OutboxFull);
        }
        self.commands.push_back(command);
        Ok(())
    }

    // Moves every waiting command to `out`, oldest first.
    pub fn drain(&mut self, out: &mut Vec<String>) {
        out.extend(self.commands.drain(..));
    }

    // Drops every waiting command.
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    // Commands that still fit.
    pub fn free(&self) -> usize {
        self.capacity - self.commands.len()
    }
}
