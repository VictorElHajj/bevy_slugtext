use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

use crate::slug;

#[derive(Component)]
#[require(Mesh3d)]
pub struct TextMesh {
    pub text: String,
    pub font: Handle<Font>,
    pub color: Color,
    pub bg_color: Color,
    pub size: f32,
    /// When `true`, the text renders as a camera-facing billboard with a constant
    /// on-screen size: `size` is interpreted as the em-height in pixels, the entity's
    /// translation is used as the anchor (rotation/scale ignored), and the label is
    /// depth-tested at that anchor so it can be occluded by nearer geometry.
    pub billboard: bool,
    /// Billboard-only. Which point of the text box is aligned to the entity's anchor. Ignored
    /// for world-space text (which is always laid out baseline-left from the origin).
    pub anchor: TextAnchor,
}

/// Which point of a text box is placed at the anchor. Vertical positions are relative to the font's
/// ascender/descender; `Baseline` is the text baseline.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum TextAnchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    #[default]
    BaselineLeft,
    BaselineCenter,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl TextAnchor {
    /// Pixel shift added to the baseline-left mesh so `self` lands on the origin, given the text's
    /// pixel `width`, `ascender`, and `descender` (`descender` is negative, `+y` is up).
    pub fn shift(self, width: f32, ascender: f32, descender: f32) -> Vec2 {
        use TextAnchor::*;
        let x = match self {
            TopLeft | CenterLeft | BaselineLeft | BottomLeft => 0.0,
            TopCenter | Center | BaselineCenter | BottomCenter => -width * 0.5,
            TopRight | CenterRight | BottomRight => -width,
        };
        let y = match self {
            TopLeft | TopCenter | TopRight => -ascender,
            CenterLeft | Center | CenterRight => -(ascender + descender) * 0.5,
            BaselineLeft | BaselineCenter => 0.0,
            BottomLeft | BottomCenter | BottomRight => -descender,
        };
        Vec2::new(x, y)
    }
}

impl Default for TextMesh {
    fn default() -> Self {
        Self {
            text: "".to_string(),
            font: Handle::default(),
            color: Color::BLACK,
            bg_color: Color::BLACK.with_alpha(0.0),
            size: 1.0,
            billboard: false,
            anchor: TextAnchor::default(),
        }
    }
}

#[derive(Component)]
pub struct TextMeshComputed;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub struct TextMaterial {
    #[texture(100, dimension = "2d", sample_type = "float")]
    pub curve_texture: Handle<Image>,

    #[texture(101, dimension = "2d", sample_type = "u_int")]
    pub band_texture: Handle<Image>,

    #[uniform(102)]
    pub color: LinearRgba,

    #[uniform(103)]
    pub bg_color: LinearRgba,

    /// 0 = world-space text, 1 = camera-facing constant-size billboard.
    #[uniform(104)]
    pub billboard: u32,
}

impl Material for TextMaterial {
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn fragment_shader() -> ShaderRef {
        "embedded://bevy_slugtext/shaders/SlugPixel.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "embedded://bevy_slugtext/shaders/SlugVertex.wgsl".into()
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            slug::ATTRIBUTE_SLUG_POS.at_shader_location(8),
            slug::ATTRIBUTE_SLUG_TEX.at_shader_location(9),
            slug::ATTRIBUTE_SLUG_JAC.at_shader_location(10),
            slug::ATTRIBUTE_SLUG_BND.at_shader_location(11),
        ])?;

        descriptor.vertex.buffers = vec![vertex_layout];
        descriptor.primitive.cull_mode = None;

        Ok(())
    }
}
