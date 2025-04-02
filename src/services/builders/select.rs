use serde::Serialize;
use serde_json::{to_value, Value};

pub struct SelectBuilder<T> {
    pub items: Vec<T>,
    pub ignore: Vec<String>,
    pub select_class: Option<String>,
    pub option_class: Option<String>,
}

impl<T: Serialize> SelectBuilder<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self {
            items,
            ignore: Vec::new(),
            select_class: None,
            option_class: None,
        }
    }

    pub fn with_ignore(mut self, fields: &str) -> Self {
        self.ignore = fields.split(|c: char| c.is_whitespace() || c == ',').filter(|s| !s.is_empty()).map(String::from).collect();
        self
    }

    pub fn with_select_class(mut self, class: &str) -> Self {
        self.select_class = Some(class.to_string());
        self
    }

    pub fn with_option_class(mut self, class: &str) -> Self {
        self.option_class = Some(class.to_string());
        self
    }

    pub fn build(&self) -> String {
        let mut html = String::new();
        if let Some(select_class) = &self.select_class {
            html.push_str(&format!("<select class='{}'>\n", select_class));
        } else {
            html.push_str("<select>\n");
        }

        for item in &self.items {
            let item_value = to_value(item).unwrap();
            let content = if let Value::Object(map) = item_value {
                let pairs: Vec<String> = map
                    .into_iter()
                    .filter_map(|(k, v)| {
                        if self.ignore.contains(&k) {
                            None
                        } else {
                            let value_str = match v {
                                Value::String(s) => s,
                                _ => v.to_string(),
                            };
                            Some(format!("{}: {}", k, value_str))
                        }
                    })
                    .collect();
                pairs.join(", ")
            } else {
                match item_value {
                    Value::String(s) => s,
                    _ => item_value.to_string(),
                }
            };

            if let Some(option_class) = &self.option_class {
                html.push_str(&format!("<option class='{}'>{}</option>\n", option_class, content));
            } else {
                html.push_str(&format!("<option>{}</option>\n", content));
            }
        }
        html.push_str("</select>");
        html
    }
}
