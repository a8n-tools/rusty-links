use dioxus::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct CategoryInfo {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct TagInfo {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct LanguageInfo {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct LicenseInfo {
    pub id: Uuid,
    pub name: String,
}

#[component]
pub fn MetadataBadges(
    categories: Vec<CategoryInfo>,
    tags: Vec<TagInfo>,
    languages: Vec<LanguageInfo>,
    licenses: Vec<LicenseInfo>,
) -> Element {
    let has_metadata = !categories.is_empty()
        || !tags.is_empty()
        || !languages.is_empty()
        || !licenses.is_empty();

    if !has_metadata {
        return rsx! {};
    }

    rsx! {
        div { class: "link-metadata",
            if !categories.is_empty() {
                div { class: "metadata-row",
                    for cat in categories.iter() {
                        span { class: "badge badge-category", "{cat.name}" }
                    }
                }
            }

            if !tags.is_empty() {
                div { class: "metadata-row",
                    for tag in tags.iter() {
                        span { class: "badge badge-tag", "#{tag.name}" }
                    }
                }
            }

            if !languages.is_empty() || !licenses.is_empty() {
                div { class: "metadata-row metadata-row-tech",
                    for lang in languages.iter() {
                        span { class: "badge badge-language", "🦀 {lang.name}" }
                    }
                    for lic in licenses.iter() {
                        span { class: "badge badge-license", "📜 {lic.name}" }
                    }
                }
            }
        }
    }
}
