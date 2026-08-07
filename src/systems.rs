use bevy::prelude::*;

use crate::{
    component::{TextMaterial, TextMesh, TextMeshComputed},
    slug,
};

pub fn compute_mesh_and_material(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut query: Query<
        (Entity, &TextMesh, &mut Mesh3d),
        Or<(Changed<TextMesh>, Without<TextMeshComputed>)>,
    >,
    mut text_materials: ResMut<Assets<TextMaterial>>,
    mut images: ResMut<Assets<Image>>,
    font_assets: Res<Assets<Font>>,
) {
    for (entity, text_mesh, mut mesh3d) in query.iter_mut() {
        let Some(font) = font_assets.get(&text_mesh.font) else {
            continue;
        };

        let Ok(face) = ttf_parser::Face::parse(font.data.data(), 0) else {
            continue;
        };

        let prepare_text = slug::prepare_text(&face, &text_mesh.text, text_mesh.size);

        let mesh = if text_mesh.billboard {
            // Realign the baseline-left layout to the requested anchor (pixel units).
            let units_per_em = face.units_per_em() as f32;
            let scale = if units_per_em > 0.0 {
                text_mesh.size / units_per_em
            } else {
                1.0
            };
            let width = prepare_text.total_advance * scale;
            let ascender = face.ascender() as f32 * scale;
            let descender = face.descender() as f32 * scale;
            let shift = text_mesh.anchor.shift(width, ascender, descender);
            prepare_text.mesh_offset(shift.x, shift.y)
        } else {
            prepare_text.mesh()
        };
        mesh3d.0 = meshes.add(mesh);

        commands
            .entity(entity)
            .insert(TextMeshComputed)
            .try_remove::<MeshMaterial3d<TextMaterial>>()
            .insert(MeshMaterial3d(text_materials.add(TextMaterial {
                curve_texture: images.add(prepare_text.curve()),
                band_texture: images.add(prepare_text.band()),
                color: text_mesh.color.to_linear(),
                bg_color: text_mesh.bg_color.to_linear(),
                billboard: text_mesh.billboard as u32,
                depth_test: text_mesh.depth_test,
            })));
    }
}
