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
    /// translation is used as the anchor (rotation/scale ignored).
    pub billboard: bool,
    /// When `true` (the default) the text is depth-tested like ordinary geometry; when `false`
    /// it draws over everything. Turn it off for markers that must always be readable — note
    /// that a depth-tested billboard sits at one depth, so an occluder slices it mid-word
    /// rather than hiding it. To hide one cleanly, test its anchor and toggle `Visibility`.
    pub depth_test: bool,
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
            depth_test: true,
            anchor: TextAnchor::default(),
        }
    }
}

#[derive(Component)]
pub struct TextMeshComputed;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
#[bind_group_data(TextMaterialKey)]
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

    /// See [`TextMesh::depth_test`]. Not a uniform -- it selects a pipeline, not a shader path.
    pub depth_test: bool,
}

/// Pipeline key: depth comparison varies per label, so it must be known at specialization time
/// rather than only as a shader uniform.
#[repr(C)]
#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct TextMaterialKey {
    depth_test: bool,
}

impl From<&TextMaterial> for TextMaterialKey {
    fn from(material: &TextMaterial) -> Self {
        Self {
            depth_test: material.depth_test,
        }
    }
}

impl Material for TextMaterial {
    fn alpha_mode(&self) -> AlphaMode {
        // The pixel shader returns `mix(bg_color, color, coverage)`, i.e. colour already multiplied
        // by coverage (premultiplied alpha). Straight `Blend` would multiply by alpha a second time
        // and darken sub-pixel (small / thin) text; `Premultiplied` composites it correctly.
        AlphaMode::Premultiplied
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
        key: bevy::pbr::MaterialPipelineKey<Self>,
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

        // Read side only: `AlphaMode::Premultiplied` already renders in the transparent phase
        // with `depth_write_enabled = false`.
        if !key.bind_group_data.depth_test {
            if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
                depth_stencil.depth_compare =
                    Some(bevy::render::render_resource::CompareFunction::Always);
            }
        }

        Ok(())
    }
}
