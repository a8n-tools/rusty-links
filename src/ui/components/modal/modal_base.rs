use dioxus::prelude::*;

#[component]
pub fn ModalBase(on_close: EventHandler<()>, children: Element) -> Element {
    rsx! {
        div {
            class: "modal-overlay",
            onclick: move |_| on_close.call(()),

            div {
                class: "modal-container",
                onclick: move |evt| evt.stop_propagation(),
                {children}
            }
        }
    }
}
