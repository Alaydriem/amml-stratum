use crate::game::{OwnedMesh, Tessellator};

/// An open begin on the game's tessellator. `finish` ends it into a mesh, `discard` ends it and drops what it made;
/// a drop without either (a fill that panicked) discards too, so the game's tessellator is never left in the
/// tessellating state.
pub struct OpenBuild<'a> {
    tessellator: &'a mut Tessellator,
    debug_name: &'a str,
    open: bool,
}

impl<'a> OpenBuild<'a> {
    pub fn new(tessellator: &'a mut Tessellator, debug_name: &'a str) -> Self {
        Self { tessellator, debug_name, open: true }
    }

    pub fn tessellator(&mut self) -> &mut Tessellator {
        self.tessellator
    }

    /// Ends the begin and gives the mesh, uploaded on its first draw; `None` when the begin was closed already.
    pub fn finish(mut self) -> Option<OwnedMesh> {
        self.open = false;
        self.tessellator.end(self.debug_name)
    }

    /// Ends the begin, if it is still open, into a mesh that is never uploaded and dropped at once.
    pub fn discard(mut self) {
        self.open = false;
        drop(self.tessellator.end_unuploaded(self.debug_name));
    }
}

impl Drop for OpenBuild<'_> {
    fn drop(&mut self) {
        if self.open {
            drop(self.tessellator.end_unuploaded(self.debug_name));
        }
    }
}
