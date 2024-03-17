#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct ReplyTemplateConfig {
    pub signature_placement: Option<ReplyTemplateSignaturePlacement>,
    pub quote_placement: Option<ReplyTemplateQuotePlacement>,
    pub quote_headline_fmt: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum ReplyTemplateSignaturePlacement {
    AboveQuote,
    #[default]
    BelowQuote,
    Attached,
    Nowhere,
}

impl ReplyTemplateSignaturePlacement {
    pub fn is_above_quote(&self) -> bool {
        matches!(self, Self::AboveQuote)
    }

    pub fn is_below_quote(&self) -> bool {
        matches!(self, Self::BelowQuote)
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
pub enum ReplyTemplateQuotePlacement {
    AboveReply,
    #[default]
    BelowReply,
    Nowhere,
}

impl ReplyTemplateQuotePlacement {
    pub fn is_above_reply(&self) -> bool {
        matches!(self, Self::AboveReply)
    }

    pub fn is_below_reply(&self) -> bool {
        matches!(self, Self::BelowReply)
    }

    pub fn is_nowhere(&self) -> bool {
        matches!(self, Self::Nowhere)
    }
}
