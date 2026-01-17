use entropy_engine::core::pipeline::ExportPipeline;
use entropy_engine::helpers::saved_data::{File, LandscapeData, PBRTextureData, StatData};
use leptos::{prelude::*};
use std::rc::Rc;
use std::cell::RefCell;
use uuid::Uuid;

#[derive(Clone, PartialEq)]
enum AssetCategory {
    Models,
    Textures,
    PBRTextures,
    Landscapes,
    Stats,
}

#[component]
pub fn AssetsBrowser(
    pipeline_store: LocalResource<Option<Rc<RefCell<ExportPipeline>>>>,
    is_initialized: ReadSignal<bool>,
) -> impl IntoView {
    let (active_category, set_active_category) = signal(AssetCategory::Models);
    
    // Lists of assets
    let (models_list, set_models_list) = signal::<Vec<File>>(Vec::new());
    let (textures_list, set_textures_list) = signal::<Vec<File>>(Vec::new());
    let (pbr_list, set_pbr_list) = signal::<Vec<PBRTextureData>>(Vec::new());
    let (landscapes_list, set_landscapes_list) = signal::<Vec<LandscapeData>>(Vec::new());
    let (stats_list, set_stats_list) = signal::<Vec<StatData>>(Vec::new());

    // Sync from Pipeline/SavedState
    create_effect(move |_| {
        if is_initialized.get() {
            if let Some(pipeline) = pipeline_store.get() {
                if let Some(pipeline_arc) = pipeline.as_ref() {
                    let pipeline_guard = pipeline_arc.borrow();
                    if let Some(editor) = pipeline_guard.export_editor.as_ref() {
                        if let Some(saved_state) = editor.saved_state.as_ref() {
                            set_models_list.set(saved_state.models.clone());
                            set_textures_list.set(saved_state.textures.clone().unwrap_or_default());
                            set_pbr_list.set(saved_state.pbr_textures.clone().unwrap_or_default());
                            set_landscapes_list.set(saved_state.landscapes.clone().unwrap_or_default());
                            set_stats_list.set(saved_state.stats.clone().unwrap_or_default());
                        }
                    }
                }
            }
        }
    });

    view! {
        <div class="assets-browser">
            <div class="assets-tabs">
                <button 
                    class:active=move || active_category.get() == AssetCategory::Models
                    on:click=move |_| set_active_category.set(AssetCategory::Models)
                >
                    {"Models"}
                </button>
                <button 
                    class:active=move || active_category.get() == AssetCategory::Textures
                    on:click=move |_| set_active_category.set(AssetCategory::Textures)
                >
                    {"Textures"}
                </button>
                <button 
                    class:active=move || active_category.get() == AssetCategory::PBRTextures
                    on:click=move |_| set_active_category.set(AssetCategory::PBRTextures)
                >
                    {"PBR Textures"}
                </button>
                <button 
                    class:active=move || active_category.get() == AssetCategory::Landscapes
                    on:click=move |_| set_active_category.set(AssetCategory::Landscapes)
                >
                    {"Landscapes"}
                </button>
                <button 
                    class:active=move || active_category.get() == AssetCategory::Stats
                    on:click=move |_| set_active_category.set(AssetCategory::Stats)
                >
                    {"Stats"}
                </button>
            </div>

            <div class="assets-content">
                {move || match active_category.get() {
                    AssetCategory::Models => view! {
                        <ModelsPanel list=models_list />
                    }.into_view().into_any(),
                    AssetCategory::Textures => view! {
                        <TexturesPanel list=textures_list />
                    }.into_view().into_any(),
                    AssetCategory::PBRTextures => view! {
                        <PBRTexturesPanel list=pbr_list />
                    }.into_view().into_any(),
                    AssetCategory::Landscapes => view! {
                        <LandscapesPanel list=landscapes_list />
                    }.into_view().into_any(),
                    AssetCategory::Stats => view! {
                        <StatsPanel list=stats_list />
                    }.into_view().into_any(),
                }}
            </div>
        </div>
    }
}

#[component]
fn ModelsPanel(list: ReadSignal<Vec<File>>) -> impl IntoView {
    // Local state for the add form
    let (new_name, set_new_name) = signal(String::new());
    let (new_path, set_new_path) = signal(String::new());

    view! {
        <div class="asset-panel">
            <div class="asset-list">
                <For
                    each=move || list.get()
                    key=|item| item.id.clone()
                    children=move |item| {
                        view! {
                            <div class="asset-item">
                                <span class="asset-name">{item.fileName}</span>
                                <span class="asset-id">{item.id}</span>
                            </div>
                        }
                    }
                />
            </div>
            
            <div class="add-asset-form">
                <h4>{"Add Model"}</h4>
                <div class="form-group">
                    <label>{"Name/File:"}</label>
                    <input type="text" 
                        on:input=move |ev| set_new_name.set(event_target_value(&ev))
                        value=new_name.get()
                    />
                </div>
                <div class="form-group">
                    <label>{"Path/URL:"}</label>
                    <input type="text" 
                        on:input=move |ev| set_new_path.set(event_target_value(&ev))
                        value=new_path.get()
                    />
                </div>
                <button class="add-btn">{"Add"}</button>
            </div>
        </div>
    }
}

#[component]
fn TexturesPanel(list: ReadSignal<Vec<File>>) -> impl IntoView {
    let (new_name, set_new_name) = signal(String::new());
    let (new_path, set_new_path) = signal(String::new());

    view! {
        <div class="asset-panel">
            <div class="asset-list">
                <For
                    each=move || list.get()
                    key=|item| item.id.clone()
                    children=move |item| {
                        view! {
                            <div class="asset-item">
                                <span class="asset-name">{item.fileName}</span>
                                <span class="asset-id">{item.id}</span>
                            </div>
                        }
                    }
                />
            </div>
            
             <div class="add-asset-form">
                <h4>{"Add Texture"}</h4>
                <div class="form-group">
                    <label>{"Name/File:"}</label>
                    <input type="text" 
                        on:input=move |ev| set_new_name.set(event_target_value(&ev))
                        value=new_name.get()
                    />
                </div>
                <div class="form-group">
                    <label>{"Path/URL:"}</label>
                    <input type="text" 
                        on:input=move |ev| set_new_path.set(event_target_value(&ev))
                        value=new_path.get()
                    />
                </div>
                <button class="add-btn">{"Add"}</button>
            </div>
        </div>
    }
}

#[component]
fn PBRTexturesPanel(list: ReadSignal<Vec<PBRTextureData>>) -> impl IntoView {
    view! {
        <div class="asset-panel">
            <div class="asset-list">
                <For
                    each=move || list.get()
                    key=|item| item.id.clone()
                    children=move |item| {
                        view! {
                            <div class="asset-item">
                                <span class="asset-name">{item.id}</span>
                                // Displaying more info might be needed
                            </div>
                        }
                    }
                />
            </div>
            
            <div class="add-asset-form">
                <h4>{"Add PBR Texture Set"}</h4>
                // Needs more inputs for different maps
                <button class="add-btn">{"Add"}</button>
            </div>
        </div>
    }
}

#[component]
fn LandscapesPanel(list: ReadSignal<Vec<LandscapeData>>) -> impl IntoView {
    view! {
        <div class="asset-panel">
            <div class="asset-list">
                <For
                    each=move || list.get()
                    key=|item| item.id.clone()
                    children=move |item| {
                        view! {
                            <div class="asset-item">
                                <span class="asset-name">{item.id}</span>
                                <span class="asset-detail">
                                    {item.heightmap.map(|f| f.fileName).unwrap_or_else(|| "No Heightmap".to_string())}
                                </span>
                            </div>
                        }
                    }
                />
            </div>
            
            <div class="add-asset-form">
                <h4>{"Add Landscape"}</h4>
                 // Needs inputs for heightmap file etc
                <button class="add-btn">{"Add"}</button>
            </div>
        </div>
    }
}

#[component]
fn StatsPanel(list: ReadSignal<Vec<StatData>>) -> impl IntoView {
    view! {
        <div class="asset-panel">
            <div class="asset-list">
                <For
                    each=move || list.get()
                    key=|item| item.id.clone()
                    children=move |item| {
                        view! {
                            <div class="asset-item">
                                <span class="asset-name">{item.name}</span>
                                <span class="asset-id">{item.id}</span>
                            </div>
                        }
                    }
                />
            </div>
             <div class="add-asset-form">
                <h4>{"Add Stat"}</h4>
                // Needs inputs for stat values
                <button class="add-btn">{"Add"}</button>
            </div>
        </div>
    }
}