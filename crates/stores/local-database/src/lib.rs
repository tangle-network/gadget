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
/// use gadget_store_local_database::LocalDatabase;
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
    /// use gadget_store_local_database::LocalDatabase;
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
            // Create an empty file with default empty JSON object
            let empty_data = HashMap::new();
            let json_string =
                serde_json::to_string(&empty_data).expect("Failed to serialize empty data to JSON");
            fs::write(path, json_string).expect("Failed to write empty file");
            empty_data
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
    /// use gadget_store_local_database::LocalDatabase;
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
    /// use gadget_store_local_database::LocalDatabase;
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
    /// use gadget_store_local_database::LocalDatabase;
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
    /// use gadget_store_local_database::LocalDatabase;
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::fs;
    use tempfile::tempdir;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
    struct TestStruct {
        field1: String,
        field2: i32,
    }

    #[test]
    fn test_create_new_database() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.json");

        let db = LocalDatabase::<u32>::open(&db_path);
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
        assert!(db_path.exists());
    }

    #[test]
    fn test_set_and_get() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.json");

        let db = LocalDatabase::<u32>::open(&db_path);
        db.set("key1", 42);
        db.set("key2", 100);

        assert_eq!(db.get("key1"), Some(42));
        assert_eq!(db.get("key2"), Some(100));
        assert_eq!(db.get("nonexistent"), None);
        assert_eq!(db.len(), 2);
    }

    #[test]
    fn test_complex_type() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.json");

        let db = LocalDatabase::<TestStruct>::open(&db_path);

        let test_struct = TestStruct {
            field1: "test".to_string(),
            field2: 42,
        };

        db.set("key1", test_struct.clone());
        assert_eq!(db.get("key1"), Some(test_struct));
    }

    #[test]
    fn test_persistence() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.json");

        // Write data
        {
            let db = LocalDatabase::<u32>::open(&db_path);
            db.set("key1", 42);
            db.set("key2", 100);
        }

        // Read data in new instance
        {
            let db = LocalDatabase::<u32>::open(&db_path);
            assert_eq!(db.get("key1"), Some(42));
            assert_eq!(db.get("key2"), Some(100));
            assert_eq!(db.len(), 2);
        }
    }

    #[test]
    fn test_overwrite() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.json");

        let db = LocalDatabase::<u32>::open(&db_path);
        db.set("key1", 42);
        assert_eq!(db.get("key1"), Some(42));

        db.set("key1", 100);
        assert_eq!(db.get("key1"), Some(100));
        assert_eq!(db.len(), 1);
    }

    #[test]
    fn test_invalid_json() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.json");

        // Write invalid JSON
        fs::write(&db_path, "{invalid_json}").unwrap();

        // Should create empty database when JSON is invalid
        let db = LocalDatabase::<u32>::open(&db_path);
        assert!(db.is_empty());
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.json");

        let db = Arc::new(LocalDatabase::<u32>::open(&db_path));
        let mut handles = vec![];

        // Spawn multiple threads to write to the database
        for i in 0..10 {
            let db_clone = Arc::clone(&db);
            let handle = thread::spawn(move || {
                db_clone.set(&format!("key{}", i), i as u32);
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(db.len(), 10);
        for i in 0..10 {
            assert_eq!(db.get(&format!("key{}", i)), Some(i as u32));
        }
    }
}
