use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A local database for storing key-value pairs.
///
/// The database is stored in a JSON file, which is updated every time a key-value pair is added or updated.
///
/// # Example
///
/// ```no_run
/// use gadget_sdk::store::LocalDatabase;
///
/// let db = LocalDatabase::<u64>::open("data.json");
///
/// db.set("key", 42);
/// assert_eq!(db.get("key"), Some(42));
/// ```
#[derive(Debug)]
pub struct LocalDatabase<T> {
    path: PathBuf,
    data: Mutex<HashMap<String, T>>,
}

impl<T> LocalDatabase<T>
where
    T: Serialize + DeserializeOwned + Clone + Default,
{
    /// Reads a `LocalDatabase` from the given path.
    ///
    /// If the file does not exist, an empty database is created.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use gadget_sdk::store::LocalDatabase;
    ///
    /// let db = LocalDatabase::<u64>::open("data.json");
    /// assert!(db.is_empty());
    /// ```
    #[must_use]
    pub fn open<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        let parent_dir = path.parent().expect("Failed to get parent directory");

        // Create the parent directory if it doesn't exist
        fs::create_dir_all(parent_dir).expect("Failed to create parent directory");

        let data = if path.exists() {
            let content = fs::read_to_string(path).expect("Failed to read the file");
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Self {
            path: path.to_owned(),
            data: Mutex::new(data),
        }
    }

    /// Returns the number of key-value pairs in the database.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use gadget_sdk::store::LocalDatabase;
    ///
    /// let db = LocalDatabase::<u64>::open("data.json");
    /// assert_eq!(db.len(), 0);
    ///
    /// db.set("key", 42);
    /// assert_eq!(db.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        let data = self.data.lock().unwrap();
        data.len()
    }

    /// Checks if the database is empty.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use gadget_sdk::store::LocalDatabase;
    ///
    /// let db = LocalDatabase::<u64>::open("data.json");
    /// assert!(db.is_empty());
    ///
    /// db.set("key", 42);
    /// assert!(!db.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        let data = self.data.lock().unwrap();
        data.is_empty()
    }

    /// Adds or updates a key-value pair in the database.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use gadget_sdk::store::LocalDatabase;
    ///
    /// let db = LocalDatabase::<u64>::open("data.json");
    ///
    /// db.set("key", 42);
    /// assert_eq!(db.get("key"), Some(42));
    /// ```
    pub fn set(&self, key: &str, value: T) {
        let mut data = self.data.lock().unwrap();
        let _old = data.insert(key.to_string(), value);

        // Save the data while the lock is held
        let json_string = serde_json::to_string(&*data).expect("Failed to serialize data to JSON");
        fs::write(&self.path, json_string).expect("Failed to write to the file");
    }

    /// Retrieves a value associated with the given key.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use gadget_sdk::store::LocalDatabase;
    ///
    /// let db = LocalDatabase::<u64>::open("data.json");
    ///
    /// db.set("key", 42);
    /// assert_eq!(db.get("key"), Some(42));
    /// ```
    pub fn get(&self, key: &str) -> Option<T> {
        let data = self.data.lock().unwrap();
        data.get(key).cloned()
    }
}
