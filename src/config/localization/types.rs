use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageFile {
    pub language_name: String,
    pub translations: HashMap<String, String>,
}

pub static CURRENT_TRANSLATIONS: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();
pub static CURRENT_LANGUAGE_CODE: OnceLock<RwLock<String>> = OnceLock::new();
