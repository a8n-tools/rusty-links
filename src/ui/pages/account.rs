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

use crate::server_functions::auth::{KnownDeviceInfo, UpdateMeRequest, UserInfo};
use crate::ui::components::loading::{LoadingSpinner, SpinnerSize};
use crate::ui::components::modal::ConfirmDialog;
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

                    DeviceSection {}
                }
            }
        }
    }
}

/// LINKS-55: the devices this account is recognised from, with a revoke per row.
///
/// What the user needs to understand before clicking, and so what the copy has
/// to say: revoking never signs anyone out and never blocks a sign-in. It only
/// stops the device satisfying the LINKS-45 trigger, so the next sign-in from
/// it is held for email approval. Revoking the last one is not a lockout: it
/// returns the account to the zero-devices baseline every account shipped at,
/// where the device trigger holds nobody.
#[component]
fn DeviceSection() -> Element {
    let mut devices = use_signal(Vec::<KnownDeviceInfo>::new);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| Option::<String>::None);
    let mut confirming = use_signal(|| Option::<KnownDeviceInfo>::None);
    let mut revoking = use_signal(|| false);

    let load = move || {
        spawn(async move {
            loading.set(true);
            match http::get_with_device_id::<Vec<KnownDeviceInfo>>("/api/auth/devices").await {
                Ok(list) => {
                    devices.set(list);
                    error.set(None);
                }
                Err(err) => error.set(Some(err)),
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        load();
    });

    // Re-read the list from the server after a revoke rather than splicing the
    // row out locally, so what is shown is what is stored.
    let confirm_revoke = move |_| {
        let Some(device) = confirming() else {
            return;
        };
        spawn(async move {
            revoking.set(true);
            error.set(None);
            match http::delete(&format!("/api/auth/devices/{}", device.id)).await {
                Ok(()) => {
                    confirming.set(None);
                    load();
                }
                Err(err) => {
                    error.set(Some(err));
                    confirming.set(None);
                }
            }
            revoking.set(false);
        });
    };

    let count = devices().len();

    rsx! {
        section {
            class: "account-section",
            "aria-labelledby": "devices-heading",
            h2 { id: "devices-heading", "Recognised devices" }

            p { class: "account-hint",
                "Browsers your account has completed a sign-in from. A sign-in from anything else "
                "is held until you approve it by email. Revoking a device does not sign it out; it "
                "means the next sign-in from it is held."
            }

            if let Some(err) = error() {
                div { class: "error-box", role: "alert", "⚠️ {err}" }
            }

            if loading() {
                LoadingSpinner {
                    size: SpinnerSize::Small,
                    message: "Loading your devices...".to_string(),
                }
            } else if count == 0 {
                div { class: "empty-state",
                    div { class: "empty-icon", "💻" }
                    div { class: "empty-title", "No recognised devices" }
                    div { class: "empty-description",
                        "This is the starting state for every account. No sign-in is held on the "
                        "device check until you next sign in, which records the browser you use."
                    }
                }
            } else {
                ul { class: "device-list",
                    for device in devices() {
                        li { key: "{device.id}", class: "device-row",
                            div { class: "device-facts",
                                div { class: "device-title",
                                    if device.is_current {
                                        span { class: "device-badge", "This browser" }
                                    }
                                    span { "Device {short_id(&device)}" }
                                }
                                div { class: "device-dates",
                                    "First used {format_stamp(&device.first_seen_at)} · "
                                    "last used {format_stamp(&device.last_seen_at)}"
                                }
                            }
                            button {
                                class: "btn-danger",
                                disabled: revoking(),
                                onclick: {
                                    let device = device.clone();
                                    move |_| confirming.set(Some(device.clone()))
                                },
                                "aria-label": "Revoke device {short_id(&device)}",
                                "Revoke"
                            }
                        }
                    }
                }
            }
        }

        if let Some(device) = confirming() {
            ConfirmDialog {
                title: "Revoke this device?".to_string(),
                message: revoke_warning(&device, count),
                confirm_text: "Revoke".to_string(),
                cancel_text: "Cancel".to_string(),
                dangerous: true,
                on_confirm: confirm_revoke,
                on_cancel: move |_| confirming.set(None),
            }
        }
    }
}

/// A short, stable handle for a row. The device id itself never reaches the
/// client, so this is a slice of the opaque row id and identifies nothing off
/// this page.
fn short_id(device: &KnownDeviceInfo) -> String {
    device.id.simple().to_string().chars().take(8).collect()
}

fn format_stamp(stamp: &chrono::DateTime<chrono::Utc>) -> String {
    stamp.format("%Y-%m-%d %H:%M UTC").to_string()
}

/// What revoking this row actually costs, said plainly before it happens.
///
/// The last-device case is called out because it is the one that sounds
/// alarming and is not: it returns the account to the baseline it started at,
/// where the device check holds nobody. Saying so is what stops a user from
/// believing they have locked themselves out.
fn revoke_warning(device: &KnownDeviceInfo, total: usize) -> String {
    if total <= 1 {
        return "This is the only device your account is recognised from. Revoking it returns \
                your account to its starting state, where no sign-in is held on the device \
                check. Your next sign-in records the browser you use. You will not be locked \
                out and you will not be signed out."
            .to_string();
    }
    if device.is_current {
        return "This is the browser you are using now. Revoking it does not sign you out, but \
                your next sign-in from it is held until you approve it by email."
            .to_string();
    }
    "The next sign-in from this device is held until you approve it by email. You will not be \
     signed out of it now."
        .to_string()
}
