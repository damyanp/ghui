use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize, Clone, TS)]
#[serde(tag = "loadState", content = "value")]
#[serde(rename_all = "camelCase")]
pub enum DelayLoad<T> {
    #[default]
    NotLoaded,
    Loaded(T),
}

impl<T> DelayLoad<T> {
    pub fn map<U>(&self, f: impl Fn(&T) -> U) -> DelayLoad<U> {
        match self {
            DelayLoad::NotLoaded => DelayLoad::NotLoaded,
            DelayLoad::Loaded(v) => DelayLoad::Loaded(f(v)),
        }
    }

    pub fn expect_loaded(&self) -> &T {
        match self {
            DelayLoad::NotLoaded => panic!(),
            DelayLoad::Loaded(v) => v,
        }
    }
}

impl<T> DelayLoad<Option<T>> {
    pub fn flatten(&self) -> Option<&T> {
        match self {
            DelayLoad::NotLoaded => None,
            DelayLoad::Loaded(v) => v.as_ref(),
        }
    }
}

impl<T> From<T> for DelayLoad<T> {
    fn from(value: T) -> Self {
        DelayLoad::Loaded(value)
    }
}
