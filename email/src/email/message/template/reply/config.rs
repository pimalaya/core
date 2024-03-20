#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct ReplyTemplateConfig {
    pub posting_style: Option<ReplyTemplatePostingStyle>,
    pub signing_style: Option<ReplyTemplateSigningStyle>,
    pub quote_headline_fmt: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum ReplyTemplatePostingStyle {
    #[default]
    Top,
    Bottom,
    Interleaved,
}

impl ReplyTemplatePostingStyle {
    pub fn is_top(&self) -> bool {
        matches!(self, Self::Top)
    }

    pub fn is_bottom(&self) -> bool {
        matches!(self, Self::Bottom)
    }

    pub fn is_interleaved(&self) -> bool {
        matches!(self, Self::Interleaved)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum ReplyTemplateSigningStyle {
    AboveQuote,
    #[default]
    BelowQuote,
    Attachment,
    Hidden,
}

impl ReplyTemplateSigningStyle {
    pub fn is_above_quote(&self) -> bool {
        matches!(self, Self::AboveQuote)
    }

    pub fn is_below_quote(&self) -> bool {
        matches!(self, Self::BelowQuote)
    }

    pub fn is_attachment(&self) -> bool {
        matches!(self, Self::Attachment)
    }

    pub fn is_hidden(&self) -> bool {
        matches!(self, Self::Hidden)
    }
}
