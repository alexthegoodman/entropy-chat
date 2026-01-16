use entropy_engine::core::pipeline::ExportPipeline;
use entropy_engine::core::editor::WindowSize;
use entropy_engine::helpers::load_project::place_project;
use entropy_engine::helpers::saved_data::{CollectableType, ComponentData, ComponentKind};
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
use entropy_engine::helpers::saved_data::{
    GenericProperties,
    ModelProperties,
    NPCProperties,
    LandscapeProperties,
    LightProperties, 
    CollectableProperties,
    PlayerProperties, 
    ScatterSettings
};
use std::time::{Duration, SystemTime};

#[component]
pub fn ComponentPropertiesEditor(
    pipeline_store: LocalResource<Option<Rc<RefCell<ExportPipeline>>>>,
    is_initialized: ReadSignal<bool>,
) -> impl IntoView {
    let (selected_component_id, set_selected_component_id) = signal::<Option<String>>(None);
    let (components_list, set_components_list) = signal::<Vec<ComponentData>>(Vec::new());
    
    // Extract components in an effect to avoid borrow issues
    create_effect(move |_| {
        if is_initialized.get() {
            if let Some(pipeline) = pipeline_store.get() {
                if let Some(pipeline_arc) = pipeline.as_ref() {
                    let pipeline_guard = pipeline_arc.borrow();
                    if let Some(editor) = pipeline_guard.export_editor.as_ref() {
                        if let Some(saved_state) = editor.saved_state.as_ref() {
                            if let Some(level) = saved_state.levels.as_ref().and_then(|l| l.get(0)) {
                                if let Some(components) = level.components.as_ref() {
                                    set_components_list.set(components.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    });
    
    view! {
        <div class="component-editor">
            <h3>{"Components"}</h3>
            
            <div class="component-list">
                <Show
                    when=move || !components_list.get().is_empty()
                    fallback=|| view! {<div class="no-components">{"No components loaded"}</div>}
                >
                    <For
                        each=move || components_list.get()
                        key=|component| component.id.clone()
                        children=move |component: ComponentData| {
                            let comp_id = component.id.clone();
                            let comp_name = component.generic_properties.name.clone();
                            let comp_kind = component.kind.clone();
                            let comp_id_clone = comp_id.clone();
                            let is_selected = move || selected_component_id.get() == Some(comp_id.clone());
                            
                            view! {
                                <div class="component-item">
                                    <div 
                                        class="component-header"
                                        class:selected=is_selected.clone()
                                        on:click=move |_| {
                                            set_selected_component_id.set(Some(comp_id_clone.clone()));
                                        }
                                    >
                                        <strong>{comp_name}</strong>
                                        <small>{" ("}{format!("{:?}", comp_kind.unwrap_or(ComponentKind::Model))}{")"}</small>
                                    </div>
                                    
                                    <Show when=is_selected>
                                        <ComponentPropertyPanel 
                                            component=component.clone()
                                        />
                                    </Show>
                                </div>
                            }
                        }
                    />
                </Show>
            </div>
        </div>
    }
}

#[component]
fn ComponentPropertyPanel(
    component: ComponentData,
) -> impl IntoView {
    view! {
        <div class="property-panel">
            // Generic Properties (always present)
            <GenericPropertiesPanel 
                generic=component.generic_properties.clone() 
                component_id=component.id.clone()
            />
            
            // Component-specific properties
            {match component.kind {
                Some(ComponentKind::Model) => view! {
                    <ModelPropertiesPanel 
                        properties=component.model_properties.clone().unwrap_or_default()
                        component_id=component.id.clone()
                    />
                }.into_view().into_any(),
                
                Some(ComponentKind::NPC) => view! {
                    <NPCPropertiesPanel 
                        properties=component.npc_properties.clone().unwrap_or_default()
                        component_id=component.id.clone()
                    />
                }.into_view().into_any(),
                
                Some(ComponentKind::Landscape) => view! {
                    <LandscapePropertiesPanel 
                        properties=component.landscape_properties.clone().unwrap_or_default()
                        component_id=component.id.clone()
                    />
                }.into_view().into_any(),
                
                Some(ComponentKind::PointLight) => view! {
                    <LightPropertiesPanel 
                        properties=component.light_properties.clone().unwrap_or_default()
                        component_id=component.id.clone()
                    />
                }.into_view().into_any(),
                
                Some(ComponentKind::WaterPlane) => view! {
                    <WaterPropertiesPanel 
                        properties=component.water_properties.clone()
                        component_id=component.id.clone()
                    />
                }.into_view().into_any(),
                
                Some(ComponentKind::Collectable) => view! {
                    <CollectablePropertiesPanel 
                        properties=component.collectable_properties.clone().unwrap_or_default()
                        component_id=component.id.clone()
                    />
                }.into_view().into_any(),
                
                Some(ComponentKind::PlayerCharacter) => view! {
                    <PlayerPropertiesPanel 
                        properties=component.player_properties.clone().unwrap_or_default()
                        component_id=component.id.clone()
                    />
                }.into_view().into_any(),
                
                _ => view! { <div></div> }.into_view().into_any(),
            }}
            
            // Scatter settings (optional for any component)
            {component.scatter.as_ref().map(|scatter| view! {
                <ScatterPropertiesPanel 
                    settings=scatter.clone()
                    component_id=component.id.clone()
                />
            })}
        </div>
    }
}

#[component]
fn GenericPropertiesPanel(
    generic: GenericProperties,
    component_id: String,
) -> impl IntoView {
    let (is_open, set_is_open) = signal(true);
    
    view! {
        <details open=is_open.get() on:toggle=move |_| set_is_open.update(|v| *v = !*v)>
            <summary>{"Generic Properties"}</summary>
            <div class="property-group">
                <label>
                    {"Name: "}
                    <input type="text" value=generic.name />
                </label>
                
                <label>
                    {"Position X: "}
                    <input type="number" step="0.1" value=generic.position[0] />
                </label>
                <label>
                    {"Position Y: "}
                    <input type="number" step="0.1" value=generic.position[1] />
                </label>
                <label>
                    {"Position Z: "}
                    <input type="number" step="0.1" value=generic.position[2] />
                </label>
                
                <label>
                    {"Rotation X: "}
                    <input type="number" step="1" value=generic.rotation[0] />
                </label>
                <label>
                    {"Rotation Y: "}
                    <input type="number" step="1" value=generic.rotation[1] />
                </label>
                <label>
                    {"Rotation Z: "}
                    <input type="number" step="1" value=generic.rotation[2] />
                </label>
                
                <label>
                    {"Scale X: "}
                    <input type="number" step="0.1" value=generic.scale[0] />
                </label>
                <label>
                    {"Scale Y: "}
                    <input type="number" step="0.1" value=generic.scale[1] />
                </label>
                <label>
                    {"Scale Z: "}
                    <input type="number" step="0.1" value=generic.scale[2] />
                </label>
            </div>
        </details>
    }
}

#[component]
fn ModelPropertiesPanel(
    properties: ModelProperties,
    component_id: String,
) -> impl IntoView {
    let (is_open, set_is_open) = signal(false);
    
    view! {
        <details open=is_open.get() on:toggle=move |_| set_is_open.update(|v| *v = !*v)>
            <summary>{"Model Properties"}</summary>
            <div class="property-group">
                <p class="info-text">{"Model components use asset_id for the model reference"}</p>
            </div>
        </details>
    }
}

#[component]
fn NPCPropertiesPanel(
    properties: NPCProperties,
    component_id: String,
) -> impl IntoView {
    let (is_open, set_is_open) = signal(false);
    
    view! {
        <details open=is_open.get() on:toggle=move |_| set_is_open.update(|v| *v = !*v)>
            <summary>{"NPC Properties"}</summary>
            <div class="property-group">
                <label>
                    {"Model ID: "}
                    <input type="text" value=properties.model_id />
                </label>
            </div>
        </details>
    }
}

#[component]
fn LandscapePropertiesPanel(
    properties: LandscapeProperties,
    component_id: String,
) -> impl IntoView {
    let (is_open, set_is_open) = signal(false);
    
    view! {
        <details open=is_open.get() on:toggle=move |_| set_is_open.update(|v| *v = !*v)>
            <summary>{"Landscape Properties"}</summary>
            <div class="property-group">
                <h4>{"Regular Textures"}</h4>
                <label>
                    {"Primary Texture ID: "}
                    <input type="text" value=properties.primary_texture_id.unwrap_or_default() />
                </label>
                <label>
                    {"Rockmap Texture ID: "}
                    <input type="text" value=properties.rockmap_texture_id.unwrap_or_default() />
                </label>
                <label>
                    {"Soil Texture ID: "}
                    <input type="text" value=properties.soil_texture_id.unwrap_or_default() />
                </label>
                
                <h4>{"PBR Textures"}</h4>
                <label>
                    {"Primary PBR Texture ID: "}
                    <input type="text" value=properties.primary_pbr_texture_id.unwrap_or_default() />
                </label>
                <label>
                    {"Rockmap PBR Texture ID: "}
                    <input type="text" value=properties.rockmap_pbr_texture_id.unwrap_or_default() />
                </label>
                <label>
                    {"Soil PBR Texture ID: "}
                    <input type="text" value=properties.soil_pbr_texture_id.unwrap_or_default() />
                </label>
            </div>
        </details>
    }
}

#[component]
fn LightPropertiesPanel(
    properties: LightProperties,
    component_id: String,
) -> impl IntoView {
    let (is_open, set_is_open) = signal(false);
    
    view! {
        <details open=is_open.get() on:toggle=move |_| set_is_open.update(|v| *v = !*v)>
            <summary>{"Light Properties"}</summary>
            <div class="property-group">
                <label>
                    {"Intensity: "}
                    <input type="number" step="0.1" min="0" value=properties.intensity />
                </label>
                
                <label>
                    {"Color R: "}
                    <input type="number" step="0.01" min="0" max="1" value=properties.color[0] />
                </label>
                <label>
                    {"Color G: "}
                    <input type="number" step="0.01" min="0" max="1" value=properties.color[1] />
                </label>
                <label>
                    {"Color B: "}
                    <input type="number" step="0.01" min="0" max="1" value=properties.color[2] />
                </label>
                <label>
                    {"Color A: "}
                    <input type="number" step="0.01" min="0" max="1" value=properties.color[3] />
                </label>
                
                <div class="color-preview" style=format!(
                    "background-color: rgba({}, {}, {}, {}); width: 50px; height: 50px; border: 1px solid #ccc;",
                    (properties.color[0] * 255.0) as u8,
                    (properties.color[1] * 255.0) as u8,
                    (properties.color[2] * 255.0) as u8,
                    properties.color[3]
                )></div>
            </div>
        </details>
    }
}

#[component]
fn WaterPropertiesPanel(
    properties: Option<WaterConfig>,
    component_id: String,
) -> impl IntoView {
    let (is_open, set_is_open) = signal(false);
    
    view! {
        <details open=is_open.get() on:toggle=move |_| set_is_open.update(|v| *v = !*v)>
            <summary>{"Water Properties"}</summary>
            <div class="property-group">
                {properties.map(|_config| view! {
                    <p class="info-text">{"Water configuration available (structure depends on WaterConfig)"}</p>
                }).unwrap_or_else(|| view! {
                    <p class="info-text">{"No water configuration"}</p>
                })}
            </div>
        </details>
    }
}

#[component]
fn CollectablePropertiesPanel(
    properties: CollectableProperties,
    component_id: String,
) -> impl IntoView {
    let (is_open, set_is_open) = signal(false);
    
    view! {
        <details open=is_open.get() on:toggle=move |_| set_is_open.update(|v| *v = !*v)>
            <summary>{"Collectable Properties"}</summary>
            <div class="property-group">
                <label>
                    {"Model ID: "}
                    <input type="text" value=properties.model_id.unwrap_or_default() placeholder="(uses sphere if empty)" />
                </label>
                
                <label>
                    {"Type: "}
                    <select>
                        <option selected=properties.collectable_type.is_none()>{"None"}</option>
                        <option selected=matches!(properties.collectable_type, Some(CollectableType::Item))>{"Item"}</option>
                        <option selected=matches!(properties.collectable_type, Some(CollectableType::MeleeWeapon))>{"Melee Weapon"}</option>
                        <option selected=matches!(properties.collectable_type, Some(CollectableType::RangedWeapon))>{"Ranged Weapon"}</option>
                        <option selected=matches!(properties.collectable_type, Some(CollectableType::Armor))>{"Armor"}</option>
                    </select>
                </label>
                
                <label>
                    {"Stat ID: "}
                    <input type="text" value=properties.stat_id.unwrap_or_default() placeholder="(optional reusable stat)" />
                </label>
            </div>
        </details>
    }
}

#[component]
fn PlayerPropertiesPanel(
    properties: PlayerProperties,
    component_id: String,
) -> impl IntoView {
    let (is_open, set_is_open) = signal(false);
    
    view! {
        <details open=is_open.get() on:toggle=move |_| set_is_open.update(|v| *v = !*v)>
            <summary>{"Player Character Properties"}</summary>
            <div class="property-group">
                <label>
                    {"Model ID: "}
                    <input type="text" value=properties.model_id.unwrap_or_default() />
                </label>
                
                <label>
                    {"Default Weapon ID: "}
                    <input type="text" value=properties.default_weapon_id.unwrap_or_default() placeholder="(component id of weapon collectable)" />
                </label>
                
                <p class="info-text">{"Default weapon will be mounted on LowerArm.r"}</p>
            </div>
        </details>
    }
}

#[component]
fn ScatterPropertiesPanel(
    settings: ScatterSettings,
    component_id: String,
) -> impl IntoView {
    let (is_open, set_is_open) = signal(false);
    
    view! {
        <details open=is_open.get() on:toggle=move |_| set_is_open.update(|v| *v = !*v)>
            <summary>{"Scatter Settings"}</summary>
            <div class="property-group">
                <label>
                    {"Density: "}
                    <input type="number" step="0.1" min="0" value=settings.density />
                </label>
                
                <label>
                    {"Radius: "}
                    <input type="number" step="0.5" min="0" value=settings.radius />
                </label>
                
                <label>
                    {"Seed: "}
                    <input type="number" step="1" min="0" value=settings.seed />
                </label>
            </div>
        </details>
    }
}