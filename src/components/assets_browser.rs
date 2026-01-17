use entropy_engine::core::pipeline::ExportPipeline;
use entropy_engine::helpers::saved_data::{File, LandscapeData, PBRTextureData, StatData, SavedState};
use leptos::{html, prelude::*};
use std::rc::Rc;
use std::cell::RefCell;
use uuid::Uuid;
use web_sys::{FormData, HtmlInputElement};
use wasm_bindgen::JsCast;
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::task::spawn_local;

#[derive(Clone, PartialEq)]
enum AssetCategory {
    Models,
    Textures,
    PBRTextures,
    Landscapes,
    Stats,
}

fn get_api_url() -> String {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location.hostname().unwrap_or_default();

    if hostname == "localhost" || hostname == "127.0.0.1" {
        "http://localhost:3000".to_string()
    } else {
        "https://entropy-site.vercel.app".to_string()
    }
}

async fn save_project_state(project_id: &str, saved_state: &SavedState) -> Result<(), String> {
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

#[component]
pub fn AssetsBrowser(
    pipeline_store: LocalResource<Option<Rc<RefCell<ExportPipeline>>>>,
    is_initialized: ReadSignal<bool>,
    project_path: Signal<Option<String>>,
    project_id: Signal<Option<String>>,
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
    
    let update_saved_state = move |action: Box<dyn FnOnce(&mut SavedState)>| {
        if let Some(pipeline) = pipeline_store.get_untracked() {
             if let Some(pipeline_arc) = pipeline.as_ref() {
                let mut pipeline_guard = pipeline_arc.borrow_mut();
                if let Some(editor) = pipeline_guard.export_editor.as_mut() {
                    if let Some(saved_state) = editor.saved_state.as_mut() {
                        action(saved_state);
                        
                        // Update local signals
                        set_models_list.set(saved_state.models.clone());
                        set_textures_list.set(saved_state.textures.clone().unwrap_or_default());
                        set_pbr_list.set(saved_state.pbr_textures.clone().unwrap_or_default());
                        set_landscapes_list.set(saved_state.landscapes.clone().unwrap_or_default());
                        set_stats_list.set(saved_state.stats.clone().unwrap_or_default());
                        
                        // Save to backend
                        let pid = project_id.get_untracked().unwrap_or_default();
                        let state_clone = saved_state.clone();
                        if !pid.is_empty() {
                            spawn_local(async move {
                                let _ = save_project_state(&pid, &state_clone).await;
                            });
                        }
                    }
                }
             }
        }
    };

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
                        <ModelsPanel 
                            list=models_list 
                            project_path=project_path 
                            on_add=update_saved_state.clone() 
                        />
                    }.into_view().into_any(),
                    AssetCategory::Textures => view! {
                        <TexturesPanel 
                            list=textures_list 
                            project_path=project_path 
                            on_add=update_saved_state.clone() 
                        />
                    }.into_view().into_any(),
                    AssetCategory::PBRTextures => view! {
                        <PBRTexturesPanel 
                            list=pbr_list 
                            project_path=project_path 
                            on_add=update_saved_state.clone() 
                        />
                    }.into_view().into_any(),
                    AssetCategory::Landscapes => view! {
                        <LandscapesPanel 
                            list=landscapes_list 
                            project_path=project_path 
                            on_add=update_saved_state.clone() 
                        />
                    }.into_view().into_any(),
                    AssetCategory::Stats => view! {
                        <StatsPanel 
                            list=stats_list 
                            on_add=update_saved_state.clone() 
                        />
                    }.into_view().into_any(),
                }}
            </div>
        </div>
    }
}

#[component]
fn ModelsPanel<F>(
    list: ReadSignal<Vec<File>>,
    project_path: Signal<Option<String>>,
    on_add: F
) -> impl IntoView 
where F: Fn(Box<dyn FnOnce(&mut SavedState)>) + Clone + 'static
{
    let file_input_ref = NodeRef::<html::Input>::new();

    let on_upload = move |_| {
        let input = file_input_ref.get();
        if let Some(input) = input {
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    let project_path_val = project_path.get().unwrap_or_default();
                    if project_path_val.is_empty() {
                         log!("No project path available");
                         return;
                    }

                    let file_name = file.name();
                    let file_name_clone = file_name.clone();
                    
                    let form_data = FormData::new().unwrap();
                    form_data.append_with_str("projectPath", &project_path_val).unwrap();
                    form_data.append_with_str("filename", &file_name).unwrap();
                    form_data.append_with_blob("file", &file).unwrap();

                    let on_add = on_add.clone();
                    
                    spawn_local(async move {
                         let url = format!("{}/api/upload-model", get_api_url());
                         let res = Request::post(&url)
                            .body(form_data)
                            .unwrap()
                            .send()
                            .await;
                            
                         if res.is_ok() {
                             log!("Model uploaded successfully");
                             let new_file = File {
                                 id: Uuid::new_v4().to_string(),
                                 fileName: file_name_clone,
                                 cloudfrontUrl: "".to_string(), // Local only for now
                                 normalFilePath: "".to_string(),
                             };
                             
                             on_add(Box::new(move |state: &mut SavedState| {
                                 state.models.push(new_file);
                             }));
                         } else {
                             log!("Model upload failed");
                         }
                    });
                }
            }
        }
    };

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
                            </div>
                        }
                    }
                />
            </div>
            
            <div class="add-asset-form">
                <h4>{"Add Model"}</h4>
                <div class="form-group">
                    <label>{"Select File:"}</label>
                    <input type="file" node_ref=file_input_ref accept=".glb,.gltf" />
                </div>
                <button class="add-btn" on:click=on_upload>{"Add"}</button>
            </div>
        </div>
    }
}

#[component]
fn TexturesPanel<F>(
    list: ReadSignal<Vec<File>>,
    project_path: Signal<Option<String>>,
    on_add: F
) -> impl IntoView 
where F: Fn(Box<dyn FnOnce(&mut SavedState)>) + Clone + 'static
{
    let file_input_ref = NodeRef::<html::Input>::new();

    let on_upload = move |_| {
        let input = file_input_ref.get();
        if let Some(input) = input {
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    let project_path_val = project_path.get().unwrap_or_default();
                    if project_path_val.is_empty() { return; }

                    let file_name = file.name();
                    let file_name_clone = file_name.clone();
                    
                    let form_data = FormData::new().unwrap();
                    form_data.append_with_str("projectPath", &project_path_val).unwrap();
                    form_data.append_with_str("filename", &file_name).unwrap();
                    form_data.append_with_blob("file", &file).unwrap();

                    let on_add = on_add.clone();
                    
                    spawn_local(async move {
                         let url = format!("{}/api/upload-texture", get_api_url());
                         let res = Request::post(&url)
                            .body(form_data)
                            .unwrap()
                            .send()
                            .await;
                            
                         if res.is_ok() {
                             let new_file = File {
                                 id: Uuid::new_v4().to_string(),
                                 fileName: file_name_clone,
                                 cloudfrontUrl: "".to_string(),
                                 normalFilePath: "".to_string(),
                             };
                             
                             on_add(Box::new(move |state: &mut SavedState| {
                                 if let Some(textures) = state.textures.as_mut() {
                                     textures.push(new_file);
                                 } else {
                                     state.textures = Some(vec![new_file]);
                                 }
                             }));
                         }
                    });
                }
            }
        }
    };

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
                            </div>
                        }
                    }
                />
            </div>
            
             <div class="add-asset-form">
                <h4>{"Add Texture"}</h4>
                <div class="form-group">
                    <label>{"Select File:"}</label>
                    <input type="file" node_ref=file_input_ref accept=".png,.jpg,.jpeg" />
                </div>
                <button class="add-btn" on:click=on_upload>{"Add"}</button>
            </div>
        </div>
    }
}

#[component]
fn PBRTexturesPanel<F>(
    list: ReadSignal<Vec<PBRTextureData>>,
    project_path: Signal<Option<String>>,
    on_add: F
) -> impl IntoView 
where F: Fn(Box<dyn FnOnce(&mut SavedState)>) + Clone + 'static
{
    let diff_ref = NodeRef::<html::Input>::new();
    let norm_ref = NodeRef::<html::Input>::new();
    let rough_ref = NodeRef::<html::Input>::new();
    let metal_ref = NodeRef::<html::Input>::new();
    let ao_ref = NodeRef::<html::Input>::new();

    let on_upload = move |_| {
        let project_path_val = project_path.get().unwrap_or_default();
        if project_path_val.is_empty() { return; }

        let on_add = on_add.clone();
        let id = Uuid::new_v4().to_string();
        
        // Helper to upload one file
        let upload_file = move |input: Option<HtmlInputElement>| {
            let path_val = project_path_val.clone();
            async move {
                if let Some(input) = input {
                    if let Some(files) = input.files() {
                        if let Some(file) = files.get(0) {
                            let file_name = file.name();
                            let form_data = FormData::new().unwrap();
                            form_data.append_with_str("projectPath", &path_val).unwrap();
                            form_data.append_with_str("filename", &file_name).unwrap();
                            form_data.append_with_blob("file", &file).unwrap();
                            
                            let url = format!("{}/api/upload-texture", get_api_url());
                             if Request::post(&url).body(form_data).unwrap().send().await.is_ok() {
                                 return Some(File {
                                     id: Uuid::new_v4().to_string(),
                                     fileName: file_name,
                                     cloudfrontUrl: "".to_string(),
                                     normalFilePath: "".to_string(),
                                 });
                             }
                        }
                    }
                }
                None
            }
        };

        spawn_local(async move {
            let diff = upload_file(diff_ref.get()).await;
            let norm = upload_file(norm_ref.get()).await;
            let rough = upload_file(rough_ref.get()).await;
            let metal = upload_file(metal_ref.get()).await;
            let ao = upload_file(ao_ref.get()).await;
            
            if diff.is_some() {
                let pbr_data = PBRTextureData {
                    id: id,
                    diff,
                    nor_gl: norm,
                    rough,
                    metallic: metal,
                    ao,
                    ..Default::default()
                };
                
                 on_add(Box::new(move |state: &mut SavedState| {
                     if let Some(pbrs) = state.pbr_textures.as_mut() {
                         pbrs.push(pbr_data);
                     } else {
                         state.pbr_textures = Some(vec![pbr_data]);
                     }
                 }));
            }
        });
    };

    view! {
        <div class="asset-panel">
            <div class="asset-list">
                <For
                    each=move || list.get()
                    key=|item| item.id.clone()
                    children=move |item| {
                        view! {
                            <div class="asset-item">
                                <span class="asset-name">{"PBR Set"}</span>
                                <span class="asset-id">{item.id}</span>
                            </div>
                        }
                    }
                />
            </div>
            
            <div class="add-asset-form">
                <h4>{"Add PBR Texture Set"}</h4>
                <div class="form-group"><label>{"Diffuse:"}</label><input type="file" node_ref=diff_ref /></div>
                <div class="form-group"><label>{"Normal:"}</label><input type="file" node_ref=norm_ref /></div>
                <div class="form-group"><label>{"Roughness:"}</label><input type="file" node_ref=rough_ref /></div>
                <div class="form-group"><label>{"Metallic:"}</label><input type="file" node_ref=metal_ref /></div>
                <div class="form-group"><label>{"AO:"}</label><input type="file" node_ref=ao_ref /></div>
                <button class="add-btn" on:click=on_upload>{"Add"}</button>
            </div>
        </div>
    }
}

#[component]
fn LandscapesPanel<F>(
    list: ReadSignal<Vec<LandscapeData>>,
    project_path: Signal<Option<String>>,
    on_add: F
) -> impl IntoView 
where F: Fn(Box<dyn FnOnce(&mut SavedState)>) + Clone + 'static
{
    let height_ref = NodeRef::<html::Input>::new();
    let rock_ref = NodeRef::<html::Input>::new();
    let soil_ref = NodeRef::<html::Input>::new();

    let on_upload = move |_| {
        let project_path_val = project_path.get().unwrap_or_default();
        if project_path_val.is_empty() { return; }

        let on_add = on_add.clone();
        let landscape_id = Uuid::new_v4().to_string();
        let landscape_id_clone = landscape_id.clone();
        
        // Helper
        let upload_map = move |input: Option<HtmlInputElement>, type_str: &str| {
            let path_val = project_path_val.clone();
            let lid = landscape_id_clone.clone();
            let t_str = type_str.to_string();
            async move {
                if let Some(input) = input {
                    if let Some(files) = input.files() {
                        if let Some(file) = files.get(0) {
                            let file_name = file.name();
                            let form_data = FormData::new().unwrap();
                            form_data.append_with_str("projectPath", &path_val).unwrap();
                            form_data.append_with_str("landscapeAssetId", &lid).unwrap();
                            form_data.append_with_str("type", &t_str).unwrap();
                            form_data.append_with_str("filename", &file_name).unwrap();
                            form_data.append_with_blob("file", &file).unwrap();
                            
                            let url = format!("{}/api/upload-landscape-map", get_api_url());
                             if Request::post(&url).body(form_data).unwrap().send().await.is_ok() {
                                 return Some(File {
                                     id: Uuid::new_v4().to_string(),
                                     fileName: file_name,
                                     cloudfrontUrl: "".to_string(),
                                     normalFilePath: "".to_string(),
                                 });
                             }
                        }
                    }
                }
                None
            }
        };

        spawn_local(async move {
            let height = upload_map(height_ref.get(), "heightmap").await;
            let rock = upload_map(rock_ref.get(), "rockmap").await;
            let soil = upload_map(soil_ref.get(), "soil").await;
            
            if height.is_some() {
                let l_data = LandscapeData {
                    id: landscape_id,
                    heightmap: height,
                    rockmap: rock,
                    soil: soil,
                };
                
                 on_add(Box::new(move |state: &mut SavedState| {
                     if let Some(lands) = state.landscapes.as_mut() {
                         lands.push(l_data);
                     } else {
                         state.landscapes = Some(vec![l_data]);
                     }
                 }));
            }
        });
    };

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
                 <div class="form-group"><label>{"Heightmap:"}</label><input type="file" node_ref=height_ref /></div>
                 <div class="form-group"><label>{"Rockmap:"}</label><input type="file" node_ref=rock_ref /></div>
                 <div class="form-group"><label>{"Soil:"}</label><input type="file" node_ref=soil_ref /></div>
                <button class="add-btn" on:click=on_upload>{"Add"}</button>
            </div>
        </div>
    }
}

#[component]
fn StatsPanel<F>(
    list: ReadSignal<Vec<StatData>>,
    on_add: F
) -> impl IntoView 
where F: Fn(Box<dyn FnOnce(&mut SavedState)>) + Clone + 'static
{
    let (name, set_name) = signal(String::new());
    // Simplified stat creation
    let on_click = move |_| {
        let n = name.get();
        if n.is_empty() { return; }
        
        let new_stat = StatData {
            id: Uuid::new_v4().to_string(),
            name: n,
            character: None,
            attack: None,
            defense: None,
            weight: None,
        };
        
        let on_add = on_add.clone();
        on_add(Box::new(move |state: &mut SavedState| {
             if let Some(stats) = state.stats.as_mut() {
                 stats.push(new_stat);
             } else {
                 state.stats = Some(vec![new_stat]);
             }
        }));
        set_name.set("".to_string());
    };

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
                <div class="form-group">
                    <label>{"Name:"}</label>
                    <input type="text" value=name on:input=move |ev| set_name.set(event_target_value(&ev)) />
                </div>
                <button class="add-btn" on:click=on_click>{"Add"}</button>
            </div>
        </div>
    }
}
