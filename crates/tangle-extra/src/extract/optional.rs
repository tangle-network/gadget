use serde::{Deserialize, Serialize, Serializer};

#[derive(Deserialize)]
#[serde(transparent)]
pub struct Optional<T: Default>(pub Option<T>);

impl<T: Default> Default for Optional<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T: Default> From<Option<T>> for Optional<T> {
    fn from(value: Option<T>) -> Self {
        Self(value)
    }
}

impl<T: Default + Serialize> Serialize for Optional<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.0.is_none() {
            return serializer.serialize_some(&T::default());
        }

        <Option<T>>::serialize(&self.0, serializer)
    }
}
