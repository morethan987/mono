use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TaskType {
    pub name: String,
}

impl TaskType {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    pub fn from_tags(tags: &[String]) -> Self {
        tags.first()
            .map(|t| TaskType { name: t.clone() })
            .unwrap_or_else(Self::default)
    }

    pub fn is_default(&self) -> bool {
        self.name == "default"
    }
}

impl Default for TaskType {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
        }
    }
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_tags() {
        let tags = vec!["work".to_string(), "urgent".to_string()];
        let task_type = TaskType::from_tags(&tags);
        assert_eq!(task_type.name, "work");

        let empty_tags: Vec<String> = vec![];
        let default_type = TaskType::from_tags(&empty_tags);
        assert!(default_type.is_default());
    }
}
