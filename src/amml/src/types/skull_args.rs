/// The arguments of `void SkullBlockRenderer::renderSkull(BaseActorRenderContext&, RenderMetadata const&, Vec3 const&
/// pos, int face, float rot, HashedString const& name, MaterialPtr const* forcedMat, ClientTexture const* forcedTex,
/// int breakingAmount, float animationValue, bool isGlint)`, `this` first. Pointers are kept as addresses; the
/// detour never reads through them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkullArgs {
    pub this: usize,
    pub ctx: usize,
    pub metadata: usize,
    pub position: usize,
    /// 1 on the floor, 2 to 5 on a wall, 0 the fallback wall face.
    pub face: i32,
    pub degrees: f32,
    /// `HashedString const*`: the skull block's full name, which picks the model.
    pub name: usize,
    pub material: usize,
    pub texture: usize,
    /// The whole stack slot of the breaking amount. The game's skull renderer leaves a stale value in its upper half,
    /// so it is forwarded as it came and never read.
    pub breaking: u64,
    pub animation: f32,
    pub glint: u8,
}
