use std::{
    fmt,
    ops::{Deref, DerefMut},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Id {
    Single(SingleId),
    Multiple(MultipleIds),
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single(id) => write!(f, "{id}"),
            Self::Multiple(ids) => write!(f, "{ids}"),
        }
    }
}

impl Id {
    pub fn single(id: impl ToString) -> Self {
        Self::Single(id.to_string().into())
    }

    pub fn multiple(id: impl IntoIterator<Item = impl ToString>) -> Self {
        Self::Multiple(id.into())
    }

    pub fn join(&self, sep: impl AsRef<str>) -> String {
        match self {
            Self::Single(id) => id.to_string(),
            Self::Multiple(ids) => ids.join(sep.as_ref()),
        }
    }
}

impl From<SingleId> for Id {
    fn from(id: SingleId) -> Self {
        Self::Single(id)
    }
}

impl From<&SingleId> for Id {
    fn from(id: &SingleId) -> Self {
        Self::Single(id.clone())
    }
}

impl From<MultipleIds> for Id {
    fn from(ids: MultipleIds) -> Self {
        Self::Multiple(ids)
    }
}

impl From<&MultipleIds> for Id {
    fn from(ids: &MultipleIds) -> Self {
        Self::Multiple(ids.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SingleId(String);

impl Deref for SingleId {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SingleId {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: AsRef<str>> From<T> for SingleId {
    fn from(id: T) -> Self {
        Self(id.as_ref().to_owned())
    }
}

impl fmt::Display for SingleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MultipleIds(Vec<String>);

impl Deref for MultipleIds {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MultipleIds {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: IntoIterator<Item = impl ToString>> From<T> for MultipleIds {
    fn from(ids: T) -> Self {
        Self(ids.into_iter().map(|id| id.to_string()).collect())
    }
}

impl fmt::Display for MultipleIds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, id) in self.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{id}")?;
        }
        Ok(())
    }
}
