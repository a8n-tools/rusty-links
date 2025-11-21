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
    is_github_repo: Option<bool>,
    github_stars: Option<i32>,
    github_archived: Option<bool>,
) -> Element {
    let has_metadata = !categories.is_empty()
        || !tags.is_empty()
        || !languages.is_empty()
        || !licenses.is_empty()
        || is_github_repo.unwrap_or(false);

    if !has_metadata {
        return rsx! {};
    }

    // Format star count with comma separators
    let format_stars = |count: i32| -> String {
        if count >= 1000 {
            format!("{:.1}k", count as f64 / 1000.0)
        } else {
            count.to_string()
        }
    };

    rsx! {
        div { class: "link-metadata",
            // GitHub metadata row (if applicable)
            if is_github_repo.unwrap_or(false) {
                div { class: "metadata-row metadata-row-github",
                    span { class: "badge badge-github", "GitHub" }

                    if let Some(stars) = github_stars {
                        span { class: "badge badge-stars", "⭐ {format_stars(stars)}" }
                    }

                    if github_archived.unwrap_or(false) {
                        span { class: "badge badge-archived", "🗄️ Archived" }
                    }
                }
            }

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
