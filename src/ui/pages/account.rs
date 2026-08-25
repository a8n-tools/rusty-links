//! LINKS-43: the account page, where a user sees and changes the settings
//! their own account owns.
//!
//! LINKS-33 shipped the server half of the new-location alert opt-out
//! (`GET`/`PATCH /api/auth/me`) with no way to reach it from a browser, so the
//! only way to silence an alert was an API client. This page is that way.
//!
//! Two rules shape it:
//!
//! - **Every control renders the persisted value, never the optimistic one.**
//!   The toggle re-renders from the `UserInfo` the PATCH answers with, so a
//!   save that fails leaves the control showing what is actually stored rather
//!   than what the user clicked. A security setting that silently disagrees
//!   with the server is worse than no setting.
//! - **Each control sends only the field it owns.** `UpdateMeRequest` reads an
//!   absent key as "not submitted", so a one-field patch can never clobber a
//!   sibling setting as the page grows.

use dioxus::prelude::*;

use crate::server_functions::auth::{UpdateMeRequest, UserInfo};
use crate::ui::components::loading::{LoadingSpinner, SpinnerSize};
use crate::ui::components::navbar::Navbar;
use crate::ui::http;

#[component]
pub fn AccountPage() -> Element {
    // The stored preference. `None` until the first load resolves, which is
    // what keeps the checkbox from rendering a guess.
    let mut notify_new_location = use_signal(|| Option::<bool>::None);
    let mut loading = use_signal(|| true);
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let mut saved = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            loading.set(true);
            match http::get::<UserInfo>("/api/auth/me").await {
                Ok(info) => {
                    notify_new_location.set(Some(info.notify_new_location));
                    error.set(None);
                }
                Err(err) => error.set(Some(err)),
            }
            loading.set(false);
        });
    });

    // Persist one setting and re-render from the answer. On failure the signal
    // is left untouched, so the checkbox snaps back to the stored value.
    let toggle_alerts = move |desired: bool| {
        spawn(async move {
            saving.set(true);
            saved.set(false);
            error.set(None);

            let request = UpdateMeRequest {
                notify_new_location: Some(desired),
            };
            match http::patch::<UserInfo, _>("/api/auth/me", &request).await {
                Ok(info) => {
                    notify_new_location.set(Some(info.notify_new_location));
                    saved.set(true);
                }
                Err(err) => error.set(Some(err)),
            }
            saving.set(false);
        });
    };

    rsx! {
        div { class: "page-container",
            Navbar {}

            div { class: "content-container",
                div { class: "page-header",
                    h1 { "Account" }
                }

                if let Some(err) = error() {
                    div { class: "error-box", role: "alert", "⚠️ {err}" }
                }

                if loading() {
                    LoadingSpinner {
                        size: SpinnerSize::Medium,
                        message: "Loading your account settings...".to_string(),
                    }
                } else if let Some(enabled) = notify_new_location() {
                    section {
                        class: "account-section",
                        "aria-labelledby": "security-alerts-heading",
                        h2 { id: "security-alerts-heading", "Security alerts" }

                        label { class: "account-toggle",
                            input {
                                r#type: "checkbox",
                                checked: enabled,
                                disabled: saving(),
                                onchange: move |event| toggle_alerts(event.checked()),
                            }
                            span { class: "account-toggle-label",
                                "Email me when someone signs in to my account from a new country."
                            }
                        }

                        p { class: "account-hint",
                            "The alert names the country, time, IP address and browser of the sign-in. "
                            "Turning it off stops the email; it does not change who can sign in."
                        }

                        // Live region: the save is a background PATCH with no
                        // page transition, so a screen reader needs to be told
                        // it landed.
                        div { class: "account-status", role: "status", "aria-live": "polite",
                            if saving() {
                                "Saving..."
                            } else if saved() {
                                "Saved."
                            }
                        }
                    }
                }
            }
        }
    }
}
