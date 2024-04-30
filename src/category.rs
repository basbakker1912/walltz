use anyhow::bail;

use crate::{
    config::CategoryConfig,
    finder::{check_string_equality, find_best_by_value},
    CONFIG,
};

#[derive(Debug, Clone)]
pub struct Category {
    pub name: String,
    pub tags: Vec<String>,
    pub aspect_ratios: Vec<String>,
}

impl Category {
    pub fn from_config(config: CategoryConfig) -> Self {
        Self {
            name: config.name,
            tags: config.tags,
            aspect_ratios: config
                .aspect_ratios
                .unwrap_or(CONFIG.aspect_ratios.to_owned()),
        }
    }

    pub fn find_in_config(name: &str) -> anyhow::Result<Self> {
        let (equal, best_value) = find_best_by_value(
            name,
            CONFIG.categories.iter(),
            |value| value.name.as_str(),
            |v1, v2| check_string_equality(v1, v2),
        );

        if let Some(value) = best_value {
            let category: Category = value.clone().into();
            if equal {
                Ok(category)
            } else {
                bail!(
                    "Didn't find any categories matching the name: '{}', did you mean: '{}'?",
                    name,
                    category.name
                );
            }
        } else {
            bail!("Didn't find any categories matching the name '{}'", name);
        }
    }
}

impl From<CategoryConfig> for Category {
    fn from(value: CategoryConfig) -> Self {
        Category::from_config(value)
    }
}
