use ammonia;
use html_escape;
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
};

use crate::{CompilerBuilder, Result};

type HeaderKey = String;
type PartMime = String;
type PartBody = Vec<u8>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HeaderVal {
    Single(String),
    Multi(Vec<String>),
}

impl Default for HeaderVal {
    fn default() -> Self {
        Self::Single(String::default())
    }
}

/// Represents the show text parts strategy [`TplBuilder`] build
/// option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShowTextPartsStrategy {
    /// Shows plain text parts first. If none of them found, fallback
    /// to HTML.
    PlainOtherwiseHtml,
    /// Shows plain text parts only.
    PlainOnly,
    /// Shows HTML parts first. If none of them found, fallback to
    /// plain text.
    HtmlOtherwisePlain,
    /// Shows HTML parts only.
    HtmlOnly,
}

impl Default for ShowTextPartsStrategy {
    fn default() -> Self {
        Self::PlainOtherwiseHtml
    }
}

/// Represents the show headers [`TplBuilder`] build option.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShowHeaders {
    /// Shows all available headers in [`TplBuilder::headers`].
    All,
    /// Shows only specific headers from [`TplBuilder::headers`] and
    /// overrides the order [`TplBuilder::headers_order`].
    Only(Vec<HeaderKey>),
}

impl Default for ShowHeaders {
    fn default() -> Self {
        Self::All
    }
}

impl ShowHeaders {
    pub fn contains(&self, key: &String) -> bool {
        match self {
            Self::All => true,
            Self::Only(headers) => headers.contains(key),
        }
    }
}

/// Represents the template builder.
///
/// # Examples
///
/// ```
#[doc = include_str!("../examples/tpl-builder.rs")]
/// ```
#[derive(Clone, Debug, Default)]
pub struct TplBuilder {
    /// Represents the template headers hash map.
    pub headers: HashMap<HeaderKey, HeaderVal>,
    /// Represents the template headers order. Each time a header is
    /// added to the template builder, its key is appended to this
    /// list. It can overidden by [`TplBuilder::show_headers`].
    pub headers_order: Vec<HeaderKey>,
    /// Represents the template parts.
    pub parts: Vec<(PartMime, PartBody)>,
    /// Represents the build option that allows you to filter headers
    /// you want to see in the final template. This option overrides
    /// [`TplBuilder::headers_order`] in case its value is
    /// [`ShowHeaders::Only`].
    pub show_headers: ShowHeaders,
    /// Represents the build option that allows you to discard any
    /// part different from `text/plain` and `text/html` in the final
    /// template.
    pub show_text_parts_only: bool,
    /// Represents the show text parts strategy build option. See
    /// [`ShowTextPartsStrategy`].
    pub show_text_parts_strategy: ShowTextPartsStrategy,
    /// Represents the build option that sanitizes text/plain parts.
    pub sanitize_text_plain_parts: bool,
    /// Represents the build option that sanitizes text/html parts.
    pub sanitize_text_html_parts: bool,
    /// Represents the build option that removes signature from
    /// text/plain parts.
    pub remove_text_plain_parts_signature: bool,
}

impl TplBuilder {
    /// Inserts a raw header in the form "key: val" to the template
    /// builder. Any existing value is replaced by the new one.
    pub fn set_raw_header<H: AsRef<str> + ToString>(mut self, header: H) -> Self {
        if let Some((key, val)) = header.as_ref().split_once(':') {
            self = self.set_header(key.trim(), val.trim());
        }
        self
    }

    pub fn set_raw_headers<'a, I: AsRef<str> + ToString, H: IntoIterator<Item = I>>(
        self,
        headers: H,
    ) -> Self {
        headers
            .into_iter()
            .fold(self, |self_, header| self_.set_raw_header(header))
    }

    pub fn set_some_raw_headers<'a, I: AsRef<str> + ToString, H: IntoIterator<Item = I>>(
        self,
        headers: Option<H>,
    ) -> Self {
        if let Some(headers) = headers {
            self.set_raw_headers(headers)
        } else {
            self
        }
    }

    /// Inserts a pair of key/val header to the template builder. Any
    /// existing value is replaced by the new one.
    pub fn set_header<K: AsRef<str> + ToString, V: ToString>(mut self, key: K, val: V) -> Self {
        if let Some(prev_val) = self.headers.get_mut(key.as_ref()) {
            *prev_val = HeaderVal::Single(val.to_string());
        } else {
            self.headers
                .insert(key.to_string(), HeaderVal::Single(val.to_string()));
            self.headers_order.push(key.to_string());
        }
        self
    }

    /// Inserts a pair of key/val header to the template builder. Any
    /// existing value is merged in a [`HeaderVal::Multi`]. Useful for
    /// address style headers like From, To, Cc, Bcc etc.
    pub fn push_header<K: AsRef<str> + ToString, V: ToString>(mut self, key: K, val: V) -> Self {
        if let Some(prev_val) = self.headers.get_mut(key.as_ref()) {
            match prev_val {
                HeaderVal::Single(prev_single_val) => {
                    *prev_val = HeaderVal::Multi(vec![prev_single_val.clone(), val.to_string()])
                }
                HeaderVal::Multi(prev_vals) => prev_vals.push(val.to_string()),
            }
        } else {
            self.headers
                .insert(key.to_string(), HeaderVal::Multi(vec![val.to_string()]));
            self.headers_order.push(key.to_string());
        }
        self
    }

    pub fn message_id<H: ToString>(self, header: H) -> Self {
        self.set_header("Message-ID", header)
    }

    pub fn from<H: ToString>(self, header: H) -> Self {
        self.set_header("From", header)
    }

    pub fn to<H: ToString>(self, header: H) -> Self {
        self.push_header("To", header)
    }

    pub fn in_reply_to<H: ToString>(self, header: H) -> Self {
        self.set_header("In-Reply-To", header)
    }

    pub fn cc<H: ToString>(self, header: H) -> Self {
        self.push_header("Cc", header)
    }

    pub fn bcc<H: ToString>(self, header: H) -> Self {
        self.push_header("Bcc", header)
    }

    pub fn subject<H: ToString>(self, header: H) -> Self {
        self.set_header("Subject", header)
    }

    /// Appends a part to the template builder.
    pub fn part<M: AsRef<str> + ToString, P: AsRef<[u8]>>(mut self, mime: M, part: P) -> Self {
        self.parts
            .push((mime.to_string(), part.as_ref().to_owned()));
        self
    }

    pub fn text_plain_part<P: AsRef<[u8]>>(self, part: P) -> Self {
        self.part("text/plain", part)
    }

    pub fn some_text_plain_part<P: AsRef<[u8]>>(self, part: Option<P>) -> Self {
        if let Some(part) = part {
            self.text_plain_part(part)
        } else {
            self
        }
    }

    pub fn text_html_part<P: AsRef<[u8]>>(self, part: P) -> Self {
        self.part("text/html", part)
    }

    pub fn some_text_html_part<P: AsRef<[u8]>>(self, part: Option<P>) -> Self {
        if let Some(part) = part {
            self.text_html_part(part)
        } else {
            self
        }
    }

    /// Shows all available headers for the current template
    /// builder. See [TplBuilder::show_headers] for more information
    /// about the `show_headers` build option.
    pub fn show_all_headers(mut self) -> Self {
        self.show_headers = ShowHeaders::All;
        self
    }

    /// Appends headers filters to the template builder. See
    /// [TplBuilder::show_headers] for more information about the
    /// `show_headers` build option.
    pub fn show_headers<S: ToString, B: IntoIterator<Item = S>>(mut self, headers: B) -> Self {
        let headers = headers
            .into_iter()
            .map(|header| header.to_string())
            .collect();

        match self.show_headers {
            ShowHeaders::All => {
                self.show_headers = ShowHeaders::Only(headers);
            }
            ShowHeaders::Only(prev_headers) => {
                let mut prev_headers = prev_headers.clone();
                prev_headers.extend(headers);
                self.show_headers = ShowHeaders::Only(prev_headers);
            }
        };

        self
    }

    /// Appends a header to show to the template builder. See
    /// [TplBuilder::show_headers] for more information about the
    /// `show_headers` build option.
    pub fn show_header<H: ToString>(self, header: H) -> Self {
        self.show_headers([header])
    }

    /// Sets the [`TplBuilder::show_text_parts_only`] build option.
    pub fn show_text_parts_only(mut self, show_text_parts_only: bool) -> Self {
        self.show_text_parts_only = show_text_parts_only;
        self
    }

    /// Sets the [`TplBuilder::show_text_parts_strategy`] build
    /// option.
    pub fn use_show_text_parts_strategy(mut self, strategy: ShowTextPartsStrategy) -> Self {
        self.show_text_parts_strategy = strategy;
        self
    }

    /// Sets the [`TplBuilder::sanitize_text_plain_parts`] build
    /// option.
    pub fn sanitize_text_plain_parts(mut self, sanitize: bool) -> Self {
        self.sanitize_text_plain_parts = sanitize;
        self
    }

    /// Sets the [`TplBuilder::sanitize_text_html_parts`] build
    /// option.
    pub fn sanitize_text_html_parts(mut self, sanitize: bool) -> Self {
        self.sanitize_text_html_parts = sanitize;
        self
    }

    /// Sets the [`TplBuilder::sanitize_text_plain_parts`] and the
    /// [`TplBuilder::sanitize_text_html_parts`] build options.
    pub fn sanitize_text_parts(self, sanitize: bool) -> Self {
        self.sanitize_text_plain_parts(sanitize)
            .sanitize_text_html_parts(sanitize)
    }

    /// Sets the [`TplBuilder::remove_text_plain_parts_signature`]
    /// build option.
    pub fn remove_text_plain_parts_signature(mut self, remove_signature: bool) -> Self {
        self.remove_text_plain_parts_signature = remove_signature;
        self
    }

    pub fn build(&self) -> Tpl {
        let mut tpl = Tpl::default();

        let headers_order = if let ShowHeaders::Only(headers) = &self.show_headers {
            headers
        } else {
            &self.headers_order
        };

        for key in headers_order {
            if let Some(val) = self.headers.get(key) {
                match val {
                    HeaderVal::Single(val) => tpl.push_str(&format!("{}: {}\n", key, val)),
                    HeaderVal::Multi(vals) => {
                        tpl.push_str(&format!("{}: {}\n", key, vals.join(", ")))
                    }
                };
            }
        }

        let mut plain: Option<String> = None;
        let mut html: Option<String> = None;

        for (mime, body) in &self.parts {
            match mime.to_lowercase().as_str() {
                "text/plain" => {
                    let mut next_plain = String::from_utf8_lossy(body).to_string();

                    if self.remove_text_plain_parts_signature {
                        next_plain = next_plain
                            .rsplit_once("-- \n")
                            .map(|(body, _signature)| body.to_owned())
                            .unwrap_or(next_plain);
                    }

                    match plain.as_mut() {
                        None => plain = Some(next_plain),
                        Some(plain) => {
                            plain.push_str("\n\n");
                            plain.push_str(&next_plain)
                        }
                    }
                }
                "text/html" => {
                    let next_html = String::from_utf8_lossy(body).to_string();

                    match html.as_mut() {
                        None => html = Some(next_html),
                        Some(html) => {
                            html.push_str("\n\n");
                            html.push_str(&next_html)
                        }
                    }
                }
                // TODO: manage other mimes
                _ => (),
            }
        }

        if self.sanitize_text_plain_parts {
            if let Some(plain) = plain.as_mut() {
                // keeps a maximum of 2 consecutive new lines
                *plain = Regex::new(r"(\r?\n\s*){2,}")
                    .unwrap()
                    .replace_all(&plain, "\n\n")
                    .to_string();

                // replaces tabulations by spaces
                *plain = Regex::new(r"\t")
                    .unwrap()
                    .replace_all(&plain, " ")
                    .to_string();

                // keeps a maximum of 2 consecutive spaces
                *plain = Regex::new(r" {2,}")
                    .unwrap()
                    .replace_all(&plain, "  ")
                    .to_string();
            }
        }

        if self.sanitize_text_html_parts {
            if let Some(html) = html.as_mut() {
                // removes html markup
                *html = ammonia::Builder::new()
                    .tags(HashSet::default())
                    .clean(&html)
                    .to_string();
                // merges new line chars
                *html = Regex::new(r"(\r?\n\s*){2,}")
                    .unwrap()
                    .replace_all(&html, "\n\n")
                    .to_string();
                // replaces tabulations and &npsp; by spaces
                *html = Regex::new(r"(\t|&nbsp;)")
                    .unwrap()
                    .replace_all(&html, " ")
                    .to_string();
                // merges spaces
                *html = Regex::new(r" {2,}")
                    .unwrap()
                    .replace_all(&html, "  ")
                    .to_string();
                // decodes html entities
                *html = html_escape::decode_html_entities(&html).to_string();
            }
        }

        let text = match self.show_text_parts_strategy {
            ShowTextPartsStrategy::PlainOtherwiseHtml => plain
                .clone()
                .filter(|plain| !plain.trim().is_empty())
                .or(html.filter(|html| !html.trim().is_empty()))
                .or(plain),
            ShowTextPartsStrategy::PlainOnly => plain,
            ShowTextPartsStrategy::HtmlOtherwisePlain => html
                .clone()
                .filter(|html| !html.trim().is_empty())
                .or(plain.filter(|plain| !plain.trim().is_empty()))
                .or(html),
            ShowTextPartsStrategy::HtmlOnly => html,
        };

        if let Some(ref text) = text {
            // adds new line delimiter only if headers are not empty
            if !tpl.is_empty() {
                tpl.push_str("\n");
            }
            tpl.push_str(text)
        }

        tpl
    }

    pub fn compile(&self, compiler: CompilerBuilder) -> Result<Vec<u8>> {
        Ok(self.build().compile(compiler)?)
    }
}

/// Represents the template built by the template builder.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Tpl(String);

impl Tpl {
    pub fn compile(&self, compiler: CompilerBuilder) -> Result<Vec<u8>> {
        Ok(compiler.compile(&self.0)?)
    }
}

impl Into<String> for Tpl {
    fn into(self) -> String {
        self.0
    }
}

impl Deref for Tpl {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Tpl {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<&str> for Tpl {
    fn from(tpl: &str) -> Self {
        Self(tpl.to_owned())
    }
}

impl From<String> for Tpl {
    fn from(tpl: String) -> Self {
        Self(tpl)
    }
}

#[cfg(test)]
mod builder {
    use concat_with::concat_line;
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn set_header_twice() {
        let tpl = TplBuilder::default()
            .set_header("key1", "val1")
            .set_header("key1", "val2")
            .set_header("key3", "val3");

        assert_eq!(
            tpl.headers,
            HashMap::from_iter([
                (
                    String::from("key1"),
                    HeaderVal::Single(String::from("val2"))
                ),
                (
                    String::from("key3"),
                    HeaderVal::Single(String::from("val3"))
                ),
            ])
        );

        assert_eq!(
            tpl.headers_order,
            vec![String::from("key1"), String::from("key3")]
        );
    }

    #[test]
    fn push_then_set_header() {
        let tpl = TplBuilder::default()
            .push_header("key1", "val1")
            .set_header("key1", "val2")
            .set_header("key3", "val3");

        assert_eq!(
            tpl.headers,
            HashMap::from_iter([
                (
                    String::from("key1"),
                    HeaderVal::Single(String::from("val2"))
                ),
                (
                    String::from("key3"),
                    HeaderVal::Single(String::from("val3"))
                ),
            ])
        );

        assert_eq!(
            tpl.headers_order,
            vec![String::from("key1"), String::from("key3")]
        );
    }

    #[test]
    fn push_header_twice() {
        let tpl = TplBuilder::default()
            .push_header("key1", "val1")
            .push_header("key1", "val2")
            .push_header("key3", "val3");

        assert_eq!(
            tpl.headers,
            HashMap::from_iter([
                (
                    String::from("key1"),
                    HeaderVal::Multi(vec![String::from("val1"), String::from("val2")])
                ),
                (
                    String::from("key3"),
                    HeaderVal::Multi(vec![String::from("val3")])
                ),
            ])
        );

        assert_eq!(
            tpl.headers_order,
            vec![String::from("key1"), String::from("key3")]
        );
    }

    #[test]
    fn set_then_push_header() {
        let tpl = TplBuilder::default()
            .set_header("key1", "val1")
            .push_header("key1", "val2")
            .push_header("key3", "val3");

        assert_eq!(
            tpl.headers,
            HashMap::from_iter([
                (
                    String::from("key1"),
                    HeaderVal::Multi(vec![String::from("val1"), String::from("val2")])
                ),
                (
                    String::from("key3"),
                    HeaderVal::Multi(vec![String::from("val3")])
                ),
            ])
        );

        assert_eq!(
            tpl.headers_order,
            vec![String::from("key1"), String::from("key3")]
        );
    }

    #[test]
    fn part() {
        let tpl = TplBuilder::default()
            .part("mime1", [21])
            .part("mime1", [42])
            .part("mime2", [84]);

        assert_eq!(
            tpl.parts,
            vec![
                (String::from("mime1"), vec![21]),
                (String::from("mime1"), vec![42]),
                (String::from("mime2"), vec![84]),
            ]
        );
    }

    #[test]
    fn show_all_then_only_headers() {
        let tpl = TplBuilder::default()
            .show_all_headers()
            .show_header("header1")
            .show_headers(["header2", "header3"]);

        assert_eq!(
            tpl.show_headers,
            ShowHeaders::Only(vec![
                String::from("header1"),
                String::from("header2"),
                String::from("header3")
            ])
        );
    }

    #[test]
    fn show_only_then_all_headers() {
        let tpl = TplBuilder::default()
            .show_header("header1")
            .show_headers(["header2", "header3"])
            .show_all_headers();

        assert_eq!(tpl.show_headers, ShowHeaders::All);
    }

    #[test]
    fn show_all_headers_order() {
        let tpl = TplBuilder::default()
            .show_all_headers()
            .set_header("key1", "val1")
            .set_header("key2", "val2")
            .set_header("key3", "val3")
            .push_header("key4", "val4")
            .push_header("key4", "val5")
            .build();

        assert_eq!(
            *tpl,
            concat_line!(
                "key1: val1",
                "key2: val2",
                "key3: val3",
                "key4: val4, val5",
                ""
            )
        );
    }

    #[test]
    fn show_only_headers_order() {
        let tpl = TplBuilder::default()
            .show_headers(["key3", "key1"])
            .set_header("key1", "val1")
            .set_header("key2", "val2")
            .set_header("key3", "val3")
            .push_header("key4", "val4")
            .push_header("key4", "val5")
            .build();

        assert_eq!(*tpl, concat_line!("key3: val3", "key1: val1", ""));
    }

    #[test]
    fn plain_otherwise_html() {
        assert_eq!(
            *TplBuilder::default()
                .use_show_text_parts_strategy(ShowTextPartsStrategy::PlainOtherwiseHtml)
                .set_header("key1", "val1")
                .text_html_part("html")
                .build(),
            concat_line!("key1: val1", "", "html")
        );
        assert_eq!(
            *TplBuilder::default()
                .use_show_text_parts_strategy(ShowTextPartsStrategy::PlainOtherwiseHtml)
                .text_plain_part("plain")
                .build(),
            "plain"
        );
        assert_eq!(
            *TplBuilder::default()
                .use_show_text_parts_strategy(ShowTextPartsStrategy::PlainOtherwiseHtml)
                .text_plain_part("plain")
                .text_html_part("html")
                .build(),
            "plain"
        );
        assert_eq!(
            *TplBuilder::default()
                .use_show_text_parts_strategy(ShowTextPartsStrategy::PlainOtherwiseHtml)
                .text_plain_part("  \t")
                .text_html_part("html")
                .build(),
            "html"
        );
    }

    #[test]
    fn plain_only() {
        assert_eq!(
            *TplBuilder::default()
                .use_show_text_parts_strategy(ShowTextPartsStrategy::PlainOnly)
                .set_header("key1", "val1")
                .text_html_part("html")
                .build(),
            concat_line!("key1: val1", "")
        );
        assert_eq!(
            *TplBuilder::default()
                .use_show_text_parts_strategy(ShowTextPartsStrategy::PlainOnly)
                .text_plain_part("plain")
                .build(),
            "plain"
        );
        assert_eq!(
            *TplBuilder::default()
                .use_show_text_parts_strategy(ShowTextPartsStrategy::PlainOnly)
                .text_plain_part("plain")
                .text_html_part("html")
                .build(),
            "plain"
        );
    }

    #[test]
    fn html_otherwise_plain() {
        assert_eq!(
            *TplBuilder::default()
                .use_show_text_parts_strategy(ShowTextPartsStrategy::HtmlOtherwisePlain)
                .set_header("key1", "val1")
                .text_html_part("html")
                .build(),
            concat_line!("key1: val1", "", "html")
        );
        assert_eq!(
            *TplBuilder::default()
                .use_show_text_parts_strategy(ShowTextPartsStrategy::HtmlOtherwisePlain)
                .text_plain_part("plain")
                .build(),
            "plain"
        );
        assert_eq!(
            *TplBuilder::default()
                .use_show_text_parts_strategy(ShowTextPartsStrategy::HtmlOtherwisePlain)
                .text_plain_part("plain")
                .text_html_part("html")
                .build(),
            "html"
        );
    }

    #[test]
    fn html_only() {
        assert_eq!(
            *TplBuilder::default()
                .use_show_text_parts_strategy(ShowTextPartsStrategy::HtmlOnly)
                .set_header("key1", "val1")
                .text_html_part("html")
                .build(),
            concat_line!("key1: val1", "", "html")
        );
        assert_eq!(
            *TplBuilder::default()
                .use_show_text_parts_strategy(ShowTextPartsStrategy::HtmlOnly)
                .text_plain_part("plain")
                .build(),
            ""
        );
        assert_eq!(
            *TplBuilder::default()
                .use_show_text_parts_strategy(ShowTextPartsStrategy::HtmlOnly)
                .text_plain_part("plain")
                .text_html_part("html")
                .build(),
            "html"
        );
    }
}
