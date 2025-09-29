use std::collections::HashMap;
use tera::{Context, Tera};

#[derive(Debug)]
pub struct Lang {
    pub lang: String,
    pub messages: HashMap<String, String>,
    pub tera: Tera,
}

impl Clone for Lang {
    fn clone(&self) -> Self {
        let mut tera = Tera::default();
        for (key, message) in &self.messages {
            tera.add_raw_template(key, message).unwrap();
        }
        Lang {
            lang: self.lang.clone(),
            messages: self.messages.clone(),
            tera,
        }
    }
}

impl Lang {
    pub fn from_json(lang: &str) -> Self {
        let path = format!("lang/{}.json", lang);
        let lang_data = std::fs::read_to_string(&path).unwrap_or_else(|_| {
            std::fs::read_to_string("lang/id.json").expect("Failed to read default language file")
        });
        let messages: HashMap<String, String> = serde_json::from_str(&lang_data).unwrap();

        let mut tera = Tera::default();
        for (key, message) in &messages {
            tera.add_raw_template(key, message).unwrap();
        }

        Lang {
            lang: lang.to_string(),
            messages,
            tera,
        }
    }

    pub fn get(&self, key: &str) -> String {
        self.messages
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    pub fn get_with_vars(&self, key: &str, vars: HashMap<String, String>) -> String {
        let mut context = Context::new();
        for (var, value) in vars {
            context.insert(&var, &value);
        }

        // Set default values for missing variables (the variable name itself)
        if let Some(message) = self.messages.get(key) {
            // Extract variable names from the message template
            let var_regex = regex::Regex::new(r"\{\{([^}]+)\}\}").unwrap();
            for cap in var_regex.captures_iter(message) {
                let var_name = &cap[1];
                if !context.contains_key(var_name) {
                    context.insert(var_name, var_name);
                }
            }
        }

        match self.tera.render(key, &context) {
            Ok(rendered) => rendered,
            Err(_) => self.get(key), // Fallback to the raw message if rendering fails
        }
    }
}
