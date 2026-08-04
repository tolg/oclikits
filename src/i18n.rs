use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    En,
    Zh,
}

static CURRENT_LANG: OnceLock<Language> = OnceLock::new();

pub fn detect_language() -> Language {
    let vars = ["AID_LANG", "LC_ALL", "LC_MESSAGES", "LANG"];
    for var in &vars {
        if let Ok(val) = std::env::var(var) {
            let lower = val.to_lowercase();
            if lower.contains("zh") {
                return Language::Zh;
            } else if !lower.is_empty() && lower != "c" && lower != "posix" {
                return Language::En;
            }
        }
    }
    Language::En
}

pub fn current_lang() -> Language {
    *CURRENT_LANG.get_or_init(detect_language)
}

pub fn is_zh() -> bool {
    current_lang() == Language::Zh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection_rules() {
        let check = |val: &str| -> Language {
            let lower = val.to_lowercase();
            if lower.contains("zh") {
                Language::Zh
            } else {
                Language::En
            }
        };

        assert_eq!(check("zh_CN.UTF-8"), Language::Zh);
        assert_eq!(check("zh_TW.UTF-8"), Language::Zh);
        assert_eq!(check("zh_HK.UTF-8"), Language::Zh);
        assert_eq!(check("zh_SG.UTF-8"), Language::Zh);
        assert_eq!(check("ZH_CN"), Language::Zh);
        assert_eq!(check("zh"), Language::Zh);
        assert_eq!(check("en_US.UTF-8"), Language::En);
        assert_eq!(check("C"), Language::En);
        assert_eq!(check("POSIX"), Language::En);
        assert_eq!(check("ja_JP.UTF-8"), Language::En);
    }
}
