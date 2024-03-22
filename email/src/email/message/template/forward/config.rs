#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct ForwardTemplateConfig {
    pub posting_style: Option<ForwardTemplatePostingStyle>,
    pub signature_style: Option<ForwardTemplateSignatureStyle>,
    pub quote_headline: Option<String>,
    pub quote_headers: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum ForwardTemplatePostingStyle {
    #[default]
    Top,
    Attached,
}

impl ForwardTemplatePostingStyle {
    pub fn is_top(&self) -> bool {
        matches!(self, Self::Top)
    }

    pub fn is_attached(&self) -> bool {
        matches!(self, Self::Attached)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum ForwardTemplateSignatureStyle {
    #[default]
    Inlined,
    Attached,
    Hidden,
}

impl ForwardTemplateSignatureStyle {
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
