use serde_json::Value;

pub struct Ui {
    data: Value,
}

impl Ui {
    pub fn new(lang: &str) -> Self {
        let json = match lang {
            "en" => include_str!("locales/en.json"),
            _ => include_str!("locales/pt-br.json"),
        };
        Self {
            data: serde_json::from_str(json).expect("invalid locale JSON"),
        }
    }

    /// Returns a static string value for the given key.
    /// Falls back to the key itself if not found.
    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.data[key].as_str().unwrap_or(key)
    }

    /// Returns a formatted string, replacing `{placeholder}` tokens in the translation.
    pub fn tf(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut s = self.data[key].as_str().unwrap_or(key).to_string();
        for (k, v) in args {
            s = s.replace(&format!("{{{k}}}"), v);
        }
        s
    }

    pub fn tab_names(&self) -> [&str; 4] {
        [
            self.t("tab.activities"),
            self.t("tab.summary"),
            self.t("tab.projects"),
            self.t("tab.config"),
        ]
    }
}
