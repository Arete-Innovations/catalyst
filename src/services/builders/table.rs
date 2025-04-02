use serde::Serialize;
use serde_json::{to_value, Value};

pub struct TableBuilder<T> {
    pub items: Vec<T>,
    pub ignore: Vec<String>,
    pub table_class: Option<String>,
    pub thead_class: Option<String>,
    pub tr_class: Option<String>,
    pub td_class: Option<String>,
}

impl<T: Serialize> TableBuilder<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self {
            items,
            ignore: Vec::new(),
            table_class: None,
            thead_class: None,
            tr_class: None,
            td_class: None,
        }
    }

    pub fn with_ignore(mut self, fields: &str) -> Self {
        self.ignore = fields.split(|c: char| c.is_whitespace() || c == ',').filter(|s| !s.is_empty()).map(String::from).collect();
        self
    }

    pub fn with_table_class(mut self, class: &str) -> Self {
        self.table_class = Some(class.to_string());
        self
    }

    pub fn with_thead_class(mut self, class: &str) -> Self {
        self.thead_class = Some(class.to_string());
        self
    }

    pub fn with_tr_class(mut self, class: &str) -> Self {
        self.tr_class = Some(class.to_string());
        self
    }

    pub fn with_td_class(mut self, class: &str) -> Self {
        self.td_class = Some(class.to_string());
        self
    }

    pub fn build(&self) -> String {
        if self.items.is_empty() {
            return "<table></table>".to_string();
        }

        let first_item = to_value(&self.items[0]).unwrap();
        let columns: Vec<String> = if let Value::Object(map) = first_item {
            map.into_iter().filter_map(|(k, _)| if self.ignore.contains(&k) { None } else { Some(k) }).collect()
        } else {
            vec![]
        };

        let table_tag = if let Some(class) = &self.table_class { format!("<table class='{}'>", class) } else { "<table>".to_string() };

        let thead_tag = if let Some(class) = &self.thead_class { format!("<thead class='{}'>", class) } else { "<thead>".to_string() };

        let mut html = String::new();
        html.push_str(&format!("{}\n{}", table_tag, thead_tag));

        html.push_str("<tr");
        if let Some(class) = &self.tr_class {
            html.push_str(&format!(" class='{}'", class));
        }
        html.push_str(">");
        for col in &columns {
            html.push_str(&format!("<th>{}</th>", col));
        }
        html.push_str("</tr></thead>\n<tbody>\n");

        for item in &self.items {
            let item_value = to_value(item).unwrap();
            if let Value::Object(map) = item_value {
                html.push_str("<tr");
                if let Some(class) = &self.tr_class {
                    html.push_str(&format!(" class='{}'", class));
                }
                html.push_str(">");
                for col in &columns {
                    html.push_str("<td");
                    if let Some(class) = &self.td_class {
                        html.push_str(&format!(" class='{}'", class));
                    }
                    html.push_str(">");
                    if let Some(val) = map.get(col) {
                        let cell = match val {
                            Value::String(s) => s.clone(),
                            _ => val.to_string(),
                        };
                        html.push_str(&cell);
                    }
                    html.push_str("</td>");
                }
                html.push_str("</tr>\n");
            }
        }
        html.push_str("</tbody>\n</table>");
        html
    }
}
