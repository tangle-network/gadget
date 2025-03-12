use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Deserialize, Default)]
#[serde(transparent)]
pub struct List<T: Default>(pub Vec<T>);

impl<T: Default> From<Vec<T>> for List<T> {
    fn from(value: Vec<T>) -> Self {
        Self(value)
    }
}

impl<T: Default + Serialize> Serialize for List<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.0.is_empty() {
            let mut s = serializer.serialize_seq(Some(1))?;
            s.serialize_element(&T::default())?;
            return s.end();
        }

        <Vec<T>>::serialize(&self.0, serializer)
    }
}
