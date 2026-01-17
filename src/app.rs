use entropy_engine::core::pipeline::ExportPipeline;
use entropy_engine::core::editor::WindowSize;
use entropy_engine::helpers::load_project::place_project;
use entropy_engine::helpers::saved_data::{ComponentData, SavedState, ComponentKind, CollectableType, GenericProperties, CollectableProperties, LightProperties, NPCProperties, AttackStats, CharacterStats};
use entropy_engine::helpers::timelines::SavedTimelineStateConfig;
use entropy_engine::game_behaviors::stateful::{BehaviorConfig, CombatType};
use js_sys::Date;
use leptos::html::Canvas;
use leptos::task::spawn_local;
use leptos::{prelude::*};
use leptos_use::use_raf_fn;
use leptos_use::utils::Pausable;
use phosphor_leptos::{CHAT, CHATS, GAME_CONTROLLER, Icon, IconWeight, VIDEO};
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::cell::RefCell;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use leptos::logging::log;
use wasm_bindgen_futures::spawn_local as wasm_spawn_local;
use entropy_engine::helpers::load_project::load_project;
use leptos::web_sys;
use entropy_engine::handlers::{EntropyPosition, handle_key_press, handle_mouse_move, handle_mouse_move_on_shift, handle_add_model, handle_add_collectable, handle_add_water_plane, handle_add_npc};
use entropy_engine::water_plane::config::WaterConfig;
use entropy_engine::procedural_grass::grass::GrassConfig;
use entropy_engine::shape_primitives::{Cube::Cube, Sphere::Sphere};
use entropy_engine::procedural_heightmaps::heightmap_generation::{HeightmapGenerator, TerrainFeature, FeatureType, FalloffType};
use entropy_engine::helpers::landscapes::generate_landscape_data;
use std::time::{Duration, SystemTime};
use gloo_net::http::Request;
use web_sys::FormData;
use nalgebra::{Isometry3, Translation3, UnitQuaternion, Vector3};

use crate::components::component_browser::ComponentPropertiesEditor;
use crate::components::assets_browser::AssetsBrowser;

pub fn get_api_url() -> String {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location.hostname().unwrap_or_default();

    if hostname == "localhost" || hostname == "127.0.0.1" {
        "http://localhost:3000".to_string()
    } else {
        "https://entropy-site.vercel.app".to_string()
    }
}

async fn save_project(project_id: &str, saved_state: &SavedState) -> Result<(), String> {
    let url = format!("{}/api/projects/{}", get_api_url(), project_id);
    let body = serde_json::json!({ "savedData": saved_state });
    
    Request::patch(&url)
        .json(&body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(rename = "savedData")]
    pub saved_data: Option<SavedState>,
    #[serde(default)]
    pub sessions: Vec<ChatSession>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: ToolCallFunction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct OpenChatResponse {
    project: Project, session: ChatSession
}

async fn execute_tool_call(
    tool_call: &ToolCall,
    pipeline_store: LocalResource<Option<Rc<RefCell<ExportPipeline>>>>,
    project_id: String,
    selected_project: ReadSignal<Option<Project>>,
) -> String {
    log!("Executing tool call: {:?}", tool_call.function.name);

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TransformObjectArgs {
        component_id: String,
        translation: Option<[f32; 3]>,
        rotation: Option<[f32; 3]>,
        scale: Option<[f32; 3]>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    // #[serde(rename_all = "camelCase")]
    struct ConfigureWaterArgs {
        #[serde(rename = "componentId")]
        component_id: Option<String>,
        shallow_color: Option<[f32; 3]>,
        medium_color: Option<[f32; 3]>,
        deep_color: Option<[f32; 3]>,
        ripple_amplitude_multiplier: Option<f32>,
        ripple_freq: Option<f32>,
        ripple_speed: Option<f32>,
        shoreline_foam_range: Option<f32>,
        crest_foam_min: Option<f32>,
        crest_foam_max: Option<f32>,
        sparkle_intensity: Option<f32>,
        sparkle_threshold: Option<f32>,
        subsurface_multiplier: Option<f32>,
        fresnel_power: Option<f32>,
        fresnel_multiplier: Option<f32>,

        // Wave 1 - primary wave
        pub wave1_amplitude: Option<f32>,
        pub wave1_frequency: Option<f32>,
        pub wave1_speed: Option<f32>,
        pub wave1_steepness: Option<f32>,
        pub wave1_direction: Option<[f32; 2]>,

        // Wave 2 - secondary wave
        pub wave2_amplitude: Option<f32>,
        pub wave2_frequency: Option<f32>,
        pub wave2_speed: Option<f32>,
        pub wave2_steepness: Option<f32>,
        pub wave2_direction: Option<[f32; 2]>,

        // Wave 3 - tertiary wave
        pub wave3_amplitude: Option<f32>,
        pub wave3_frequency: Option<f32>,
        pub wave3_speed: Option<f32>,
        pub wave3_steepness: Option<f32>,
        pub wave3_direction: Option<[f32; 2]>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct ConfigureGrassArgs {
        #[serde(rename = "componentId")]
        component_id: Option<String>,
        wind_strength: Option<f32>,
        wind_speed: Option<f32>,
        blade_height: Option<f32>,
        blade_width: Option<f32>,
        blade_density: Option<f32>, // Changing to f32 to match tool definition, will cast to u32
        render_distance: Option<f32>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct SpawnPrimitiveArgs {
        r#type: String,
        position: [f32; 3],
        scale: Option<[f32; 3]>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct ConfigureSkyArgs {
        #[serde(rename = "componentId")]
        component_id: Option<String>,
        horizon_color: Option<[f32; 3]>,
        zenith_color: Option<[f32; 3]>,
        sun_direction: Option<[f32; 3]>,
        sun_color: Option<[f32; 3]>,
        sun_intensity: Option<f32>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct ConfigureTreesArgs {
        #[serde(rename = "componentId")]
        component_id: Option<String>,
        seed: Option<u32>,
        trunk_height: Option<f32>,
        trunk_radius: Option<f32>,
        branch_levels: Option<u32>,
        foliage_radius: Option<f32>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct SpawnModelArgs {
        #[serde(rename = "assetId")]
        asset_id: String,
        position: Option<[f32; 3]>,
        rotation: Option<[f32; 3]>,
        scale: Option<[f32; 3]>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct SpawnPointLightArgs {
        position: [f32; 3],
        color: Option<[f32; 3]>,
        intensity: Option<f32>,
        radius: Option<f32>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct SpawnCollectableArgs {
        #[serde(rename = "assetId")]
        asset_id: String,
        r#type: String, // "Item", "MeleeWeapon", etc.
        position: Option<[f32; 3]>,
        rotation: Option<[f32; 3]>,
        scale: Option<[f32; 3]>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct SpawnNPCArgs {
        #[serde(rename = "assetId")]
        asset_id: String,
        position: Option<[f32; 3]>,
        rotation: Option<[f32; 3]>,
        scale: Option<[f32; 3]>,
        aggressiveness: Option<f32>,
        combat_type: Option<String>,
        wander_radius: Option<f32>,
        wander_speed: Option<f32>,
        detection_radius: Option<f32>,
        damage: Option<f32>,
        health: Option<f32>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct SaveScriptArgs {
        filename: String,
        content: String,
        componentId: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TerrainFeatureArgs {
        r#type: String,
        center: [f64; 2],
        radius: f64,
        intensity: f64,
        falloff: String,
        flat_top: Option<f64>,
        transition: Option<f64>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct GenerateHeightmapArgs {
        #[serde(rename = "componentId")]
        component_id: Option<String>,
        seed: Option<u32>,
        scale: Option<f64>,
        persistence: Option<f64>,
        lacunarity: Option<f64>,
        features: Option<Vec<TerrainFeatureArgs>>,
    }

    let mut saved_state_clone = None;

    if tool_call.function.name == "transformObject" {
        let args: Result<TransformObjectArgs, _> = serde_json::from_str(&tool_call.function.arguments);
        if let Ok(args) = args {
            if let Some(pipeline_arc_val) = pipeline_store.get() {
                if let Some(pipeline_arc) = pipeline_arc_val.as_ref() {
                    let mut pipeline = pipeline_arc.borrow_mut();
                    if let Some(editor) = pipeline.export_editor.as_mut() {
                        // Update SavedState
                        if let Some(saved_state) = editor.saved_state.as_mut() {
                            if let Some(level) = saved_state.levels.as_mut().and_then(|l| l.get_mut(0)) {
                                if let Some(components) = level.components.as_mut() {
                                    if let Some(component) = components.iter_mut().find(|c| c.id == args.component_id) {
                                        if let Some(translation) = args.translation {
                                            component.generic_properties.position = translation;
                                        }
                                        if let Some(rotation) = args.rotation {
                                            component.generic_properties.rotation = rotation;
                                        }
                                        if let Some(scale) = args.scale {
                                            component.generic_properties.scale = scale;
                                        }
                                    }
                                }
                            }
                            saved_state_clone = Some(saved_state.clone());
                        }

                        // Update RendererState
                        if let Some(renderer_state) = editor.renderer_state.as_mut() {
                            if let Some(model) = renderer_state.models.iter_mut().find(|m| m.id == args.component_id) {
                                for mesh in model.meshes.iter_mut() {
                                    if let Some(translation) = args.translation {
                                        mesh.transform.update_position(translation);
                                    }
                                    if let Some(rotation) = args.rotation {
                                        mesh.transform.update_rotation(rotation);
                                    }
                                    if let Some(scale) = args.scale {
                                        mesh.transform.update_scale(scale);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    } else if tool_call.function.name == "configureWater" {
        log!("Configuring water plane...");
        let args: Result<ConfigureWaterArgs, _> = serde_json::from_str(&tool_call.function.arguments);
        if let Ok(args) = args {
            if let Some(pipeline_arc_val) = pipeline_store.get() {
                if let Some(pipeline_arc) = pipeline_arc_val.as_ref() {
                    let mut pipeline = pipeline_arc.borrow_mut();
                    if let Some(editor) = pipeline.export_editor.as_mut() {
                        if let Some(renderer_state) = editor.renderer_state.as_mut() {
                            
                            // Check if we have any water planes
                            if renderer_state.water_planes.is_empty() {
                                // Try to create one if we have a landscape
                                if let Some(landscape) = renderer_state.landscapes.first() {
                                     let landscape_id = landscape.id.clone();
                                     let device = &editor.gpu_resources.as_ref().unwrap().device;
                                     let camera_binding = editor.camera_binding.as_ref().unwrap(); 
                                     let surface_format = wgpu::TextureFormat::Rgba8Unorm; // Matching ProjectCanvas
                                     
                                     handle_add_water_plane(renderer_state, device, &camera_binding.bind_group_layout, surface_format, landscape_id.clone());
                                     log!("Water plane created for landscape {}", landscape_id);
                                }
                            }

                            // Now configure the first water plane (assuming single water plane support for now)
                            if let Some(water_plane) = renderer_state.water_planes.get_mut(0) {
                                let mut current_config = water_plane.config; // Get current config

                                log!("Configuring water plane still... {:?}", args);

                                if let Some(color) = args.shallow_color {
                                    current_config.shallow_color = [color[0], color[1], color[2], 1.0];
                                }
                                if let Some(color) = args.medium_color {
                                    current_config.medium_color = [color[0], color[1], color[2], 1.0];
                                }
                                if let Some(color) = args.deep_color {
                                    current_config.deep_color = [color[0], color[1], color[2], 1.0];
                                }
                                if let Some(val) = args.ripple_amplitude_multiplier {
                                    current_config.ripple_amplitude_multiplier = val;
                                }
                                if let Some(val) = args.ripple_freq {
                                    current_config.ripple_freq = val;
                                }
                                if let Some(val) = args.ripple_speed {
                                    current_config.ripple_speed = val;
                                }
                                if let Some(val) = args.shoreline_foam_range {
                                    current_config.shoreline_foam_range = val;
                                }
                                if let Some(val) = args.crest_foam_min {
                                    current_config.crest_foam_min = val;
                                }
                                if let Some(val) = args.crest_foam_max {
                                    current_config.crest_foam_max = val;
                                }
                                if let Some(val) = args.sparkle_intensity {
                                    current_config.sparkle_intensity = val;
                                }
                                if let Some(val) = args.sparkle_threshold {
                                    current_config.sparkle_threshold = val;
                                }
                                if let Some(val) = args.subsurface_multiplier {
                                    current_config.subsurface_multiplier = val;
                                }
                                if let Some(val) = args.fresnel_power {
                                    current_config.fresnel_power = val;
                                }
                                if let Some(val) = args.fresnel_multiplier {
                                    current_config.fresnel_multiplier = val;
                                }

                                if let Some(val) = args.wave1_amplitude {
                                    current_config.wave1_amplitude = val;
                                }
                                if let Some(val) = args.wave1_frequency {
                                    current_config.wave1_frequency = val;
                                }
                                if let Some(val) = args.wave1_speed {
                                    current_config.wave1_speed = val;
                                }
                                if let Some(val) = args.wave1_steepness {
                                    current_config.wave1_steepness = val;
                                }
                                if let Some(val) = args.wave1_direction {
                                    current_config.wave1_direction = val;
                                }
                                
                                if let Some(val) = args.wave2_amplitude {
                                    current_config.wave2_amplitude = val;
                                }
                                if let Some(val) = args.wave2_frequency {
                                    current_config.wave2_frequency = val;
                                }
                                if let Some(val) = args.wave2_speed {
                                    current_config.wave2_speed = val;
                                }
                                if let Some(val) = args.wave2_steepness {
                                    current_config.wave2_steepness = val;
                                }
                                if let Some(val) = args.wave2_direction {
                                    current_config.wave2_direction = val;
                                }

                                if let Some(val) = args.wave3_amplitude {
                                    current_config.wave3_amplitude = val;
                                }
                                if let Some(val) = args.wave3_frequency {
                                    current_config.wave3_frequency = val;
                                }
                                if let Some(val) = args.wave3_speed {
                                    current_config.wave3_speed = val;
                                }
                                if let Some(val) = args.wave3_steepness {
                                    current_config.wave3_steepness = val;
                                }
                                if let Some(val) = args.wave3_direction {
                                    current_config.wave3_direction = val;
                                }

                                // water_plane.config = current_config;
                                water_plane.update_config(&editor.gpu_resources.as_ref().expect("Couldn't get gpu resources").queue, current_config);

                                log!("Water plane configured {:?}", water_plane.config);

                                if let Some(saved_state) = editor.saved_state.as_mut() {
                                    saved_state_clone = Some(saved_state.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    } else if tool_call.function.name == "configureSky" {
        log!("Configuring sky...");
        let args: Result<ConfigureSkyArgs, _> = serde_json::from_str(&tool_call.function.arguments);
        if let Ok(args) = args {
            if let Some(pipeline_arc_val) = pipeline_store.get() {
                if let Some(pipeline_arc) = pipeline_arc_val.as_ref() {
                    let mut pipeline = pipeline_arc.borrow_mut();
                    if let Some(editor) = pipeline.export_editor.as_mut() {
                        if let Some(saved_state) = editor.saved_state.as_mut() {
                            if let Some(level) = saved_state.levels.as_mut().and_then(|l| l.get_mut(0)) {
                                if level.procedural_sky.is_none() {
                                    level.procedural_sky = Some(entropy_engine::helpers::saved_data::ProceduralSkyConfig::default());
                                }
                                if let Some(sky) = level.procedural_sky.as_mut() {
                                    if let Some(color) = args.horizon_color { sky.horizon_color = color; }
                                    if let Some(color) = args.zenith_color { sky.zenith_color = color; }
                                    if let Some(dir) = args.sun_direction { sky.sun_direction = dir; }
                                    if let Some(color) = args.sun_color { sky.sun_color = color; }
                                    if let Some(intensity) = args.sun_intensity { sky.sun_intensity = intensity; }
                                }
                            }
                            saved_state_clone = Some(saved_state.clone());
                        }
                    }
                }
            }
        }
    } else if tool_call.function.name == "configureTrees" {
        log!("Configuring trees...");
        let args: Result<ConfigureTreesArgs, _> = serde_json::from_str(&tool_call.function.arguments);
        if let Ok(args) = args {
            if let Some(pipeline_arc_val) = pipeline_store.get() {
                if let Some(pipeline_arc) = pipeline_arc_val.as_ref() {
                    let mut pipeline = pipeline_arc.borrow_mut();
                    if let Some(editor) = pipeline.export_editor.as_mut() {
                        let mut new_tree_props = None;
                        
                        // Update SavedState
                        if let Some(saved_state) = editor.saved_state.as_mut() {
                            if let Some(level) = saved_state.levels.as_mut().and_then(|l| l.get_mut(0)) {
                                if let Some(components) = level.components.as_mut() {
                                    
                                    let mut found = false;
                                    for component in components.iter_mut() {
                                        if component.kind == Some(entropy_engine::helpers::saved_data::ComponentKind::ProceduralTree) {
                                            if let Some(target_id) = &args.component_id {
                                                if &component.id != target_id {
                                                    continue;
                                                }
                                            }
                                            
                                            if component.procedural_tree_properties.is_none() {
                                                component.procedural_tree_properties = Some(entropy_engine::helpers::saved_data::ProceduralTreeProperties::default());
                                            }
                                            if let Some(props) = component.procedural_tree_properties.as_mut() {
                                                if let Some(val) = args.seed { props.seed = val; }
                                                if let Some(val) = args.trunk_height { props.trunk_height = val; }
                                                if let Some(val) = args.trunk_radius { props.trunk_radius = val; }
                                                if let Some(val) = args.branch_levels { props.branch_levels = val; }
                                                if let Some(val) = args.foliage_radius { props.foliage_radius = val; }
                                                new_tree_props = Some(props.clone());
                                            }
                                            found = true;
                                            break; 
                                        }
                                    }
                                    
                                    if !found && args.component_id.is_none() {
                                        let props = entropy_engine::helpers::saved_data::ProceduralTreeProperties {
                                            seed: args.seed.unwrap_or(0),
                                            trunk_height: args.trunk_height.unwrap_or(3.5),
                                            trunk_radius: args.trunk_radius.unwrap_or(0.25),
                                            branch_levels: args.branch_levels.unwrap_or(4),
                                            foliage_radius: args.foliage_radius.unwrap_or(0.5),
                                        };
                                        
                                        let new_component = ComponentData {
                                            id: Uuid::new_v4().to_string(),
                                            kind: Some(entropy_engine::helpers::saved_data::ComponentKind::ProceduralTree),
                                            asset_id: "".to_string(),
                                            procedural_tree_properties: Some(props.clone()),
                                            ..Default::default()
                                        };
                                        components.push(new_component);
                                        new_tree_props = Some(props);
                                        log!("Created new tree component in saved state.");
                                    }
                                }
                            }
                            saved_state_clone = Some(saved_state.clone());
                        }

                        // Update RendererState (live update)
                        if let Some(renderer_state) = editor.renderer_state.as_mut() {
                            if let Some(new_props) = new_tree_props {
                                // For now, update ALL trees since we don't have ID mapping easily accessible in renderer_state yet
                                // Or assume single tree system per level
                                for trees in &mut renderer_state.procedural_trees {
                                    let device = &editor.gpu_resources.as_ref().unwrap().device;
                                    trees.regenerate(device, new_props.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    } else if tool_call.function.name == "spawnModel" {
        log!("Spawning model...");
        let args: Result<SpawnModelArgs, _> = serde_json::from_str(&tool_call.function.arguments);
        if let Ok(args) = args {
            if let Some(pipeline_arc_val) = pipeline_store.get() {
                if let Some(pipeline_arc) = pipeline_arc_val.as_ref() {
                    let mut pipeline = pipeline_arc.borrow_mut();
                    if let Some(editor) = pipeline.export_editor.as_mut() {
                        // let project_id = editor.project_id.clone();
                        let project_id = selected_project.get().as_ref().expect("Couldn't get selected project").id.clone();
                        let mut asset_file_name = String::new();
                        
                        // Find asset filename in SavedState
                        if let Some(saved_state) = editor.saved_state.as_ref() {
                            if let Some(model) = saved_state.models.iter().find(|m| m.id == args.asset_id) {
                                asset_file_name = model.fileName.clone();
                            }
                        }

                        if !asset_file_name.is_empty() {
                            let component_id = Uuid::new_v4().to_string();
                            let pos = args.position.unwrap_or([0.0, 0.0, 0.0]);
                            let rot = args.rotation.unwrap_or([0.0, 0.0, 0.0]);
                            let scale = args.scale.unwrap_or([1.0, 1.0, 1.0]);

                            let model_position = Translation3::new(pos[0], pos[1], pos[2]);
                            let model_rotation = UnitQuaternion::from_euler_angles(
                                rot[0].to_radians(), rot[1].to_radians(), rot[2].to_radians()
                            );
                            let model_iso = Isometry3::from_parts(model_position, model_rotation);
                            let model_scale = Vector3::new(scale[0], scale[1], scale[2]);

                            let renderer_state = editor.renderer_state.as_mut().unwrap();
                            let gpu_resources = editor.gpu_resources.as_ref().unwrap();
                            let camera = editor.camera.as_ref().unwrap();

                            handle_add_model(
                                renderer_state,
                                &gpu_resources.device,
                                &gpu_resources.queue,
                                project_id,
                                args.asset_id.clone(),
                                component_id.clone(),
                                asset_file_name,
                                model_iso,
                                model_scale,
                                camera,
                                None // Script state
                            ).await;

                            // Update SavedState
                            if let Some(saved_state) = editor.saved_state.as_mut() {
                                if let Some(level) = saved_state.levels.as_mut().and_then(|l| l.get_mut(0)) {
                                    let new_component = ComponentData {
                                        id: component_id,
                                        kind: Some(ComponentKind::Model),
                                        asset_id: args.asset_id,
                                        generic_properties: GenericProperties {
                                            name: "New Model".to_string(),
                                            position: pos,
                                            rotation: rot,
                                            scale: scale,
                                        },
                                        ..Default::default()
                                    };
                                    
                                    if let Some(components) = level.components.as_mut() {
                                        components.push(new_component);
                                    } else {
                                        level.components = Some(vec![new_component]);
                                    }
                                }
                                saved_state_clone = Some(saved_state.clone());
                            }
                        } else {
                            log!("Asset not found: {}", args.asset_id);
                        }
                    }
                }
            }
        }
    } else if tool_call.function.name == "spawnPointLight" {
        log!("Spawning point light...");
        let args: Result<SpawnPointLightArgs, _> = serde_json::from_str(&tool_call.function.arguments);
        if let Ok(args) = args {
            if let Some(pipeline_arc_val) = pipeline_store.get() {
                if let Some(pipeline_arc) = pipeline_arc_val.as_ref() {
                    let mut pipeline = pipeline_arc.borrow_mut();
                    if let Some(editor) = pipeline.export_editor.as_mut() {
                        let component_id = Uuid::new_v4().to_string();
                        let color = args.color.unwrap_or([1.0, 1.0, 1.0]);
                        let intensity = args.intensity.unwrap_or(1.0);
                        let radius = args.radius.unwrap_or(10.0);

                        // Update RendererState
                        if let Some(renderer_state) = editor.renderer_state.as_mut() {
                            renderer_state.point_lights.push(entropy_engine::core::editor::PointLight {
                                position: args.position,
                                _padding1: 0,
                                color,
                                _padding2: 0,
                                intensity,
                                max_distance: radius, // Using radius as max_distance
                                _padding3: [0; 2],
                            });
                        }

                        // Update SavedState
                        if let Some(saved_state) = editor.saved_state.as_mut() {
                            if let Some(level) = saved_state.levels.as_mut().and_then(|l| l.get_mut(0)) {
                                let new_component = ComponentData {
                                    id: component_id,
                                    kind: Some(ComponentKind::PointLight),
                                    asset_id: "".to_string(),
                                    generic_properties: GenericProperties {
                                        name: "New Light".to_string(),
                                        position: args.position,
                                        ..Default::default()
                                    },
                                    light_properties: Some(LightProperties {
                                        color: [color[0], color[1], color[2], 1.0],
                                        intensity,
                                    }),
                                    ..Default::default()
                                };
                                
                                if let Some(components) = level.components.as_mut() {
                                    components.push(new_component);
                                } else {
                                    level.components = Some(vec![new_component]);
                                }
                            }
                            saved_state_clone = Some(saved_state.clone());
                        }
                    }
                }
            }
        }
    } else if tool_call.function.name == "spawnCollectable" {
        log!("Spawning collectable...");
        let args: Result<SpawnCollectableArgs, _> = serde_json::from_str(&tool_call.function.arguments);
        if let Ok(args) = args {
            if let Some(pipeline_arc_val) = pipeline_store.get() {
                if let Some(pipeline_arc) = pipeline_arc_val.as_ref() {
                    let mut pipeline = pipeline_arc.borrow_mut();
                    if let Some(editor) = pipeline.export_editor.as_mut() {
                        // let project_id = editor.project_id.clone();
                        let project_id = selected_project.get().as_ref().expect("Couldn't get selected project").id.clone();
                        let mut asset_file_name = String::new();
                        let mut stat_data = None;

                        // Find asset and default stat in SavedState
                        if let Some(saved_state) = editor.saved_state.as_ref() {
                            if let Some(model) = saved_state.models.iter().find(|m| m.id == args.asset_id) {
                                asset_file_name = model.fileName.clone();
                            }
                            if let Some(stats) = &saved_state.stats {
                                if !stats.is_empty() {
                                    stat_data = Some(stats[0].clone()); // Pick first stat for now
                                }
                            }
                        }

                        if !asset_file_name.is_empty() && stat_data.is_some() {
                            let component_id = Uuid::new_v4().to_string();
                            let pos = args.position.unwrap_or([0.0, 0.0, 0.0]);
                            let rot = args.rotation.unwrap_or([0.0, 0.0, 0.0]);
                            let scale = args.scale.unwrap_or([1.0, 1.0, 1.0]);

                            let model_position = Translation3::new(pos[0], pos[1], pos[2]);
                            let model_rotation = UnitQuaternion::from_euler_angles(
                                rot[0].to_radians(), rot[1].to_radians(), rot[2].to_radians()
                            );
                            let model_iso = Isometry3::from_parts(model_position, model_rotation);
                            let model_scale = Vector3::new(scale[0], scale[1], scale[2]);

                            let collectable_type = match args.r#type.as_str() {
                                "MeleeWeapon" => CollectableType::MeleeWeapon,
                                "RangedWeapon" => CollectableType::RangedWeapon,
                                "Armor" => CollectableType::Armor,
                                _ => CollectableType::Item,
                            };

                            let related_stat = stat_data.unwrap(); // Verified safe above

                            let collectable_properties = CollectableProperties {
                                model_id: Some(component_id.clone()), // Use same ID for model part
                                collectable_type: Some(collectable_type.clone()),
                                stat_id: Some(related_stat.id.clone()),
                            };

                            let renderer_state = editor.renderer_state.as_mut().unwrap();
                            let gpu_resources = editor.gpu_resources.as_ref().unwrap();
                            let camera = editor.camera.as_ref().unwrap();

                            handle_add_collectable(
                                renderer_state,
                                &gpu_resources.device,
                                &gpu_resources.queue,
                                project_id,
                                args.asset_id.clone(),
                                component_id.clone(),
                                asset_file_name,
                                model_iso,
                                model_scale,
                                camera,
                                &collectable_properties,
                                &related_stat,
                                false, // Don't hide
                                None // Script state
                            ).await;

                            // Update SavedState
                            if let Some(saved_state) = editor.saved_state.as_mut() {
                                if let Some(level) = saved_state.levels.as_mut().and_then(|l| l.get_mut(0)) {
                                    let new_component = ComponentData {
                                        id: component_id,
                                        kind: Some(ComponentKind::Collectable),
                                        asset_id: args.asset_id,
                                        generic_properties: GenericProperties {
                                            name: "New Collectable".to_string(),
                                            position: pos,
                                            rotation: rot,
                                            scale: scale,
                                        },
                                        collectable_properties: Some(collectable_properties),
                                        ..Default::default()
                                    };
                                    
                                    if let Some(components) = level.components.as_mut() {
                                        components.push(new_component);
                                    } else {
                                        level.components = Some(vec![new_component]);
                                    }
                                }
                                saved_state_clone = Some(saved_state.clone());
                            }
                        } else {
                            log!("Asset or Stats not found for collectable. AssetId: {}", args.asset_id);
                        }
                    }
                }
            }
        }
    } else if tool_call.function.name == "configureGrass" {
        log!("Configuring grass...");
        let args: Result<ConfigureGrassArgs, _> = serde_json::from_str(&tool_call.function.arguments);
        if let Ok(args) = args {
             if let Some(pipeline_arc_val) = pipeline_store.get() {
                if let Some(pipeline_arc) = pipeline_arc_val.as_ref() {
                    let mut pipeline = pipeline_arc.borrow_mut();
                    if let Some(editor) = pipeline.export_editor.as_mut() {
                        
                        // Update RendererState (Live)
                        if let Some(renderer_state) = editor.renderer_state.as_mut() {
                             for grass in renderer_state.grasses.iter_mut() {
                                 if let Some(val) = args.wind_strength { grass.config.wind_strength = val; }
                                 if let Some(val) = args.wind_speed { grass.config.wind_speed = val; }
                                 if let Some(val) = args.blade_height { grass.config.blade_height = val; }
                                 if let Some(val) = args.blade_width { grass.config.blade_width = val; }
                                 if let Some(val) = args.blade_density { grass.config.blade_density = val; }
                                 if let Some(val) = args.render_distance { grass.config.render_distance = val; }
                             }
                        }

                        // Update SavedState
                        if let Some(saved_state) = editor.saved_state.as_mut() {
                            if let Some(level) = saved_state.levels.as_mut().and_then(|l| l.get_mut(0)) {
                                if let Some(components) = level.components.as_mut() {
                                    // Find existing grass
                                    let mut found = false;
                                    for component in components.iter_mut() {
                                        if component.kind == Some(entropy_engine::helpers::saved_data::ComponentKind::ProceduralGrass) {
                                            if let Some(target_id) = &args.component_id {
                                                if &component.id != target_id {
                                                    continue;
                                                }
                                            }
                                            
                                            if component.procedural_grass_properties.is_none() {
                                                component.procedural_grass_properties = Some(entropy_engine::helpers::saved_data::ProceduralGrassProperties::default());
                                            }
                                            if let Some(props) = component.procedural_grass_properties.as_mut() {
                                                if let Some(val) = args.wind_strength { props.wind_strength = val; }
                                                if let Some(val) = args.wind_speed { props.wind_speed = val; }
                                                if let Some(val) = args.blade_height { props.blade_height = val; }
                                                if let Some(val) = args.blade_width { props.blade_width = val; }
                                                if let Some(val) = args.blade_density { props.blade_density = val as u32; }
                                                if let Some(val) = args.render_distance { props.render_distance = val; }
                                            }
                                            found = true;
                                        }
                                    }
                                    
                                    if !found && args.component_id.is_none() {
                                        let new_grass_props = entropy_engine::helpers::saved_data::ProceduralGrassProperties {
                                            wind_strength: args.wind_strength.unwrap_or(2.5),
                                            wind_speed: args.wind_speed.unwrap_or(0.3),
                                            blade_height: args.blade_height.unwrap_or(2.75),
                                            blade_width: args.blade_width.unwrap_or(0.03),
                                            blade_density: args.blade_density.unwrap_or(15.0) as u32,
                                            render_distance: args.render_distance.unwrap_or(150.0),
                                            grid_size: 10.0,
                                            brownian_strength: 0.5,
                                        };
                                        
                                        let new_component = ComponentData {
                                            id: Uuid::new_v4().to_string(),
                                            kind: Some(entropy_engine::helpers::saved_data::ComponentKind::ProceduralGrass),
                                            asset_id: "".to_string(),
                                            procedural_grass_properties: Some(new_grass_props),
                                            ..Default::default()
                                        };
                                        components.push(new_component);
                                        log!("Created new grass component in saved state.");
                                    }
                                }
                            }
                            saved_state_clone = Some(saved_state.clone());
                        }
                    }
                }
            }
        }
    } else if tool_call.function.name == "spawnPrimitive" {
        log!("Spawning primitive...");
        let args: Result<SpawnPrimitiveArgs, _> = serde_json::from_str(&tool_call.function.arguments);
        if let Ok(args) = args {
            if let Some(pipeline_arc_val) = pipeline_store.get() {
                if let Some(pipeline_arc) = pipeline_arc_val.as_ref() {
                    let mut pipeline = pipeline_arc.borrow_mut();
                    if let Some(editor) = pipeline.export_editor.as_mut() {
                        let device = &editor.gpu_resources.as_ref().unwrap().device;
                        let queue = &editor.gpu_resources.as_ref().unwrap().queue;
                        let model_layout = editor.model_bind_group_layout.as_ref().unwrap();
                        let group_layout = editor.group_bind_group_layout.as_ref().unwrap();
                        let camera = editor.camera.as_ref().unwrap();

                        // We need access to texture render mode buffer which is in RendererState or Pipeline
                        // But access via RendererState is hard because we are borrowing pipeline/editor.
                        // However, Cube::new needs it.
                        // In pipeline.rs, `texture_render_mode_buffer` is passed to `RendererState`.
                        // But `editor.renderer_state` has it.
                        // `renderer_state.texture_render_mode_buffer`
                        
                        let buffer = if let Some(rs) = &editor.renderer_state {
                            rs.texture_render_mode_buffer.clone()
                        } else {
                            // Fallback or error
                            log!("Renderer state not found");
                            return "{\"success\": false}".to_string();
                        };

                        if let Some(renderer_state) = editor.renderer_state.as_mut() {
                            match args.r#type.as_str() {
                                "Cube" => {
                                    let mut cube = Cube::new(
                                        device,
                                        queue,
                                        model_layout,
                                        group_layout,
                                        &buffer,
                                        camera
                                    );
                                    cube.transform.update_position(args.position);
                                    if let Some(scale) = args.scale {
                                        cube.transform.update_scale(scale);
                                    }
                                    renderer_state.cubes.push(cube);
                                },
                                "Sphere" => {
                                    let mut sphere = Sphere::new(
                                        device,
                                        queue,
                                        model_layout,
                                        group_layout,
                                        &buffer,
                                        camera,
                                        1.0, // radius
                                        32, // sectors
                                        32, // stacks
                                        [1.0, 1.0, 1.0], // color
                                        false // debug_moving
                                    );
                                    sphere.transform.update_position(args.position);
                                    if let Some(scale) = args.scale {
                                        sphere.transform.update_scale(scale);
                                    }
                                    renderer_state.spheres.push(sphere);
                                },
                                _ => log!("Unknown primitive type"),
                            }
                            
                            if let Some(saved_state) = editor.saved_state.as_mut() {
                                saved_state_clone = Some(saved_state.clone());
                            }
                        }
                    }
                }
            }
        }
    } else if tool_call.function.name == "spawnNPC" {
        log!("Spawning NPC...");
        let args: Result<SpawnNPCArgs, _> = serde_json::from_str(&tool_call.function.arguments);
        if let Ok(args) = args {
            if let Some(pipeline_arc_val) = pipeline_store.get() {
                if let Some(pipeline_arc) = pipeline_arc_val.as_ref() {
                    let mut pipeline = pipeline_arc.borrow_mut();
                    if let Some(editor) = pipeline.export_editor.as_mut() {
                         let project_id = selected_project.get().as_ref().expect("Couldn't get selected project").id.clone();
                         let mut asset_file_name = String::new();

                        // Find asset in SavedState
                        if let Some(saved_state) = editor.saved_state.as_ref() {
                            if let Some(model) = saved_state.models.iter().find(|m| m.id == args.asset_id) {
                                asset_file_name = model.fileName.clone();
                            }
                        }

                        if !asset_file_name.is_empty() {
                            let component_id = Uuid::new_v4().to_string();
                            let pos = args.position.unwrap_or([0.0, 0.0, 0.0]);
                            let rot = args.rotation.unwrap_or([0.0, 0.0, 0.0]);
                            let scale = args.scale.unwrap_or([1.0, 1.0, 1.0]);

                            let model_position = Translation3::new(pos[0], pos[1], pos[2]);
                            let model_rotation = UnitQuaternion::from_euler_angles(
                                rot[0].to_radians(), rot[1].to_radians(), rot[2].to_radians()
                            );
                            let model_iso = Isometry3::from_parts(model_position, model_rotation);
                            let model_scale = Vector3::new(scale[0], scale[1], scale[2]);

                            let combat_type = match args.combat_type.as_deref() {
                                Some("Ranged") => CombatType::Ranged,
                                _ => CombatType::Melee,
                            };

                            let damage = args.damage.unwrap_or(10.0);
                            let attack_stats = Some(AttackStats {
                                damage: damage,
                                range: if combat_type == CombatType::Melee { 2.0 } else { 15.0 },
                                cooldown: 1.5,
                                wind_up_time: 0.5,
                                recovery_time: 0.5,
                            });

                            let behavior_config = BehaviorConfig {
                                aggressiveness: args.aggressiveness.unwrap_or(0.5),
                                combat_type: combat_type,
                                wander_radius: args.wander_radius.unwrap_or(10.0),
                                wander_speed: args.wander_speed.unwrap_or(2.0),
                                detection_radius: args.detection_radius.unwrap_or(15.0),
                                melee_stats: if combat_type == CombatType::Melee { attack_stats } else { None },
                                ranged_stats: if combat_type == CombatType::Ranged { attack_stats } else { None },
                            };

                            let renderer_state = editor.renderer_state.as_mut().unwrap();
                            let gpu_resources = editor.gpu_resources.as_ref().unwrap();
                            let camera = editor.camera.as_ref().unwrap();

                            handle_add_npc(
                                renderer_state,
                                &gpu_resources.device,
                                &gpu_resources.queue,
                                project_id,
                                args.asset_id.clone(),
                                component_id.clone(),
                                asset_file_name,
                                model_iso,
                                model_scale,
                                camera,
                                None, // Script state
                                behavior_config.clone()
                            ).await;

                            // Update SavedState
                            if let Some(saved_state) = editor.saved_state.as_mut() {
                                if let Some(level) = saved_state.levels.as_mut().and_then(|l| l.get_mut(0)) {
                                    let new_component = ComponentData {
                                        id: component_id,
                                        kind: Some(ComponentKind::NPC),
                                        asset_id: args.asset_id.clone(),
                                        generic_properties: GenericProperties {
                                            name: "New NPC".to_string(),
                                            position: pos,
                                            rotation: rot,
                                            scale: scale,
                                        },
                                        npc_properties: Some(NPCProperties {
                                            model_id: args.asset_id,
                                            behavior: behavior_config,
                                        }),
                                        ..Default::default()
                                    };
                                    
                                    if let Some(components) = level.components.as_mut() {
                                        components.push(new_component);
                                    } else {
                                        level.components = Some(vec![new_component]);
                                    }
                                }
                                saved_state_clone = Some(saved_state.clone());
                            }
                        } else {
                            log!("Asset not found for NPC: {}", args.asset_id);
                        }
                    }
                }
            }
        }
    } else if tool_call.function.name == "saveScript" {
        log!("Saving script...");
        let args: Result<SaveScriptArgs, _> = serde_json::from_str(&tool_call.function.arguments);
        if let Ok(args) = args {
            // Update the component script path if provided
            if let Some(component_id) = &args.componentId {
                if let Some(pipeline_arc_val) = pipeline_store.get() {
                    if let Some(pipeline_arc) = pipeline_arc_val.as_ref() {
                        let mut pipeline = pipeline_arc.borrow_mut();
                        if let Some(editor) = pipeline.export_editor.as_mut() {
                            // Update SavedState
                            if let Some(saved_state) = editor.saved_state.as_mut() {
                                if let Some(level) = saved_state.levels.as_mut().and_then(|l| l.get_mut(0)) {
                                    if let Some(components) = level.components.as_mut() {
                                        if let Some(component) = components.iter_mut().find(|c| c.id == *component_id) {
                                            let script_path = format!("scripts/{}", args.filename);
                                            component.rhai_script_path = Some(script_path);
                                        }
                                    }
                                }
                                saved_state_clone = Some(saved_state.clone());
                            }
                        }
                    }
                }
            }
            
             // We need the project path. It's in `selected_project`.
            let project_path = selected_project.get_untracked().map(|p| p.path).unwrap_or_default();
            
            if !project_path.is_empty() {
                let url = format!("{}/api/save-script", get_api_url());
                let body = serde_json::json!({
                    "projectPath": project_path,
                    "filename": args.filename,
                    "content": args.content
                });
                
                spawn_local(async move {
                    let _ = Request::post(&url)
                        .json(&body)
                        .expect("Couldn't make post body")
                        .send()
                        .await;
                });
            }
        }
    } else if tool_call.function.name == "generateHeightmap" {
        log!("Generating heightmap...");
        let args: Result<GenerateHeightmapArgs, _> = serde_json::from_str(&tool_call.function.arguments);
        if let Ok(args) = args {
            if let Some(pipeline_arc_val) = pipeline_store.get() {
                if let Some(pipeline_arc) = pipeline_arc_val.as_ref() {
                    let mut pipeline = pipeline_arc.borrow_mut();
                    if let Some(editor) = pipeline.export_editor.as_mut() {
                        // 1. Find existing landscape info
                        let mut existing_info = None;
                        
                        if let Some(target_id) = &args.component_id {
                             if let Some(saved_state) = editor.saved_state.as_ref() {
                                if let Some(levels) = saved_state.levels.as_ref() {
                                    if let Some(components) = levels.get(0).and_then(|l| l.components.as_ref()) {
                                        if let Some(component) = components.iter().find(|c| c.id == *target_id) {
                                             let position = component.generic_properties.position;
                                             let asset_id = component.asset_id.clone();
                                             
                                             if let Some(landscapes) = saved_state.landscapes.as_ref() {
                                                if let Some(landscape_data) = landscapes.iter().find(|l| l.id == asset_id) {
                                                    if let Some(heightmap_file) = &landscape_data.heightmap {
                                                        existing_info = Some((position, asset_id, heightmap_file.fileName.clone()));
                                                    }
                                                }
                                             }
                                        }
                                    }
                                }
                             }
                        } else {
                             // Try to find first existing landscape if no ID specified
                             if let Some(saved_state) = editor.saved_state.as_ref() {
                                if let Some(levels) = saved_state.levels.as_ref() {
                                    if let Some(components) = levels.get(0).and_then(|l| l.components.as_ref()) {
                                        if let Some(component) = components.iter().find(|c| c.kind == Some(entropy_engine::helpers::saved_data::ComponentKind::Landscape)) {
                                            let position = component.generic_properties.position;
                                            let asset_id = component.asset_id.clone();
                                            if let Some(landscapes) = saved_state.landscapes.as_ref() {
                                                if let Some(landscape_data) = landscapes.iter().find(|l| l.id == asset_id) {
                                                    if let Some(heightmap_file) = &landscape_data.heightmap {
                                                        existing_info = Some((position, asset_id, heightmap_file.fileName.clone()));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        let (position, asset_id, filename) = if let Some(info) = existing_info {
                            info
                        } else {
                            log!("Creating new landscape info.");
                            let new_asset_id = Uuid::new_v4().to_string();
                            ([0.0, 0.0, 0.0], new_asset_id, format!("heightmap_{}.png", Uuid::new_v4()))
                        };

                        let width = 1024;
                        let height = 1024;
                        let mut generator = HeightmapGenerator::new(width, height)
                                                                    .with_scale(1024.0)
                                                                    .with_octaves(8)
                                                                    .with_persistence(0.5)
                                                                    .with_seed(42);
                        
                        if let Some(seed) = args.seed { generator = generator.with_seed(seed); }
                        if let Some(scale) = args.scale { generator = generator.with_scale(scale); }
                        if let Some(persistence) = args.persistence { generator = generator.with_persistence(persistence); }
                        if let Some(lacunarity) = args.lacunarity { generator = generator.with_lacunarity(lacunarity); }

                        if let Some(features) = args.features {
                            for f in features {
                                let f_type = match f.r#type.as_str() {
                                    "Mountain" => FeatureType::Mountain,
                                    "Valley" => FeatureType::Valley,
                                    "Plateau" => FeatureType::Plateau,
                                    "Ridge" => FeatureType::Ridge,
                                    _ => FeatureType::Mountain,
                                };
                                let falloff = match f.falloff.as_str() {
                                    "Linear" => FalloffType::Linear,
                                    "Smooth" => FalloffType::Smooth,
                                    "Gaussian" => FalloffType::Gaussian,
                                    _ => FalloffType::Smooth,
                                };
                                let mut feature = TerrainFeature::new(
                                    (f.center[0], f.center[1]),
                                    f.radius,
                                    f.intensity,
                                    falloff,
                                    f_type
                                );
                                if let Some(ft) = f.flat_top { feature = feature.with_flat_top(ft); }
                                if let Some(t) = f.transition { feature = feature.with_transition(t); }
                                generator.add_feature(feature);
                            }
                        }

                        let img = generator.generate();
                        
                        // Convert to PNG bytes
                        let mut png_bytes: Vec<u8> = Vec::new();
                        let _ = image::ImageBuffer::from_raw(width, height, img.clone().into_raw())
                            .map(|buf: image::ImageBuffer<image::Luma<u16>, Vec<u16>>| {
                                let dyn_img = image::DynamicImage::ImageLuma16(buf);
                                let _ = dyn_img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png);
                            });

                        // Upload to API
                        // We need the project path. It's in `selected_project`.
                        let project_path = selected_project.get_untracked().map(|p| p.path).unwrap_or_default();
                        
                        if !png_bytes.is_empty() && !project_path.is_empty() {
                            let file_part = FormData::new().expect("FormData error");
                            // file_part.append("projectPath", &project_path);
                            
                            // Manually constructing body or using a way to send FormData compatible with the server
                            // Gloo-net Request supports body.
                            
                            // Let's use web_sys for FormData as it's cleaner in browser context
                            let form_data = web_sys::FormData::new().unwrap();
                            form_data.append_with_str("projectPath", &project_path).unwrap();
                            form_data.append_with_str("landscapeAssetId", &asset_id).unwrap();
                            form_data.append_with_str("filename", &filename).unwrap();
                            
                            let uint8_array = js_sys::Uint8Array::from(&png_bytes[..]);
                            let blob_parts = js_sys::Array::new();
                            blob_parts.push(&uint8_array);
                            let blob = web_sys::Blob::new_with_u8_array_sequence(&blob_parts).unwrap();
                            form_data.append_with_blob("file", &blob).unwrap();

                            let url = format!("{}/api/save-heightmap", get_api_url());
                            spawn_local(async move {
                                let _ = Request::post(&url)
                                    .body(form_data)
                                    .expect("Couldn't make post body")
                                    .send()
                                    .await;
                            });
                        }

                        // Update In-Memory
                        let height_data: Vec<f32> = img.pixels().map(|p| p.0[0] as f32 / 65535.0).collect();

                        let landscape_data = generate_landscape_data(
                            width as usize,
                            height as usize,
                            height_data,
                            1024.0 * 4.0, // size match existing default or reasonable size
                            1024.0 * 4.0,
                            150.0 * 4.0, // height scale
                        );

                        if let Some(renderer_state) = editor.renderer_state.as_mut() {
                            // Clear existing landscapes
                            renderer_state.landscapes.clear();
                            renderer_state.terrain_managers.clear();
                            
                            // Add new landscape with CORRECT position
                            let device = &editor.gpu_resources.as_ref().unwrap().device;
                            let queue = &editor.gpu_resources.as_ref().unwrap().queue;
                            let camera = editor.camera.as_ref().unwrap();
                            
                            renderer_state.add_landscape(
                                device,
                                queue,
                                &"generated_landscape".to_string(),
                                &landscape_data,
                                position, // Use the position from saved_state
                                camera
                            );
                            
                            log!("Heightmap generated and loaded!");
                            
                            if let Some(saved_state) = editor.saved_state.as_mut() {
                                saved_state_clone = Some(saved_state.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(saved_state) = saved_state_clone {
        spawn_local(async move {
            let _ = save_project(&project_id, &saved_state).await;
        });
    }

    "{\"success\": true}".to_string()
}

#[component]
pub fn ProjectCanvas(
    selected_project: ReadSignal<Option<Project>>,
    pipeline_store: LocalResource<Option<Rc<RefCell<ExportPipeline>>>>,
    is_initialized: ReadSignal<bool>,
    set_is_initialized: WriteSignal<bool>,
) -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    
    create_effect(move |_| {
        let canvas = canvas_ref.get();
        if canvas.is_none() {
            return;
        }
        let canvas = canvas.expect("canvas should be loaded");

        if let Some(project) = selected_project.get() {
            let project_data = project.clone();
            if let Some(pipeline_arc) = pipeline_store.get() {
                if let Some(pipeline_arc) = pipeline_arc.as_ref() {
                    let pipeline_arc_clone = pipeline_arc.clone();
                    spawn_local(async move {
                        let mut pipeline_guard = pipeline_arc_clone.borrow_mut();

                        log!("initializing...");
                        
                        #[cfg(target_arch = "wasm32")]
                        pipeline_guard
                            .initialize(
                                Some(canvas),
                                WindowSize {
                                    width: 1024,
                                    height: 768,
                                },
                                Vec::new(),
                                SavedTimelineStateConfig {
                                    timeline_sequences: Vec::new(),
                                },
                                1024,
                                768,
                                Uuid::new_v4().to_string(),
                                false,
                            )
                            .await;

                        log!("loading project...");

                        let editor = pipeline_guard.export_editor.as_mut().expect("Couldn't get editor");
                        // Manually load saved state
                        if let Some(saved_data) = project_data.saved_data {
                            editor.saved_state = Some(saved_data.clone());
                            // We also need to trigger loading of assets/models based on this saved state
                            // But load_project usually does that. 
                            // Since we can't easily call load_project which might do FS ops,
                            // we might need to rely on what's available or implement a lighter load_project.
                            // For now, let's assume placing the project state is enough or call a helper if available.
                            
                            // Re-implement basic loading logic from place_project/load_project if needed
                            // But place_project is available.
                             place_project(editor, &project_data.id, saved_data.clone()).await;
                        }

                        log!("configuring surface...");

                        let editor = pipeline_guard.export_editor.as_ref().expect("Couldn't get editor");
                        let camera = editor.camera.as_ref().expect("Couldn't get camera");
                        let gpu_resources = pipeline_guard.gpu_resources.as_ref().expect("Couldn't get gpu resources");
                        let surface = gpu_resources.surface.as_ref().expect("Couldn't get surface").clone();
                        let size = camera.viewport.window_size.clone();

                        let swapchain_format = wgpu::TextureFormat::Rgba8Unorm;
                        let surface_config = wgpu::SurfaceConfiguration {
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                            format: swapchain_format,
                            width: size.width,
                            height: size.height,
                            present_mode: wgpu::PresentMode::Fifo,
                            alpha_mode: wgpu::CompositeAlphaMode::PreMultiplied,
                            view_formats: vec![],
                            desired_maximum_frame_latency: 2
                        };

                        surface.configure(&gpu_resources.device, &surface_config);

                        log!("Setup Complete!");

                        set_is_initialized.set(true);
                    });
                }
            }
        }
    });

    let Pausable { pause, resume, is_active } = use_raf_fn(move |_| {
        if is_initialized.get() {
            if let Some(pipeline) = pipeline_store.get_untracked() {
                if let Some(pipeline_arc) = pipeline.as_ref() {
                    let mut pipeline = pipeline_arc.borrow_mut();
                    let gpu_resources = match pipeline.gpu_resources.as_ref() {
                        Some(res) => res.clone(),
                        None => return,
                    };

                    let surface = match gpu_resources.surface.as_ref() {
                        Some(s) => s,
                        None => return,
                    };

                    let output = match surface.get_current_texture() {
                        Ok(o) => o,
                        Err(e) => {
                            log!("Failed to get current texture: {:?}", e);
                            return;
                        }
                    };

                    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
                    let now = js_sys::Date::now();
                    pipeline.render_frame(Some(&view), now, false);
                    output.present();
                }   
            }
        }
    });

    view! {
        <section>
            <Show
                when=move || { !is_initialized.get() }
                fallback=|| view! { <span>{""}</span> }
            >
                <span>{"Initializing..."}</span>
            </Show>
            <canvas 
                id="project-canvas" 
                node_ref=canvas_ref 
                tabindex="0"
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    let key = ev.key();
                    if let Some(pipeline_store_val) = pipeline_store.get() {
                        if let Some(pipeline_arc) = pipeline_store_val.as_ref() {
                            let mut pipeline = pipeline_arc.borrow_mut();
                            if let Some(editor) = pipeline.export_editor.as_mut() {
                                let camera = editor.camera.as_ref().expect("Couldn't get camera");

                                // log!("handle_key_press {:?} {:?} {:?}", key, camera.position, camera.direction);

                                handle_key_press(editor, key.as_str(), true);
                            }
                        }
                    }
                }
                on:mousemove=move |ev: web_sys::MouseEvent| {
                    
                        if let Some(pipeline_store_val) = pipeline_store.get() {
                            if let Some(pipeline_arc) = pipeline_store_val.as_ref() {
                                let mut pipeline = pipeline_arc.borrow_mut();
                                if let Some(editor) = pipeline.export_editor.as_mut() {
                                    let canv = canvas_ref.get();
                                    let canv = canv.as_ref().expect("Couldn't get canvas ref");
                                    let rect = canv.get_bounding_client_rect();

                                    let dx = ev.movement_x() as f32;
                                    let dy = ev.movement_y() as f32;

                                    // log!("handle_mouse_move_on_shift {:?} {:?}", dx, dy);

                                    let left_mouse_pressed = ev.button() == 0;
                                    
                                    handle_mouse_move(
                                        left_mouse_pressed,
                                        EntropyPosition {
                                            x: ev.client_x() as f32 - rect.left() as f32,
                                            y: ev.client_y() as f32 - rect.top() as f32,
                                        }, 
                                        dx, 
                                        dy, 
                                        editor
                                    );
                                    
                                    if ev.shift_key() {
                                        handle_mouse_move_on_shift(dx, dy, editor);
                                    }
                                }
                            }
                        }
                   
                }
            />
        </section>
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (show_chat, set_show_chat) = signal(false);
    let (selected_project, set_selected_project) = signal::<Option<Project>>(None);
    let (current_session, set_current_session) = signal::<Option<ChatSession>>(None);
    let (refetch_projects, set_refetch_projects) = signal(false);
    let (refetch_messages, set_refetch_messages) = signal(false);
    let (is_initialized, set_is_initialized) = signal(false);
    let (message_content, set_message_content) = signal(String::new());
    let (local_messages, set_local_messages) = signal(Vec::<ChatMessage>::new());
    let (active_editor_tab, set_active_editor_tab) = signal(0);
    let input_ref: NodeRef<leptos::html::Input> = NodeRef::new();

    // DO NOT use "create_resource" as the leptos_reactive crate is deprecated, LocalResource is the recommended way for a client-side Tauri + Leptos app
    let projects_resource: LocalResource<Result<Vec<ProjectInfo>, String>> = LocalResource::new(
        // move || refetch_projects.get(),
        move || async move {
            if refetch_projects.get() {
                set_refetch_projects.update_untracked(|val| *val = false);
            }
            Request::get(&format!("{}/api/projects", get_api_url()))
                .send()
                .await
                .map_err(|e| e.to_string())?
                .json()
                .await
                .map_err(|e| e.to_string())
        },
    );

    let pipeline_store: LocalResource<Option<Rc<RefCell<ExportPipeline>>>> =
        LocalResource::new(
        move || async move {
            Some(Rc::new(RefCell::new(ExportPipeline::new())))
        },
    );

    let messages_resource: LocalResource<std::result::Result<Vec<ChatMessage>, String>> = LocalResource::new(
    move || async move { 
            if refetch_messages.get() {
                set_refetch_messages.update_untracked(|val| *val = false);
            }
            let session_id = current_session.get().map(|s| s.id);
            if let Some(session_id) = session_id {
                let url = format!("{}/api/sessions/{}/messages", get_api_url(), session_id);
                let mut remote: Vec<ChatMessage> = Request::get(&url)
                    .send()
                    .await
                    .map_err(|e| e.to_string())?
                    .json()
                    .await
                    .map_err(|e| e.to_string())?;
                
                // Combine with local messages here
                remote.extend(local_messages.get_untracked().iter().cloned());
                Ok(remote)
            }
            else {
                Ok(local_messages.get_untracked())
            }
        },
    );

    let open_project_chat = move |project_info: ProjectInfo| {
        spawn_local(async move {
            // 1. Fetch full project details (including savedData)
            let project_res = Request::get(&format!("{}/api/projects/{}", get_api_url(), project_info.id))
                .send()
                .await;
            
            if let Ok(resp) = project_res {
                if let Ok(project) = resp.json::<Project>().await {
                    
                    // 2. Create or get session
                    // For now, always create a new session or pick the last one if we implemented logic for it.
                    // The simplest is to create a new session for this "chat instance".
                    // Or if we want persistent chat, we should list sessions and pick one.
                    // Let's create a new one for simplicity as per requirements "Start New Project" or "Chat with...".
                    // The old code returned `session` from `open_project_chat`.
                    
                    let session_res = Request::post(&format!("{}/api/sessions", get_api_url()))
                        .json(&serde_json::json!({ "projectId": project.id }))
                        .expect("Couldn't get json")
                        .send()
                        .await;

                    if let Ok(session_resp) = session_res {
                        if let Ok(session) = session_resp.json::<ChatSession>().await {
                            log!("Setting up chat {:?} {:?}", project.id, session.id);

                            set_selected_project.update(|val| *val = Some(project));
                            set_current_session.update(|val| *val = Some(session));
                            set_show_chat.update(|val| *val = true);
                        } else {
                            log!("Failed to parse session response");
                        }
                    } else {
                        log!("Failed to create session");
                    }
                } else {
                    log!("Failed to parse project response");
                }
            } else {
                log!("Failed to fetch project");
            }
        });
    };

    let send_message = move |pipeline_store: LocalResource<Option<Rc<RefCell<ExportPipeline>>>>| {
        if let Some(session) = current_session.get() {
            let content = message_content.get(); // Get value before spawn
            set_local_messages.set(Vec::new());
            
            // Get current saved state from pipeline
            let mut current_saved_state = None;
            if let Some(pipeline_arc_val) = pipeline_store.get_untracked() {
                if let Some(pipeline_arc) = pipeline_arc_val.as_ref() {
                    let mut pipeline = pipeline_arc.borrow_mut();
                    if let Some(editor) = pipeline.export_editor.as_mut() {
                        current_saved_state = editor.saved_state.clone();
                    }
                }
            }

            spawn_local(async move {
                let session_id = session.id.clone();
                let project_id = selected_project.get().as_ref().expect("Couldn't get selected project").id.clone();
                
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                struct SendMessageArgs {
                    role: String,
                    content: String,
                    #[serde(skip_serializing_if = "Option::is_none")]
                    tool_call_id: Option<String>,
                    #[serde(rename = "saved_state")]
                    saved_state: Option<SavedState>,
                }

                let body = SendMessageArgs {
                    role: "user".to_string(),
                    content,
                    tool_call_id: None,
                    saved_state: current_saved_state,
                };

                set_message_content.update(|val| *val = String::new());
                if let Some(input) = input_ref.get_untracked() {
                    input.set_value("");
                }

                let url = format!("{}/api/sessions/{}/messages", get_api_url(), session_id);
                let response = Request::post(&url)
                    .json(&body)
                    .expect("Couldn't get json")
                    .send()
                    .await;

                if let Ok(resp) = response {
                    if let Ok(message) = resp.json::<ChatMessage>().await {
                        log!("Response okay");

                        if let Some(tool_calls) = message.tool_calls {
                            log!("Tool calls...");

                            let tool_calls_data = tool_calls.clone();

                            set_local_messages.update(|messages| {
                                for tool_call in tool_calls_data {
                                    messages.push(ChatMessage {
                                        id: Uuid::new_v4().to_string(),
                                        role: "system".to_string(),
                                        content: Some(format!("Implementing changes... {:?} {:?}", tool_call.function.name, tool_call.function.arguments)),
                                        tool_call_id: None,
                                        tool_calls: None,
                                    });
                                }
                            });

                            for tool_call in tool_calls {
                                let _ = execute_tool_call(&tool_call, pipeline_store, project_id.clone(), selected_project).await;
                            }
                        }
                    }
                }
                
                set_refetch_messages.update(|val| *val = true);
            });
        }
    };

    view! {
        <main class="container">
            <Show
                when=move || { !show_chat.get() }
                fallback=|| view! { <span>{""}</span> }
            >
            <section class="inbox">
                <h2>{"Welcome, Alex"}</h2>
                <h1>{"Projects"}</h1>

                <button class="primary-btn">{"Start New Project"}</button>

                <span class="instructions">{"Chat with apps / projects or other content and add people or bots to the conversation. Optionally mark as public."}</span>

                <section class="more">
                    <div class="">
                        <h3>{"Your Files"}</h3>
                        <Suspense fallback=move || {
                            view! { <div>"Loading projects..."</div> }
                        }>
                            <div class="files-inner">
                                {move || {
                                    projects_resource
                                        .get()
                                        .map(|project_items| {
                                            let project_items = project_items.as_deref();
                                            
                                            if let Ok(items) = project_items {
                                                if items.is_empty() {
                                                    return view! { <p>{"No projects found."}</p> }.into_view().into_any();
                                                }

                                                items
                                                    .into_iter()
                                                    .map(|project| {
                                                        let p = project.clone();
                                                        view! {
                                                            <div class="inbox-item" on:click=move |_| {
                                                                open_project_chat(p.clone());
                                                            }>
                                                                <div class="item-icon">
                                                                    <Icon icon=GAME_CONTROLLER color="#AE2983" weight=IconWeight::Fill size="32px" />
                                                                </div>

                                                                <div class="item-meta">
                                                                    <div class="item-title">
                                                                        {project.name.clone()}
                                                                    </div>

                                                                    <div class="item-type">
                                                                        {"Project"} // Hardcoded for now
                                                                    </div>

                                                                    <div class="item-date">
                                                                        {"N/A"} // No date in ProjectInfo yet
                                                                    </div>
                                                                </div>
                                                            </div>
                                                        }
                                                    })
                                                    .collect_view().into_any()
                                            } else {
                                                view! { <p>{"Error."}</p> }.into_view().into_any()
                                            }
                                        })
                                }}
                            </div>
                        </Suspense>
                    </div>
                </section>
            </section>
            </Show>

            <Show
                when=move || { show_chat.get() }
                fallback=|| view! { <span>{""}</span> }
            >
            <section class="chat-view">
                <div class="chat-pane">
                    <h3>{"Chat with "} {move || selected_project.get().map(|p| p.name).unwrap_or_default()}</h3>
                    <button on:click=move |_| set_show_chat.set(false)>{"Close Chat"}</button>
                    <div class="chat-messages">
                        <Suspense fallback=move || {
                            view! { <div>"Loading messages..."</div> }
                        }>
                            {move || {
                                messages_resource.get().and_then(|result| {
                                    result.as_ref().ok().map(|messages| {
                                        messages
                                            .into_iter()
                                            .map(|message| {
                                                view! {
                                                    <div class="chat-message">
                                                        <strong>{message.role.clone()}":"</strong>
                                                        <span>{message.content.clone().unwrap_or_default()}</span>
                                                    </div>
                                                }
                                            })
                                            .collect_view()
                                    })
                                })
                            }}
                        </Suspense>
                        // Recommendations
                        // <button class="primary-btn">"Let's turn the ocean blood red and more intense"</button>
                        // <button class="primary-btn">"Please move the sword near the shoreline"</button>
                        // <button class="primary-btn">"Let's turn the grass blue and more windy"</button>
                        // <button class="primary-btn">"Can we create some dialogue between Enemy 1 and the Player?"</button>
                        <span>"Browse the scene preview with shift-click and the wasd keys, with the preview selected."</span>
                        <span>"You can also drop models and images here in the chat, but remember to let Chat know if you are sending textures, heightmaps, or something else so it gets organized properly"</span>
                        <span>"Feel free to chat about point lights, models, collectables, game behaviors, NPCs, particle effects, dialogue, quests, water, trees, grass, new terrains, or anything else that you would like to see in your game world"</span>
                    </div>
                    <div class="chat-input">
                        <input
                            type="text"
                            placeholder="Type a message..."
                            node_ref=input_ref
                            on:input=move |ev| {
                                set_message_content.set(event_target_value(&ev));
                            }
                        />
                        <button on:click=move |_| send_message(pipeline_store)>{"Send"}</button>
                    </div>
                </div>
                <div class="content-preview-pane">
                    <h3>{"Content Preview: "} {move || selected_project.get().map(|p| p.name).unwrap_or_default()}</h3>
                    <ProjectCanvas 
                        selected_project={selected_project} 
                        pipeline_store={pipeline_store}
                        is_initialized={is_initialized}
                        set_is_initialized={set_is_initialized} 
                    />
                    
                    <div class="editor-tabs">
                         <button 
                            class:active=move || active_editor_tab.get() == 0
                            on:click=move |_| set_active_editor_tab.set(0)
                        >{"Components"}</button>
                        <button 
                            class:active=move || active_editor_tab.get() == 1
                            on:click=move |_| set_active_editor_tab.set(1)
                        >{"Assets"}</button>
                    </div>

                    <Show when=move || active_editor_tab.get() == 0>
                        <ComponentPropertiesEditor
                            pipeline_store={pipeline_store}
                            is_initialized={is_initialized}
                        />
                    </Show>
                    
                    <Show when=move || active_editor_tab.get() == 1>
                        <AssetsBrowser
                            pipeline_store={pipeline_store}
                            is_initialized={is_initialized}
                            project_path=Signal::derive(move || selected_project.get().map(|p| p.path))
                            project_id=Signal::derive(move || selected_project.get().map(|p| p.id))
                        />
                    </Show>
                </div>
            </section>
            </Show>
        </main>
    }
}

