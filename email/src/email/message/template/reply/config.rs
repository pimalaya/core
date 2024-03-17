#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct ReplyTemplateConfig {
    pub signature_placement: Option<SignaturePlacement>,
    pub quote_placement: Option<QuotePlacement>,
    pub quote_headline_fmt: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum SignaturePlacement {
    AboveQuote,
    #[default]
    BelowQuote,
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
    AboveReply,
    #[default]
    BelowReply,
    Nowhere,
}
