//! GLB file loading and parsing for VZGLYD.
//!
//! This module provides functionality to load and parse GLB (binary glTF) files
//! and convert them into imported scene representations that can be compiled
//! into slide specs.

use glam::{Mat3, Mat4, Vec3};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::path::{Path, PathBuf};
use thiserror::Error;
use vzglyd_slide::PipelineKind;

/// Errors that can occur during GLB loading.
#[derive(Debug, Error)]
pub enum GlbError {
    /// Failed to read the GLB file.
    #[error("failed to read GLB file '{0}': {1}")]
    ReadError(String, String),
    /// Failed to parse GLB data.
    #[error("failed to parse GLB '{0}': {1}")]
    ParseError(String, String),
    /// GLB format error (missing blob, external buffers, etc.).
    #[error("GLB format error: {0}")]
    FormatError(String),
    /// Unsupported GLB feature.
    #[error("unsupported GLB feature: {0}")]
    Unsupported(String),
}

/// A vertex imported from a GLB file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImportedVertex {
    /// Position in 3D space.
    pub position: [f32; 3],
    /// Normal vector (may be computed if not provided).
    pub normal: Option<[f32; 3]>,
    /// Texture coordinates (UV).
    pub tex_coords: Option<[f32; 2]>,
    /// Vertex color (RGBA).
    pub color: Option<[f32; 4]>,
}

/// A mesh imported from a GLB file.
#[derive(Debug, Clone)]
pub struct ImportedMesh {
    /// Vertices of the mesh.
    pub vertices: Vec<ImportedVertex>,
    /// Indices for the mesh.
    pub indices: Vec<u16>,
}

/// Extra metadata imported from GLB extras.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportedExtras {
    /// Raw extras JSON map.
    pub raw: JsonMap<String, JsonValue>,
    /// VZGLYD ID from extras.
    pub vzglyd_id: Option<String>,
    /// VZGLYD pipeline hint from extras.
    pub vzglyd_pipeline: Option<String>,
    /// VZGLYD material class hint from extras.
    pub vzglyd_material: Option<String>,
    /// VZGLYD anchor tag from extras.
    pub vzglyd_anchor: Option<String>,
    /// Whether the node is tagged as an anchor.
    pub vzglyd_anchor_tagged: bool,
    /// Whether the node is hidden.
    pub vzglyd_hidden: bool,
    /// Whether the node should billboard.
    pub vzglyd_billboard: bool,
    /// Whether this is the entry camera.
    pub vzglyd_entry_camera: bool,
}

/// Metadata for an imported scene.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedSceneMetadata {
    /// Scene name from GLB.
    pub scene_name: Option<String>,
    /// Extra metadata.
    pub extras: ImportedExtras,
}

/// Material imported from a GLB scene.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedSceneMaterial {
    /// Material name.
    pub name: Option<String>,
    /// Base color factor (RGBA).
    pub base_color_factor: [f32; 4],
    /// Material class hint.
    pub class_hint: Option<String>,
    /// Extra metadata.
    pub metadata: ImportedExtras,
}

/// A mesh node in an imported scene.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedSceneMeshNode {
    /// Unique ID for the mesh node.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Name of the parent node.
    pub node_name: Option<String>,
    /// Name of the mesh.
    pub mesh_name: Option<String>,
    /// Index of the node in the GLB.
    pub node_index: usize,
    /// Index of the primitive in the mesh.
    pub primitive_index: usize,
    /// World transform matrix.
    pub world_transform: [[f32; 4]; 4],
    /// Vertices of the mesh.
    pub vertices: Vec<ImportedVertex>,
    /// Indices of the mesh.
    pub indices: Vec<u16>,
    /// Material of the mesh.
    pub material: ImportedSceneMaterial,
    /// Extra metadata.
    pub metadata: ImportedExtras,
}

/// Camera projection type.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportedCameraProjection {
    /// Perspective camera.
    Perspective {
        /// Aspect ratio.
        aspect_ratio: Option<f32>,
        /// Vertical field of view in radians.
        yfov_rad: f32,
        /// Near plane distance.
        znear: f32,
        /// Far plane distance.
        zfar: Option<f32>,
    },
    /// Orthographic camera.
    Orthographic {
        /// Horizontal magnification.
        xmag: f32,
        /// Vertical magnification.
        ymag: f32,
        /// Near plane distance.
        znear: f32,
        /// Far plane distance.
        zfar: f32,
    },
}

/// A camera imported from a GLB scene.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedSceneCamera {
    /// Unique ID for the camera.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Name of the parent node.
    pub node_name: Option<String>,
    /// Name of the camera.
    pub camera_name: Option<String>,
    /// Index of the node in the GLB.
    pub node_index: usize,
    /// World transform matrix.
    pub world_transform: [[f32; 4]; 4],
    /// Camera projection.
    pub projection: ImportedCameraProjection,
    /// Extra metadata.
    pub metadata: ImportedExtras,
}

/// An anchor point imported from a GLB scene.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedSceneAnchor {
    /// Unique ID for the anchor.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Name of the parent node.
    pub node_name: Option<String>,
    /// Index of the node in the GLB.
    pub node_index: usize,
    /// World transform matrix.
    pub world_transform: [[f32; 4]; 4],
    /// Extra metadata.
    pub metadata: ImportedExtras,
}

/// A directional light imported from a GLB scene.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedSceneDirectionalLight {
    /// Unique ID for the light.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Name of the parent node.
    pub node_name: Option<String>,
    /// Name of the light.
    pub light_name: Option<String>,
    /// Index of the node in the GLB.
    pub node_index: usize,
    /// World transform matrix.
    pub world_transform: [[f32; 4]; 4],
    /// Light direction.
    pub direction: [f32; 3],
    /// Light color (RGB).
    pub color: [f32; 3],
    /// Light intensity.
    pub intensity: f32,
    /// Extra metadata.
    pub metadata: ImportedExtras,
}

/// A scene imported from a GLB file.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedScene {
    /// Unique ID for the scene.
    pub id: String,
    /// Source path of the GLB file.
    pub source_path: PathBuf,
    /// Human-readable label.
    pub label: Option<String>,
    /// Entry camera selector.
    pub entry_camera: Option<String>,
    /// Compile profile hint.
    pub compile_profile: Option<String>,
    /// Scene metadata.
    pub metadata: ImportedSceneMetadata,
    /// Mesh nodes in the scene.
    pub mesh_nodes: Vec<ImportedSceneMeshNode>,
    /// Cameras in the scene.
    pub cameras: Vec<ImportedSceneCamera>,
    /// Anchor points in the scene.
    pub anchors: Vec<ImportedSceneAnchor>,
    /// Directional lights in the scene.
    pub directional_lights: Vec<ImportedSceneDirectionalLight>,
    /// Warnings generated during import.
    pub warnings: Vec<String>,
}

/// Reference to a scene asset in a manifest.
pub struct SceneAssetRef {
    /// Path to the scene asset.
    pub path: String,
    /// Optional ID for the scene.
    pub id: Option<String>,
    /// Optional label for the scene.
    pub label: Option<String>,
    /// Optional entry camera selector.
    pub entry_camera: Option<String>,
    /// Optional compile profile.
    pub compile_profile: Option<String>,
}

impl SceneAssetRef {
    /// Create a new scene asset reference.
    pub fn new(path: String) -> Self {
        Self {
            path,
            id: None,
            label: None,
            entry_camera: None,
            compile_profile: None,
        }
    }
}

/// Load a GLB mesh from a file path.
pub fn load_glb_mesh(path: &Path) -> Result<ImportedMesh, GlbError> {
    let scene = load_glb_scene(path, None)?;
    flatten_scene_mesh_nodes(&scene, path)
}

/// Load a GLB scene from a file path with optional selector.
pub fn load_glb_scene(
    path: &Path,
    selector: Option<&SceneAssetRef>,
) -> Result<ImportedScene, GlbError> {
    if path.extension().and_then(|ext| ext.to_str()) != Some("glb") {
        return Err(GlbError::FormatError(format!(
            "unsupported scene format for '{}': only .glb scene assets are supported",
            path.display()
        )));
    }

    let bytes = std::fs::read(path).map_err(|error| {
        GlbError::ReadError(
            path.display().to_string(),
            format!("failed to read scene '{}': {error}", path.display()),
        )
    })?;
    let gltf = gltf::Gltf::from_slice(&bytes).map_err(|error| {
        GlbError::ParseError(path.display().to_string(), error.to_string())
    })?;
    let blob = gltf.blob.as_deref().ok_or_else(|| {
        GlbError::FormatError(format!(
            "GLB '{}' is missing its binary buffer chunk",
            path.display()
        ))
    })?;

    for buffer in gltf.document.buffers() {
        if !matches!(buffer.source(), gltf::buffer::Source::Bin) {
            return Err(GlbError::FormatError(format!(
                "GLB '{}' references an external buffer; package scene assets must be self-contained",
                path.display()
            )));
        }
    }

    let gltf_scene = gltf
        .document
        .default_scene()
        .or_else(|| gltf.document.scenes().next())
        .ok_or_else(|| {
            GlbError::FormatError(format!(
                "GLB '{}' does not declare a scene to import",
                path.display()
            ))
        })?;

    let scene_name = gltf_scene.name().map(str::to_owned);
    let file_stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_owned);
    let scene_id = selector
        .and_then(|scene| scene.id.clone())
        .or_else(|| scene_name.clone())
        .or_else(|| file_stem.clone())
        .unwrap_or_else(|| "scene".into());

    let mut imported = ImportedScene {
        id: scene_id.clone(),
        source_path: path.to_path_buf(),
        label: selector
            .and_then(|scene| scene.label.clone())
            .or_else(|| scene_name.clone())
            .or_else(|| file_stem.clone()),
        entry_camera: selector.and_then(|scene| scene.entry_camera.clone()),
        compile_profile: selector
            .and_then(|scene| scene.compile_profile.clone())
            .or_else(|| Some("default_world".into())),
        metadata: ImportedSceneMetadata {
            scene_name,
            extras: ImportedExtras::default(),
        },
        mesh_nodes: Vec::new(),
        cameras: Vec::new(),
        anchors: Vec::new(),
        directional_lights: Vec::new(),
        warnings: Vec::new(),
    };
    imported.metadata.extras = parse_imported_extras(
        gltf_scene.extras(),
        &format!("scene '{}'", imported.id),
        &mut imported.warnings,
    );

    for node in gltf_scene.nodes() {
        append_glb_scene_node(&mut imported, node, Mat4::IDENTITY, blob, path)?;
    }

    Ok(imported)
}

/// Flatten all mesh nodes in a scene into a single mesh.
fn flatten_scene_mesh_nodes(scene: &ImportedScene, path: &Path) -> Result<ImportedMesh, GlbError> {
    if scene.mesh_nodes.is_empty() {
        return Err(GlbError::FormatError(format!(
            "GLB '{}' did not produce any triangle geometry",
            path.display()
        )));
    }

    let mut imported = ImportedMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    for mesh_node in &scene.mesh_nodes {
        let vertex_offset = imported.vertices.len();
        if vertex_offset + mesh_node.vertices.len() > u16::MAX as usize + 1 {
            return Err(GlbError::FormatError(format!(
                "GLB '{}' exceeds the engine's u16 static mesh index limit",
                path.display()
            )));
        }

        imported.vertices.extend(mesh_node.vertices.iter().copied());
        for index in &mesh_node.indices {
            let final_index = vertex_offset.checked_add(*index as usize).ok_or_else(|| {
                GlbError::FormatError(format!(
                    "GLB '{}' produced an index that overflows the static mesh limit",
                    path.display()
                ))
            })?;
            let final_index = u16::try_from(final_index).map_err(|_| {
                GlbError::FormatError(format!(
                    "GLB '{}' produced an index outside the engine's u16 range",
                    path.display()
                ))
            })?;
            imported.indices.push(final_index);
        }
    }

    Ok(imported)
}

/// Process a node from a GLB scene and append its data to the imported scene.
fn append_glb_scene_node(
    imported_scene: &mut ImportedScene,
    node: gltf::Node<'_>,
    parent_transform: Mat4,
    blob: &[u8],
    path: &Path,
) -> Result<(), GlbError> {
    let local_transform = Mat4::from_cols_array_2d(&node.transform().matrix());
    let world_transform = parent_transform * local_transform;
    let node_name = node.name().map(str::to_owned);
    let display_name = node_name
        .clone()
        .unwrap_or_else(|| format!("node_{}", node.index()));
    let metadata = parse_imported_extras(
        node.extras(),
        &format!("node '{display_name}'"),
        &mut imported_scene.warnings,
    );
    let children: Vec<_> = node.children().collect();
    let mesh = node.mesh();
    let camera = node.camera();
    let light = node.light();

    if node.skin().is_some() {
        imported_scene.warnings.push(format!(
            "ignored skin data on node '{display_name}' while importing scene '{}'",
            imported_scene.id
        ));
    }
    if node.weights().is_some_and(|weights| !weights.is_empty()) {
        imported_scene.warnings.push(format!(
            "ignored morph target weights on node '{display_name}' while importing scene '{}'",
            imported_scene.id
        ));
    }

    if let Some(mesh) = mesh.as_ref() {
        let mesh_name = mesh.name().map(str::to_owned);
        let primitives: Vec<_> = mesh.primitives().collect();
        let primitive_count = primitives.len();
        for (primitive_index, primitive) in primitives.into_iter().enumerate() {
            let (primitive_mesh, material) = import_scene_primitive(
                primitive,
                world_transform,
                blob,
                path,
                &mut imported_scene.warnings,
                &display_name,
            )?;
            imported_scene.mesh_nodes.push(ImportedSceneMeshNode {
                id: stable_scene_mesh_id(
                    &metadata,
                    node_name.as_deref(),
                    mesh_name.as_deref(),
                    node.index(),
                    primitive_index,
                    primitive_count,
                ),
                label: scene_mesh_label(
                    node_name.as_deref(),
                    mesh_name.as_deref(),
                    node.index(),
                    primitive_index,
                    primitive_count,
                ),
                node_name: node_name.clone(),
                mesh_name: mesh_name.clone(),
                node_index: node.index(),
                primitive_index,
                world_transform: world_transform.to_cols_array_2d(),
                vertices: primitive_mesh.vertices,
                indices: primitive_mesh.indices,
                material,
                metadata: metadata.clone(),
            });
        }
    }

    if let Some(camera) = camera.as_ref() {
        imported_scene.cameras.push(ImportedSceneCamera {
            id: stable_scene_camera_id(
                &metadata,
                node_name.as_deref(),
                camera.name(),
                node.index(),
            ),
            label: camera_label(node_name.as_deref(), camera.name(), node.index()),
            node_name: node_name.clone(),
            camera_name: camera.name().map(str::to_owned),
            node_index: node.index(),
            world_transform: world_transform.to_cols_array_2d(),
            projection: import_camera_projection(camera),
            metadata: metadata.clone(),
        });
    }

    if let Some(light) = light.as_ref() {
        match light.kind() {
            gltf::khr_lights_punctual::Kind::Directional => {
                let direction = world_transform.transform_vector3(Vec3::Z).normalize_or_zero();
                if direction.length_squared() == 0.0 {
                    imported_scene.warnings.push(format!(
                        "ignored directional light on node '{display_name}' with a degenerate transform while importing scene '{}'",
                        imported_scene.id
                    ));
                } else {
                    imported_scene
                        .directional_lights
                        .push(ImportedSceneDirectionalLight {
                            id: stable_scene_light_id(
                                &metadata,
                                node_name.as_deref(),
                                light.name(),
                                node.index(),
                            ),
                            label: scene_light_label(node_name.as_deref(), light.name(), node.index()),
                            node_name: node_name.clone(),
                            light_name: light.name().map(str::to_owned),
                            node_index: node.index(),
                            world_transform: world_transform.to_cols_array_2d(),
                            direction: direction.to_array(),
                            color: light.color(),
                            intensity: light.intensity(),
                            metadata: metadata.clone(),
                        });
                }
            }
            gltf::khr_lights_punctual::Kind::Point => imported_scene.warnings.push(format!(
                "ignored unsupported point light on node '{display_name}' while importing scene '{}'",
                imported_scene.id
            )),
            gltf::khr_lights_punctual::Kind::Spot { .. } => imported_scene.warnings.push(format!(
                "ignored unsupported spot light on node '{display_name}' while importing scene '{}'",
                imported_scene.id
            )),
        }
    }

    let is_anchor = metadata.vzglyd_anchor_tagged || metadata.vzglyd_id.is_some();
    if mesh.is_none() && camera.is_none() && light.is_none() && is_anchor {
        imported_scene.anchors.push(ImportedSceneAnchor {
            id: stable_anchor_id(&metadata, node_name.as_deref(), node.index()),
            label: anchor_label(node_name.as_deref(), node.index()),
            node_name: node_name.clone(),
            node_index: node.index(),
            world_transform: world_transform.to_cols_array_2d(),
            metadata: metadata.clone(),
        });
    } else if mesh.is_none() && camera.is_none() && light.is_none() && children.is_empty() {
        imported_scene.warnings.push(format!(
            "ignored unsupported empty node '{display_name}' while importing scene '{}'",
            imported_scene.id
        ));
    }

    for child in children {
        append_glb_scene_node(imported_scene, child, world_transform, blob, path)?;
    }

    Ok(())
}

/// Import a primitive from a GLB scene.
fn import_scene_primitive(
    primitive: gltf::Primitive<'_>,
    world_transform: Mat4,
    blob: &[u8],
    path: &Path,
    warnings: &mut Vec<String>,
    node_label: &str,
) -> Result<(ImportedMesh, ImportedSceneMaterial), GlbError> {
    if primitive.mode() != gltf::mesh::Mode::Triangles {
        return Err(GlbError::Unsupported(format!(
            "GLB '{}' uses primitive mode {:?}; only triangle meshes are supported",
            path.display(),
            primitive.mode()
        )));
    }

    let reader = primitive.reader(|buffer| match buffer.source() {
        gltf::buffer::Source::Bin => Some(blob),
        gltf::buffer::Source::Uri(_) => None,
    });
    let positions: Vec<[f32; 3]> = reader
        .read_positions()
        .ok_or_else(|| {
            GlbError::FormatError(format!(
                "GLB '{}' contains a primitive without POSITION data",
                path.display()
            ))
        })?
        .collect();
    let normals: Option<Vec<[f32; 3]>> = reader.read_normals().map(Iterator::collect);
    let tex_coords: Option<Vec<[f32; 2]>> = reader
        .read_tex_coords(0)
        .map(|coords| coords.into_f32().collect());
    let vertex_colors: Option<Vec<[f32; 4]>> = reader
        .read_colors(0)
        .map(|colors| colors.into_rgba_f32().collect());
    let primitive_indices: Vec<u32> = reader
        .read_indices()
        .map(|indices| indices.into_u32().collect())
        .unwrap_or_else(|| (0..positions.len() as u32).collect());
    if positions.len() > u16::MAX as usize + 1 {
        return Err(GlbError::FormatError(format!(
            "GLB '{}' exceeds the engine's u16 static mesh index limit",
            path.display()
        )));
    }

    let material = primitive.material();
    let material_factor = material.pbr_metallic_roughness().base_color_factor();
    let material_color = (material_factor != [1.0, 1.0, 1.0, 1.0]).then_some(material_factor);
    let material_metadata = parse_imported_extras(
        material.extras(),
        &format!("material on node '{node_label}'"),
        warnings,
    );
    let normal_transform = Mat3::from_mat4(world_transform).inverse().transpose();
    let mut imported = ImportedMesh {
        vertices: Vec::with_capacity(positions.len()),
        indices: Vec::with_capacity(primitive_indices.len()),
    };

    for (vertex_index, position) in positions.iter().enumerate() {
        let world_position = world_transform.transform_point3(Vec3::from_array(*position));
        let transformed_normal = normals
            .as_ref()
            .and_then(|normals| normals.get(vertex_index).copied())
            .map(|normal| {
                normal_transform
                    .mul_vec3(Vec3::from_array(normal))
                    .normalize_or_zero()
                    .to_array()
            });
        let color = vertex_colors
            .as_ref()
            .and_then(|colors| colors.get(vertex_index).copied())
            .map(|vertex_color| multiply_rgba(vertex_color, material_factor))
            .or(material_color);
        imported.vertices.push(ImportedVertex {
            position: world_position.to_array(),
            normal: transformed_normal,
            tex_coords: tex_coords
                .as_ref()
                .and_then(|coords| coords.get(vertex_index).copied()),
            color,
        });
    }

    for index in primitive_indices {
        let final_index = u16::try_from(index).map_err(|_| {
            GlbError::FormatError(format!(
                "GLB '{}' produced an index outside the engine's u16 range",
                path.display()
            ))
        })?;
        imported.indices.push(final_index);
    }

    fill_missing_normals(&mut imported);

    Ok((
        imported,
        ImportedSceneMaterial {
            name: material.name().map(str::to_owned),
            base_color_factor: material_factor,
            class_hint: material_metadata
                .vzglyd_material
                .clone()
                .or_else(|| material.name().map(str::to_owned)),
            metadata: material_metadata,
        },
    ))
}

/// Import camera projection from GLB.
fn import_camera_projection(camera: &gltf::Camera<'_>) -> ImportedCameraProjection {
    match camera.projection() {
        gltf::camera::Projection::Perspective(perspective) => {
            ImportedCameraProjection::Perspective {
                aspect_ratio: perspective.aspect_ratio(),
                yfov_rad: perspective.yfov(),
                znear: perspective.znear(),
                zfar: perspective.zfar(),
            }
        }
        gltf::camera::Projection::Orthographic(orthographic) => {
            ImportedCameraProjection::Orthographic {
                xmag: orthographic.xmag(),
                ymag: orthographic.ymag(),
                znear: orthographic.znear(),
                zfar: orthographic.zfar(),
            }
        }
    }
}

/// Parse extras from a GLB element.
fn parse_imported_extras(
    extras: &gltf::json::Extras,
    context: &str,
    warnings: &mut Vec<String>,
) -> ImportedExtras {
    let Some(raw) = extras.as_ref() else {
        return ImportedExtras::default();
    };

    let value = match serde_json::from_str::<JsonValue>(raw.get()) {
        Ok(value) => value,
        Err(error) => {
            warnings.push(format!("ignored invalid extras on {context}: {error}"));
            return ImportedExtras::default();
        }
    };
    let JsonValue::Object(raw) = value else {
        warnings.push(format!("ignored non-object extras on {context}"));
        return ImportedExtras::default();
    };

    ImportedExtras {
        vzglyd_id: read_extra_string(&raw, "vzglyd_id", context, warnings),
        vzglyd_pipeline: read_extra_string(&raw, "vzglyd_pipeline", context, warnings),
        vzglyd_material: read_extra_string(&raw, "vzglyd_material", context, warnings),
        vzglyd_anchor: read_extra_anchor(&raw, context, warnings),
        vzglyd_anchor_tagged: read_extra_anchor_tagged(&raw, context, warnings),
        vzglyd_hidden: read_extra_bool(&raw, "vzglyd_hidden", context, warnings),
        vzglyd_billboard: read_extra_bool(&raw, "vzglyd_billboard", context, warnings),
        vzglyd_entry_camera: read_extra_bool(&raw, "vzglyd_entry_camera", context, warnings),
        raw,
    }
}

/// Read a string value from extras.
fn read_extra_string(
    extras: &JsonMap<String, JsonValue>,
    key: &str,
    context: &str,
    warnings: &mut Vec<String>,
) -> Option<String> {
    match extras.get(key) {
        Some(JsonValue::String(value)) => Some(value.clone()),
        Some(other) => {
            warnings.push(format!(
                "ignored non-string extras key '{key}' on {context}: {other}"
            ));
            None
        }
        None => None,
    }
}

/// Read a boolean value from extras.
fn read_extra_bool(
    extras: &JsonMap<String, JsonValue>,
    key: &str,
    context: &str,
    warnings: &mut Vec<String>,
) -> bool {
    match extras.get(key) {
        Some(JsonValue::Bool(value)) => *value,
        Some(other) => {
            warnings.push(format!(
                "ignored non-bool extras key '{key}' on {context}: {other}"
            ));
            false
        }
        None => false,
    }
}

/// Read an anchor value from extras.
fn read_extra_anchor(
    extras: &JsonMap<String, JsonValue>,
    context: &str,
    warnings: &mut Vec<String>,
) -> Option<String> {
    match extras.get("vzglyd_anchor") {
        Some(JsonValue::String(value)) => Some(value.clone()),
        Some(JsonValue::Bool(true)) | None => None,
        Some(JsonValue::Bool(false)) => None,
        Some(other) => {
            warnings.push(format!(
                "ignored unsupported extras key 'vzglyd_anchor' on {context}: {other}"
            ));
            None
        }
    }
}

/// Read whether the node is tagged as an anchor.
fn read_extra_anchor_tagged(
    extras: &JsonMap<String, JsonValue>,
    context: &str,
    warnings: &mut Vec<String>,
) -> bool {
    match extras.get("vzglyd_anchor") {
        Some(JsonValue::String(_)) => true,
        Some(JsonValue::Bool(value)) => *value,
        None => false,
        Some(other) => {
            warnings.push(format!(
                "ignored unsupported extras key 'vzglyd_anchor' on {context}: {other}"
            ));
            false
        }
    }
}

/// Generate a label for a scene mesh.
fn scene_mesh_label(
    node_name: Option<&str>,
    mesh_name: Option<&str>,
    node_index: usize,
    primitive_index: usize,
    primitive_count: usize,
) -> String {
    let mut label = node_name
        .or(mesh_name)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("mesh_node_{node_index}"));
    if primitive_count > 1 {
        label.push_str(&format!("#primitive_{primitive_index}"));
    }
    label
}

/// Generate a stable ID for a scene mesh.
fn stable_scene_mesh_id(
    metadata: &ImportedExtras,
    node_name: Option<&str>,
    mesh_name: Option<&str>,
    node_index: usize,
    primitive_index: usize,
    primitive_count: usize,
) -> String {
    let mut id = metadata
        .vzglyd_id
        .clone()
        .or_else(|| node_name.map(str::to_owned))
        .or_else(|| mesh_name.map(str::to_owned))
        .unwrap_or_else(|| format!("mesh_node_{node_index}"));
    if primitive_count > 1 {
        id.push_str(&format!("#primitive_{primitive_index}"));
    }
    id
}

/// Generate a label for a camera.
fn camera_label(node_name: Option<&str>, camera_name: Option<&str>, node_index: usize) -> String {
    node_name
        .or(camera_name)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("camera_node_{node_index}"))
}

/// Generate a stable ID for a scene camera.
fn stable_scene_camera_id(
    metadata: &ImportedExtras,
    node_name: Option<&str>,
    camera_name: Option<&str>,
    node_index: usize,
) -> String {
    metadata
        .vzglyd_id
        .clone()
        .or_else(|| node_name.map(str::to_owned))
        .or_else(|| camera_name.map(str::to_owned))
        .unwrap_or_else(|| format!("camera_node_{node_index}"))
}

/// Generate a label for a scene light.
fn scene_light_label(
    node_name: Option<&str>,
    light_name: Option<&str>,
    node_index: usize,
) -> String {
    node_name
        .or(light_name)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("light_node_{node_index}"))
}

/// Generate a stable ID for a scene light.
fn stable_scene_light_id(
    metadata: &ImportedExtras,
    node_name: Option<&str>,
    light_name: Option<&str>,
    node_index: usize,
) -> String {
    metadata
        .vzglyd_id
        .clone()
        .or_else(|| node_name.map(str::to_owned))
        .or_else(|| light_name.map(str::to_owned))
        .unwrap_or_else(|| format!("light_node_{node_index}"))
}

/// Generate a label for an anchor.
fn anchor_label(node_name: Option<&str>, node_index: usize) -> String {
    node_name
        .map(str::to_owned)
        .unwrap_or_else(|| format!("anchor_node_{node_index}"))
}

/// Generate a stable ID for an anchor.
fn stable_anchor_id(
    metadata: &ImportedExtras,
    node_name: Option<&str>,
    node_index: usize,
) -> String {
    metadata
        .vzglyd_id
        .clone()
        .or_else(|| node_name.map(str::to_owned))
        .unwrap_or_else(|| format!("anchor_node_{node_index}"))
}

/// Fill missing normals by computing them from face geometry.
fn fill_missing_normals(imported: &mut ImportedMesh) {
    if imported
        .vertices
        .iter()
        .all(|vertex| vertex.normal.is_some())
    {
        return;
    }

    let mut accum = vec![Vec3::ZERO; imported.vertices.len()];
    for triangle in imported.indices.chunks_exact(3) {
        let i0 = triangle[0] as usize;
        let i1 = triangle[1] as usize;
        let i2 = triangle[2] as usize;
        let p0 = Vec3::from_array(imported.vertices[i0].position);
        let p1 = Vec3::from_array(imported.vertices[i1].position);
        let p2 = Vec3::from_array(imported.vertices[i2].position);
        let face_normal = (p1 - p0).cross(p2 - p0);
        accum[i0] += face_normal;
        accum[i1] += face_normal;
        accum[i2] += face_normal;
    }

    for (vertex, sum) in imported.vertices.iter_mut().zip(accum) {
        if vertex.normal.is_none() {
            vertex.normal = Some(sum.normalize_or_zero().to_array());
        }
    }
}

/// Multiply two RGBA colors.
fn multiply_rgba(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2], a[3] * b[3]]
}
