#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct NewTemplateConfig {
    pub signature_style: Option<NewTemplateSignatureStyle>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum NewTemplateSignatureStyle {
    #[default]
    Inlined,
    Attached,
    Hidden,
}

impl NewTemplateSignatureStyle {
    pub fn is_inlined(&self) -> bool {
        matches!(self, Self::Inlined)
    }

    pub fn is_attached(&self) -> bool {
        matches!(self, Self::Attached)
    }

    pub fn is_hidden(&self) -> bool {
        matches!(self, Self::Hidden)
    }
}
