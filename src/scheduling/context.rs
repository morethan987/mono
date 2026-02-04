//! App type classification based on application IDs
//!
//! Maps compositor app_ids to task types for context awareness

use crate::models::TaskType;
use std::collections::HashMap;

/// Classifier that maps application IDs to task types
pub struct AppClassifier {
    /// Mapping of app_id (or prefix) to task type tags
    mapping: HashMap<String, Vec<String>>,
}

impl AppClassifier {
    /// Create a new classifier with default mappings
    pub fn new() -> Self {
        let mut mapping = HashMap::new();

        // Browsers
        mapping.insert("firefox".to_string(), vec!["study".to_string()]);
        mapping.insert("chromium".to_string(), vec!["study".to_string()]);
        mapping.insert("librewolf".to_string(), vec!["study".to_string()]);
        mapping.insert("google-chrome".to_string(), vec!["study".to_string()]);

        // Terminals
        mapping.insert("wezterm".to_string(), vec!["work".to_string()]);
        mapping.insert("alacritty".to_string(), vec!["work".to_string()]);
        mapping.insert("foot".to_string(), vec!["work".to_string()]);
        mapping.insert("kitty".to_string(), vec!["work".to_string()]);

        // Editors
        mapping.insert("code".to_string(), vec!["work".to_string()]);
        mapping.insert("nvim".to_string(), vec!["work".to_string()]);
        mapping.insert("emacs".to_string(), vec!["work".to_string()]);

        // Social / Chat
        mapping.insert("discord".to_string(), vec!["social".to_string()]);
        mapping.insert("slack".to_string(), vec!["social".to_string()]);
        mapping.insert("telegram".to_string(), vec!["social".to_string()]);
        mapping.insert("weixin".to_string(), vec!["social".to_string()]);

        // Games / Media
        mapping.insert("steam".to_string(), vec!["rest".to_string()]);
        mapping.insert("mpv".to_string(), vec!["rest".to_string()]);
        mapping.insert("spotify".to_string(), vec!["rest".to_string()]);
        mapping.insert("vlc".to_string(), vec!["rest".to_string()]);

        Self { mapping }
    }

    /// Classify an app_id into a TaskType
    pub fn classify(&self, app_id: &str) -> TaskType {
        self.classify_with_title(app_id, "")
    }

    /// Classify an app_id with window title refinement
    pub fn classify_with_title(&self, app_id: &str, title: &str) -> TaskType {
        let app_id_lower = app_id.to_lowercase();
        let title_lower = title.to_lowercase();

        // Browser refinement
        if app_id_lower.contains("firefox")
            || app_id_lower.contains("chromium")
            || app_id_lower.contains("chrome")
            || app_id_lower.contains("librewolf")
        {
            if title_lower.contains("github")
                || title_lower.contains("stackoverflow")
                || title_lower.contains("rust")
                || title_lower.contains("docs")
            {
                return TaskType::from_tags(&["work".to_string()]);
            }
            if title_lower.contains("youtube")
                || title_lower.contains("netflix")
                || title_lower.contains("twitch")
            {
                return TaskType::from_tags(&["rest".to_string()]);
            }
            if title_lower.contains("reddit")
                || title_lower.contains("twitter")
                || title_lower.contains("facebook")
            {
                return TaskType::from_tags(&["social".to_string()]);
            }
        }

        // Editor refinement
        if app_id_lower.contains("code")
            || app_id_lower.contains("nvim")
            || app_id_lower.contains("emacs")
        {
            if title_lower.contains("mono") {
                return TaskType::from_tags(&["work".to_string(), "project:mono".to_string()]);
            }
        }

        // Try exact match first
        if let Some(tags) = self.mapping.get(&app_id_lower) {
            return TaskType::from_tags(tags);
        }

        // Try prefix match (e.g. org.wezfurlong.wezterm)
        for (key, tags) in &self.mapping {
            if app_id_lower.contains(key) {
                return TaskType::from_tags(tags);
            }
        }

        // Default to uncategorized
        TaskType::from_tags(&["uncategorized".to_string()])
    }
}

impl Default for AppClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classification() {
        let classifier = AppClassifier::new();

        assert_eq!(classifier.classify("firefox").to_string(), "study");
        assert_eq!(
            classifier.classify("org.wezfurlong.wezterm").to_string(),
            "work"
        );
        assert_eq!(classifier.classify("Code").to_string(), "work");
        assert_eq!(classifier.classify("discord").to_string(), "social");
        assert_eq!(
            classifier.classify("unknown-app").to_string(),
            "uncategorized"
        );
    }

    #[test]
    fn test_title_refinement() {
        let classifier = AppClassifier::new();

        // Browsers
        assert_eq!(
            classifier
                .classify_with_title("firefox", "GitHub - mono")
                .to_string(),
            "work"
        );
        assert_eq!(
            classifier
                .classify_with_title("firefox", "YouTube Music")
                .to_string(),
            "rest"
        );
        assert_eq!(
            classifier
                .classify_with_title("chromium", "Reddit")
                .to_string(),
            "social"
        );

        // Editors
        assert_eq!(
            classifier
                .classify_with_title("code", "mono/src/main.rs")
                .to_string(),
            "work"
        );
    }
}
