use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug)]
pub struct LocalDatabase<T> {
    path: String,
    data: Mutex<HashMap<String, T>>,
}

impl<T> LocalDatabase<T>
where
    T: Serialize + DeserializeOwned + Clone + Default,
{
    /// Creates a new `LocalDatabase` instance with the given path.
    #[must_use] pub fn new(path: &str) -> Self {
        let data = if Path::new(path).exists() {
            let content = fs::read_to_string(path).expect("Failed to read the file");
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Self {
            path: path.to_string(),
            data: Mutex::new(data),
        }
    }

    /// Adds or updates a key-value pair in the database.
    pub fn set(&self, key: &str, value: T) {
        let mut data = self.data.lock().unwrap();
        let _old = data.insert(key.to_string(), value);
        self.save();
    }

    /// Retrieves a value associated with the given key.
    pub fn get(&self, key: &str) -> Option<T> {
        let data = self.data.lock().unwrap();
        data.get(key).cloned()
    }

    /// Saves the current state of the database to the file.
    fn save(&self) {
        let data = self.data.lock().unwrap();
        let json_string = serde_json::to_string(&*data).expect("Failed to serialize data to JSON");
        fs::write(&self.path, json_string).expect("Failed to write to the file");
    }
}
