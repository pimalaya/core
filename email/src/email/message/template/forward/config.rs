#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct ForwardTemplateConfig {
    pub signature_placement: Option<ForwardTemplateSignaturePlacement>,
    pub quote_placement: Option<ForwardTemplateQuotePlacement>,
    pub quote_headline_fmt: Option<String>,
    pub quote_headers: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum ForwardTemplateSignaturePlacement {
    #[default]
    Inline,
    Attached,
    Nowhere,
}

impl ForwardTemplateSignaturePlacement {
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum ForwardTemplateQuotePlacement {
    #[default]
    Inline,
    Attached,
}

impl ForwardTemplateQuotePlacement {
    pub fn is_inline(&self) -> bool {
        matches!(self, Self::Inline)
    }

    pub fn is_attached(&self) -> bool {
        matches!(self, Self::Attached)
    }
}
