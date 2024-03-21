use super::{
    forward::config::ForwardTemplateConfig, new::config::NewTemplateConfig,
    reply::config::ReplyTemplateConfig,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct TemplateConfig {
    /// Configuration dedicated to new templates.
    pub new: Option<NewTemplateConfig>,

    /// Configuration dedicated to reply templates.
    pub reply: Option<ReplyTemplateConfig>,

    /// Configuration dedicated to forward templates.
    pub forward: Option<ForwardTemplateConfig>,
}
