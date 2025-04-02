use serde::Serialize;
use serde_json::{to_value, Value};

pub enum ListType {
    Unordered,
    Ordered,
}

pub struct ListBuilder<T> {
    pub items: Vec<T>,
    pub ignore: Vec<String>,
    pub list_type: ListType,
    pub list_class: Option<String>,
    pub item_class: Option<String>,
}

impl<T: Serialize> ListBuilder<T> {
    pub fn new(list_type: ListType, items: Vec<T>) -> Self {
        Self {
            items,
            ignore: Vec::new(),
            list_type,
            list_class: None,
            item_class: None,
        }
    }

    pub fn with_ignore(mut self, fields: &str) -> Self {
        self.ignore = fields.split(|c: char| c.is_whitespace() || c == ',').filter(|s| !s.is_empty()).map(String::from).collect();
        self
    }

    pub fn with_list_class(mut self, class: &str) -> Self {
        self.list_class = Some(class.to_string());
        self
    }

    pub fn with_item_class(mut self, class: &str) -> Self {
        self.item_class = Some(class.to_string());
        self
    }

    pub fn build(&self) -> String {
        let tag = match self.list_type {
            ListType::Ordered => "ol",
            ListType::Unordered => "ul",
        };

        let mut html = String::new();
        if let Some(list_class) = &self.list_class {
            html.push_str(&format!("<{} class='{}'>\n", tag, list_class));
        } else {
            html.push_str(&format!("<{}>\n", tag));
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

            if let Some(item_class) = &self.item_class {
                html.push_str(&format!("<li class='{}'>{}</li>\n", item_class, content));
            } else {
                html.push_str(&format!("<li>{}</li>\n", content));
            }
        }
        html.push_str(&format!("</{}>", tag));
        html
    }
}
