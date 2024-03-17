#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct ForwardTemplateConfig {
    pub signature_placement: Option<SignaturePlacement>,
    pub quote_placement: Option<QuotePlacement>,
    pub quote_headline_fmt: Option<String>,
    pub quote_headers: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum SignaturePlacement {
    #[default]
    Inline,
    Attached,
    Nowhere,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum QuotePlacement {
    #[default]
    Inline,
    Attached,
}
