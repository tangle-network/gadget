mod error;

pub use error::Error;
use std::io::ErrorKind;

use gadget_std::collections::HashMap;
use gadget_std::fs;
use gadget_std::path::{Path, PathBuf};
use gadget_std::sync::Mutex;
use serde::{de::DeserializeOwned, Serialize};

/// A local database for storing key-value pairs.
///
/// The database is stored in a JSON file, which is updated every time a key-value pair is added or updated.
///
/// # Example
///
/// ```no_run
/// use gadget_store_local_database::LocalDatabase;
///
/// # fn main() -> Result<(), gadget_store_local_database::Error> {
/// let db = LocalDatabase::<u64>::open("data.json")?;
///
/// db.set("key", 42)?;
/// assert_eq!(db.get("key"), Some(42));
/// # Ok(()) }
/// ```
#[derive(Debug)]
pub struct LocalDatabase<T> {
    path: PathBuf,
    data: Mutex<HashMap<String, T>>,
}

impl<T> LocalDatabase<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    /// Reads a `LocalDatabase` from the given path.
    ///
    /// If the file does not exist, an empty database is created.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gadget_store_local_database::LocalDatabase;
    ///
    /// # fn main() -> Result<(), gadget_store_local_database::Error> {
    /// let db = LocalDatabase::<u64>::open("data.json")?;
    /// assert!(db.is_empty());
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    ///
    /// * The parent of `path` is not a directory
    /// * Unable to write to `path`
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref();
        let parent_dir = path.parent().ok_or(Error::Io(std::io::Error::new(
            ErrorKind::NotFound,
            "parent directory not found",
        )))?;

        // Create the parent directory if it doesn't exist
        fs::create_dir_all(parent_dir)?;

        let data = if path.exists() {
            let content = fs::read_to_string(path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            // Create an empty file with default empty JSON object
            let empty_data = HashMap::new();
            let json_string = serde_json::to_string(&empty_data)?;
            fs::write(path, json_string)?;
            empty_data
        };

        Ok(Self {
            path: path.to_owned(),
            data: Mutex::new(data),
        })
    }

    /// Returns the number of key-value pairs in the database.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use gadget_store_local_database::LocalDatabase;
    ///
    /// # fn main() -> Result<(), gadget_store_local_database::Error> {
    /// let db = LocalDatabase::<u64>::open("data.json")?;
    /// assert_eq!(db.len(), 0);
    ///
    /// db.set("key", 42)?;
    /// assert_eq!(db.len(), 1);
    /// # Ok(()) }
    /// ```
    #[allow(clippy::missing_panics_doc)]
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
    /// # fn main() -> Result<(), gadget_store_local_database::Error> {
    /// let db = LocalDatabase::<u64>::open("data.json")?;
    /// assert!(db.is_empty());
    ///
    /// db.set("key", 42)?;
    /// assert!(!db.is_empty());
    /// # Ok(()) }
    /// ```
    #[allow(clippy::missing_panics_doc)]
    pub fn is_empty(&self) -> bool {
        let data = self.data.lock().unwrap();
        data.is_empty()
    }

    /// Adds or updates a key-value pair in the database.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gadget_store_local_database::LocalDatabase;
    ///
    /// # fn main() -> Result<(), gadget_store_local_database::Error> {
    /// let db = LocalDatabase::<u64>::open("data.json")?;
    ///
    /// db.set("key", 42)?;
    /// assert_eq!(db.get("key"), Some(42));
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    ///
    /// * Unable to serialize `T`
    /// * Unable to write to `path`
    #[allow(clippy::missing_panics_doc)]
    pub fn set(&self, key: &str, value: T) -> Result<(), Error> {
        let mut data = self.data.lock().unwrap();
        let _old = data.insert(key.to_string(), value);

        let json_string = serde_json::to_string(&*data)?;
        fs::write(&self.path, json_string)?;

        Ok(())
    }

    /// Retrieves a value associated with the given key.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use gadget_store_local_database::LocalDatabase;
    ///
    /// # fn main() -> Result<(), gadget_store_local_database::Error> {
    /// let db = LocalDatabase::<u64>::open("data.json")?;
    ///
    /// db.set("key", 42);
    /// assert_eq!(db.get("key"), Some(42));
    /// # Ok(()) }
    /// ```
    #[allow(clippy::missing_panics_doc)]
    pub fn get(&self, key: &str) -> Option<T> {
        let data = self.data.lock().unwrap();
        data.get(key).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gadget_std::fs;
    use serde::{Deserialize, Serialize};
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

        let db = LocalDatabase::<u32>::open(&db_path).unwrap();
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
        assert!(db_path.exists());
    }

    #[test]
    fn test_set_and_get() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.json");

        let db = LocalDatabase::<u32>::open(&db_path).unwrap();
        db.set("key1", 42).unwrap();
        db.set("key2", 100).unwrap();

        assert_eq!(db.get("key1"), Some(42));
        assert_eq!(db.get("key2"), Some(100));
        assert_eq!(db.get("nonexistent"), None);
        assert_eq!(db.len(), 2);
    }

    #[test]
    fn test_complex_type() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.json");

        let db = LocalDatabase::<TestStruct>::open(&db_path).unwrap();

        let test_struct = TestStruct {
            field1: "test".to_string(),
            field2: 42,
        };

        db.set("key1", test_struct.clone()).unwrap();
        assert_eq!(db.get("key1"), Some(test_struct));
    }

    #[test]
    fn test_persistence() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.json");

        // Write data
        {
            let db = LocalDatabase::<u32>::open(&db_path).unwrap();
            db.set("key1", 42).unwrap();
            db.set("key2", 100).unwrap();
        }

        // Read data in new instance
        {
            let db = LocalDatabase::<u32>::open(&db_path).unwrap();
            assert_eq!(db.get("key1"), Some(42));
            assert_eq!(db.get("key2"), Some(100));
            assert_eq!(db.len(), 2);
        }
    }

    #[test]
    fn test_overwrite() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.json");

        let db = LocalDatabase::<u32>::open(&db_path).unwrap();
        db.set("key1", 42).unwrap();
        assert_eq!(db.get("key1"), Some(42));

        db.set("key1", 100).unwrap();
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
        let db = LocalDatabase::<u32>::open(&db_path).unwrap();
        assert!(db.is_empty());
    }

    #[test]
    fn test_concurrent_access() {
        use gadget_std::sync::Arc;
        use gadget_std::thread;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.json");

        let db = Arc::new(LocalDatabase::<u32>::open(&db_path).unwrap());
        let mut handles = vec![];

        // Spawn multiple threads to write to the database
        for i in 0..10 {
            let db_clone = Arc::clone(&db);
            let handle = thread::spawn(move || {
                db_clone.set(&format!("key{}", i), i).unwrap();
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(db.len(), 10);
        for i in 0..10 {
            assert_eq!(db.get(&format!("key{}", i)), Some(i));
        }
    }
}
