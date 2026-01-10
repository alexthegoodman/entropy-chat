use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;

fn main() {
    // Find the container element by its ID
    let container = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("leptos-container") // Use the ID from index.html
        .expect("Did not find the element with ID 'leptos-container'")
        .unchecked_into::<web_sys::HtmlElement>();

    let _ = leptos::mount::mount_to(container, || view! { <p>"Hello, Entropy Chat!"</p> });
}