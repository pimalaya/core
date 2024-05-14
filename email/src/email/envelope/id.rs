use std::{
    fmt,
    ops::{Deref, DerefMut},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Id {
    Single(SingleId),
    Multiple(MultipleIds),
}

impl Id {
    pub fn single(id: impl Into<SingleId>) -> Self {
        Self::Single(id.into())
    }

    pub fn multiple(ids: impl Into<MultipleIds>) -> Self {
        Self::Multiple(ids.into())
    }

    pub fn join(&self, sep: impl AsRef<str>) -> String {
        match self {
            Self::Single(id) => id.to_string(),
            Self::Multiple(ids) => ids.join(sep.as_ref()),
        }
    }

    pub fn iter(&self) -> IdIterator {
        IdIterator::new(self)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single(id) => write!(f, "{}", id.deref()),
            Self::Multiple(ids) => write!(f, "{ids}"),
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

impl SingleId {
    pub fn as_str(&self) -> &str {
        self.deref().as_str()
    }
}

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

impl<T: ToString> From<T> for SingleId {
    fn from(id: T) -> Self {
        Self(id.to_string())
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

pub struct IdIterator<'a> {
    id: &'a Id,
    index: usize,
}

impl<'a> IdIterator<'a> {
    pub fn new(id: &'a Id) -> Self {
        Self { id, index: 0 }
    }
}

impl<'a> Iterator for IdIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match self.id {
            Id::Single(_) if self.index > 0 => None,
            Id::Single(SingleId(id)) => {
                self.index = 1;
                Some(id.as_str())
            }
            Id::Multiple(MultipleIds(ids)) => {
                if self.index < ids.len() {
                    let id = Some(ids[self.index].as_str());
                    self.index += 1;
                    id
                } else {
                    None
                }
            }
        }
    }
}
