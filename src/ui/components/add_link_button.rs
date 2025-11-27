use dioxus::prelude::*;
use uuid::Uuid;
use crate::ui::components::modal::{AddLinkDialog, LinkDetailsModal};
use crate::ui::components::table::links_table::Link;
use crate::ui::utils::{read_clipboard, is_valid_url};

#[component]
pub fn AddLinkButton(
    on_add: EventHandler<()>,
) -> Element {
    let mut show_dialog = use_signal(|| false);
    let mut clipboard_url = use_signal(|| Option::<String>::None);
    let mut show_duplicate_modal = use_signal(|| false);
    let mut duplicate_link_id = use_signal(|| Option::<Uuid>::None);
    let mut show_new_link_modal = use_signal(|| false);
    let mut new_link_id = use_signal(|| Option::<Uuid>::None);

    // Check clipboard on button click
    let check_clipboard_and_open = move |_| {
        spawn(async move {
            // Attempt to read clipboard
            match read_clipboard().await {
                Ok(text) if is_valid_url(&text) => {
                    clipboard_url.set(Some(text));
                },
                _ => {
                    clipboard_url.set(None);
                }
            }
            show_dialog.set(true);
        });
    };

    // Handle duplicate found
    let handle_duplicate = move |link: Link| {
        show_dialog.set(false);
        duplicate_link_id.set(Some(link.id));
        show_duplicate_modal.set(true);
    };

    // Handle link created successfully
    let handle_success = move |link: Link| {
        show_dialog.set(false);
        new_link_id.set(Some(link.id));
        show_new_link_modal.set(true);
        on_add.call(());
    };

    rsx! {
        button {
            class: "btn-primary btn-add-link",
            onclick: check_clipboard_and_open,
            // Plus icon SVG
            svg {
                class: "icon",
                xmlns: "http://www.w3.org/2000/svg",
                width: "20",
                height: "20",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                line { x1: "12", y1: "5", x2: "12", y2: "19" }
                line { x1: "5", y1: "12", x2: "19", y2: "12" }
            }
            "Add Link"
        }

        // Add Link Dialog
        if show_dialog() {
            AddLinkDialog {
                initial_url: clipboard_url().unwrap_or_default(),
                on_close: move |_| show_dialog.set(false),
                on_success: handle_success,
                on_duplicate: handle_duplicate
            }
        }

        // Show duplicate link modal
        if show_duplicate_modal() {
            if let Some(link_id) = duplicate_link_id() {
                LinkDetailsModal {
                    link_id: link_id,
                    is_open: true,
                    on_close: move |_| {
                        show_duplicate_modal.set(false);
                        duplicate_link_id.set(None);
                    },
                    on_save: move |_| {
                        show_duplicate_modal.set(false);
                        duplicate_link_id.set(None);
                        on_add.call(());
                    }
                }
            }
        }

        // Show newly created link modal
        if show_new_link_modal() {
            if let Some(link_id) = new_link_id() {
                LinkDetailsModal {
                    link_id: link_id,
                    is_open: true,
                    on_close: move |_| {
                        show_new_link_modal.set(false);
                        new_link_id.set(None);
                    },
                    on_save: move |_| {
                        show_new_link_modal.set(false);
                        new_link_id.set(None);
                        on_add.call(());
                    }
                }
            }
        }
    }
}
