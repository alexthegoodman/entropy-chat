use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;

// use crate::test::App;
// pub mod test;
use crate::app::App;
pub mod app;
pub mod components;

fn main() {
    console_error_panic_hook::set_once();

    // Find the container element by its ID
    let container = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("leptos-container") // Use the ID from index.html
        .expect("Did not find the element with ID 'leptos-container'")
        .unchecked_into::<web_sys::HtmlElement>();

    leptos::mount::mount_to(container, App).forget();
}

// use leptos::prelude::*;
// use wasm_bindgen::prelude::wasm_bindgen;
// use web_sys::wasm_bindgen::JsCast;

// // Define your main App component
// #[component]
// fn App() -> impl IntoView {
//     view! {
//         <h1>"Hello from Leptos in a Container!"</h1>
//     }
// }

// // This is the client-side entry point that `trunk` or `cargo-leptos` calls
// // #[cfg(feature = "csr")]
// #[wasm_bindgen]
// pub fn main() {
//     console_error_panic_hook::set_once();

//     // Find the container element by its ID
//     let container = web_sys::window()
//         .unwrap()
//         .document()
//         .unwrap()
//         .get_element_by_id("leptos-container") // Use the ID from index.html
//         .expect("Did not find the element with ID 'leptos-container'")
//         .unchecked_into::<web_sys::HtmlElement>();

//     // Mount the App component to the specific container element
//     mount_to(container, || view! { <App/> });
// }

// // You would use `hydrate_to` for SSR hydration similarly.
