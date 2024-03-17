#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct NewTemplateConfig {
    pub signature_placement: Option<NewTemplateSignaturePlacement>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum NewTemplateSignaturePlacement {
    #[default]
    Inline,
    Attached,
    Nowhere,
}

impl NewTemplateSignaturePlacement {
    pub fn is_inline(&self) -> bool {
        matches!(self, Self::Inline)
    }

    pub fn is_attached(&self) -> bool {
        matches!(self, Self::Attached)
    }

    pub fn is_nowhere(&self) -> bool {
        matches!(self, Self::Nowhere)
    }
}
