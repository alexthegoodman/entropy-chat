use entropy_engine::core::pipeline::ExportPipeline;
use entropy_engine::core::editor::WindowSize;
use entropy_engine::helpers::load_project::place_project;
use entropy_engine::helpers::saved_data::{ComponentData, SavedState};
use entropy_engine::helpers::timelines::SavedTimelineStateConfig;
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
use entropy_engine::handlers::{EntropyPosition, handle_key_press, handle_mouse_move, handle_mouse_move_on_shift};
use entropy_engine::water_plane::config::WaterConfig;
use std::time::{Duration, SystemTime};
use gloo_net::http::Request;

use crate::components::component_browser::ComponentPropertiesEditor;

async fn save_project(project_id: &str, saved_state: &SavedState) -> Result<(), String> {
    let url = format!("/api/projects/{}", project_id);
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
                                    // Assuming water config is part of saved_state (it should be in level components or global config)
                                    // If not, we might need to update where it is stored in SavedState.
                                    // For now, let's clone saved_state to trigger a save, assuming the water config changes are reflected in it 
                                    // OR we need to update the saved_state from the renderer_state.
                                    // But typically editor.saved_state is the source of truth for saving.
                                    // If water config is NOT in saved_state, saving it won't help unless we sync it.
                                    // Let's assume for now we just trigger save.
                                    saved_state_clone = Some(saved_state.clone());
                                }
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
    let input_ref: NodeRef<leptos::html::Input> = NodeRef::new();

    // DO NOT use "create_resource" as the leptos_reactive crate is deprecated, LocalResource is the recommended way for a client-side Tauri + Leptos app
    let projects_resource: LocalResource<Result<Vec<ProjectInfo>, String>> = LocalResource::new(
        // move || refetch_projects.get(),
        move || async move {
            if refetch_projects.get() {
                set_refetch_projects.update_untracked(|val| *val = false);
            }
            Request::get("/api/projects")
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
                let url = format!("/api/sessions/{}/messages", session_id);
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
            } else {
                Ok(local_messages.get_untracked())
            }
        },
    );

    let open_project_chat = move |project_info: ProjectInfo| {
        spawn_local(async move {
            // 1. Fetch full project details (including savedData)
            let project_res = Request::get(&format!("/api/projects/{}", project_info.id))
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
                    
                    let session_res = Request::post("/api/sessions")
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

                let url = format!("/api/sessions/{}/messages", session_id);
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
                                let _ = execute_tool_call(&tool_call, pipeline_store, project_id.clone()).await;
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
                    <ComponentPropertiesEditor
                        pipeline_store={pipeline_store}
                        is_initialized={is_initialized}
                    />
                </div>
            </section>
            </Show>
        </main>
    }
}

