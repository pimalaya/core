use std::collections::HashMap;

use crate::mml::tokens::{Part, FILENAME};

use super::{
    disposition, encrypt, filename, multipart_type, name, part_type, prelude::*, sign,
    GREATER_THAN, NEW_LINE,
};

pub(crate) const SINGLE_PART_BEGIN: &str = "<#part";
pub(crate) const SINGLE_PART_END: &str = "<#/part>";
pub(crate) const MULTI_PART_BEGIN: &str = "<#multipart";
pub(crate) const MULTI_PART_END: &str = "<#/multipart>";

/// Represents the template parser. It parses MIME headers followed by
/// parts.
pub(crate) fn parts() -> impl Parser<char, Vec<Part>, Error = Simple<char>> {
    choice((
        multi_part(),
        attachment(),
        single_part(),
        text_plain_part().map(Part::TextPlainPart),
    ))
    .repeated()
    // .padded()
    .then_ignore(end())
}

/// Represents the plain text part parser. It parses everything that
/// is inside and outside (multi)parts.
pub(crate) fn text_plain_part() -> impl Parser<char, String, Error = Simple<char>> {
    choice((
        just(SINGLE_PART_BEGIN),
        just(SINGLE_PART_END),
        just(MULTI_PART_BEGIN),
        just(MULTI_PART_END),
    ))
    .not()
    .repeated()
    .at_least(1)
    .collect()
}

/// Represents the attachment parser. The attachment is a part that
/// closes straight after its declaration and therefore does not take
/// any children. It takes instead a special (and required) property
/// filename which should be an expandable path to a valid file. The
/// binary content of the file will be used as content of the
/// part. The content type is automatically detected by
/// [tree_magic](https://github.com/aahancoc/tree_magic/), although it
/// can be overriden by the `type` property.
///
/// # Examples
///
/// ```mml,ignore
/// <#part filename="/absolute/path/to/file with space.ext">
/// <#part filename=/absolute/path/to/file.ext signed=command>
/// <#part filename=./relative/path/to/file.ext encrypted=command>
/// <#part filename=~/path/to/file.ext encrypted=command signed=command>
/// <#part filename=$XDG_DATA_HOME/path/to/file.ext>
/// ```
pub(crate) fn attachment() -> impl Parser<char, Part, Error = Simple<char>> {
    choice((
        part_type(),
        filename(),
        name(),
        disposition(),
        encrypt(),
        sign(),
    ))
    .repeated()
    .try_map(|props, span| {
        if let Some(_) = props.iter().find(|(key, _)| key == FILENAME) {
            Ok(props)
        } else {
            Err(Simple::custom(span, "missing attachment property filename"))
        }
    })
    .delimited_by(
        just(SINGLE_PART_BEGIN),
        just(GREATER_THAN).then_ignore(just(NEW_LINE).or_not()),
    )
    .map(HashMap::from_iter)
    .map(Part::Attachment)
}

/// Represents the single part parser. It parses a full part,
/// including properties and content till the next opening part or the
/// next closing part/multipart.
pub(crate) fn single_part() -> impl Parser<char, Part, Error = Simple<char>> {
    just(SINGLE_PART_BEGIN)
        .ignore_then(
            choice((part_type(), name(), disposition(), encrypt(), sign()))
                .repeated()
                .then_ignore(just(GREATER_THAN))
                .then_ignore(just(NEW_LINE).or_not())
                .map(HashMap::from_iter),
        )
        .then(text_plain_part())
        .then_ignore(just(SINGLE_PART_END).then(just(NEW_LINE).or_not()).or_not())
        .map(Part::SinglePart)
}

/// Represents the multipart parser. It parses everything between tags
/// `<#multipart>` and `<#/multipart>`. A multipart can contain
/// multiple parts as well as multiple other multiparts
/// (recursively). This parser is useful when you need to group parts
/// together instead of having them at the root level.
///
/// # Examples
///
/// From https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/Advanced-MML-Example.html:
///
/// ```ignore
/// // <#multipart type=mixed>
/// //   <#part type=image/jpeg filename=~/rms.jpg disposition=inline>
/// //   <#multipart type=alternative>
/// //     This is a plain text part.
/// //     <#part type=text/enriched name=enriched.txt>
/// //     <center>This is a centered enriched part</center>
/// //   <#/multipart>
/// //   This is a new plain text part.
/// //   <#part disposition=attachment>
/// //   This plain text part is an attachment.
/// // <#/multipart>
/// ```
pub(crate) fn multi_part() -> impl Parser<char, Part, Error = Simple<char>> {
    recursive(|multipart| {
        just(MULTI_PART_BEGIN)
            .ignore_then(
                choice((multipart_type(), encrypt(), sign()))
                    .repeated()
                    .then_ignore(just(GREATER_THAN))
                    .then_ignore(just(NEW_LINE).or_not())
                    .map(HashMap::from_iter),
            )
            .then(
                choice((
                    multipart,
                    attachment(),
                    single_part(),
                    text_plain_part().map(Part::TextPlainPart),
                ))
                .repeated()
                .then_ignore(just(MULTI_PART_END))
                .then_ignore(just(NEW_LINE).or_not()),
            )
            .map(Part::MultiPart)
    })
}

#[cfg(test)]
mod parts {
    use concat_with::concat_line;
    use std::collections::HashMap;

    use crate::mml::{
        parsers::prelude::*,
        tokens::{Part, DISPOSITION, FILENAME, NAME, TYPE},
    };

    #[test]
    fn single_part_no_new_line() {
        assert_eq!(
            super::single_part().parse("<#part>This is a plain text part."),
            Ok(Part::SinglePart((
                HashMap::default(),
                String::from("This is a plain text part.")
            ))),
        );

        assert_eq!(
            super::single_part().parse("<#part>This is a plain text part.<#part>ignored"),
            Ok(Part::SinglePart((
                HashMap::default(),
                String::from("This is a plain text part.")
            ))),
        );

        assert_eq!(
            super::single_part().parse("<#part>This is a plain text part.<#/part>"),
            Ok(Part::SinglePart((
                HashMap::default(),
                String::from("This is a plain text part.")
            ))),
        );

        assert_eq!(
            super::single_part().parse("<#part>This is a plain text part.<#multipart>ignored"),
            Ok(Part::SinglePart((
                HashMap::default(),
                String::from("This is a plain text part.")
            ))),
        );

        assert_eq!(
            super::single_part().parse("<#part>This is a plain text part.<#/multipart>"),
            Ok(Part::SinglePart((
                HashMap::default(),
                String::from("This is a plain text part.")
            ))),
        );
    }

    #[test]
    fn single_part_new_line() {
        assert_eq!(
            super::single_part().parse(concat_line!("<#part>", "This is a plain text part.")),
            Ok(Part::SinglePart((
                HashMap::default(),
                String::from("This is a plain text part.")
            ))),
        );

        assert_eq!(
            super::single_part().parse(concat_line!(
                "<#part>",
                "This is a plain text part.",
                "",
                "<#/part>",
            )),
            Ok(Part::SinglePart((
                HashMap::default(),
                String::from("This is a plain text part.\n\n")
            ))),
        );
    }

    #[test]
    fn single_html_part() {
        assert_eq!(
            super::single_part().parse(concat_line!(
                "<#part type=text/html>",
                "<h1>This is a HTML text part.</h1>",
                "<#/part>"
            )),
            Ok(Part::SinglePart((
                HashMap::from_iter([(TYPE.into(), "text/html".into())]),
                String::from("<h1>This is a HTML text part.</h1>\n"),
            ))),
        );
    }

    #[test]
    fn attachment() {
        assert_eq!(
            super::attachment()
                .parse("<#part type=image/jpeg filename=~/rms.jpg disposition=inline>"),
            Ok(Part::Attachment(HashMap::from_iter([
                (TYPE.into(), "image/jpeg".into()),
                (FILENAME.into(), "~/rms.jpg".into()),
                (DISPOSITION.into(), "inline".into())
            ]))),
        );
    }

    #[test]
    fn multi_part() {
        assert_eq!(
            super::multi_part().parse(concat_line!(
                "<#multipart>",
                "This is a plain text part.",
                "<#/multipart>"
            )),
            Ok(Part::MultiPart((
                HashMap::default(),
                vec![Part::TextPlainPart(String::from(
                    "This is a plain text part.\n"
                ))]
            ))),
        );
    }

    #[test]
    fn nested_multi_part() {
        assert_eq!(
            super::multi_part().parse(concat_line!(
                "<#multipart>",
                "<#multipart>",
                "This is a plain text part.",
                "<#/multipart>",
                "<#/multipart>"
            )),
            Ok(Part::MultiPart((
                HashMap::default(),
                vec![Part::MultiPart((
                    HashMap::default(),
                    vec![Part::TextPlainPart(String::from(
                        "This is a plain text part.\n"
                    ))]
                ))]
            ))),
        );

        assert_eq!(
            super::multi_part().parse(concat_line!(
                "<#multipart>",
                "<#multipart>",
                "<#multipart>",
                "<#multipart>",
                "This is a plain text part.",
                "<#/multipart>",
                "<#/multipart>",
                "<#/multipart>",
                "<#/multipart>"
            )),
            Ok(Part::MultiPart((
                HashMap::default(),
                vec![Part::MultiPart((
                    HashMap::default(),
                    vec![Part::MultiPart((
                        HashMap::default(),
                        vec![Part::MultiPart((
                            HashMap::default(),
                            vec![Part::TextPlainPart(String::from(
                                "This is a plain text part.\n"
                            ))]
                        ))]
                    ))]
                ))]
            ))),
        );
    }

    #[test]
    fn adjacent_multi_part() {
        assert_eq!(
            super::multi_part().parse(concat_line!(
                "<#multipart>",
                "<#multipart>",
                "This is a plain text part.",
                "<#/multipart>",
                "<#multipart>",
                "This is a new plain text part.",
                "<#/multipart>",
                "<#/multipart>"
            )),
            Ok(Part::MultiPart((
                HashMap::default(),
                vec![
                    Part::MultiPart((
                        HashMap::default(),
                        vec![Part::TextPlainPart(String::from(
                            "This is a plain text part.\n"
                        ))]
                    )),
                    Part::MultiPart((
                        HashMap::default(),
                        vec![Part::TextPlainPart(String::from(
                            "This is a new plain text part.\n"
                        ))]
                    ))
                ]
            ))),
        );
    }

    // Simple example from the [Emacs MML] module.
    //
    // [Emacs MML]: https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/Simple-MML-Example.html
    #[test]
    fn simple_mml() {
        assert_eq!(
            super::parts()
                .parse(concat_line!(
                    "<#multipart type=alternative>",
                    "This is a plain text part.",
                    "<#part type=text/enriched>",
                    "<center>This is a centered enriched part</center>",
                    "<#/multipart>",
                ))
                .unwrap(),
            vec![Part::MultiPart((
                HashMap::from_iter([(TYPE.into(), "alternative".into())]),
                vec![
                    Part::TextPlainPart("This is a plain text part.\n".into()),
                    Part::SinglePart((
                        HashMap::from_iter([(TYPE.into(), "text/enriched".into())]),
                        String::from("<center>This is a centered enriched part</center>\n")
                    ))
                ]
            ))],
        );
    }

    // Advanced example from the [Emacs MML] module.
    //
    // [Emacs MML]: https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/Advanced-MML-Example.html
    #[test]
    fn advanced_mml() {
        assert_eq!(
            super::parts()
                .parse(concat_line!(
                    "<#multipart type=mixed>",
                    "<#part type=image/jpeg filename=~/rms.jpg disposition=inline>",
                    "<#multipart type=alternative>",
                    "This is a plain text part.",
                    "<#part type=text/enriched name=enriched.txt>",
                    "<center>This is a centered enriched part</center>",
                    "<#/multipart>",
                    "This is a new plain text part.",
                    "<#part disposition=attachment>",
                    "This plain text part is an attachment.",
                    "<#/multipart>",
                ))
                .unwrap(),
            vec![Part::MultiPart((
                HashMap::from_iter([(TYPE.into(), "mixed".into())]),
                vec![
                    Part::Attachment(HashMap::from_iter([
                        (TYPE.into(), "image/jpeg".into()),
                        (FILENAME.into(), "~/rms.jpg".into()),
                        (DISPOSITION.into(), "inline".into())
                    ])),
                    Part::MultiPart((
                        HashMap::from_iter([(TYPE.into(), "alternative".into())]),
                        vec![
                            Part::TextPlainPart("This is a plain text part.\n".into()),
                            Part::SinglePart((
                                HashMap::from_iter([
                                    (TYPE.into(), "text/enriched".into()),
                                    (NAME.into(), "enriched.txt".into())
                                ]),
                                "<center>This is a centered enriched part</center>\n".into(),
                            ))
                        ]
                    )),
                    Part::TextPlainPart("This is a new plain text part.\n".into()),
                    Part::SinglePart((
                        HashMap::from_iter([(DISPOSITION.into(), "attachment".into())]),
                        "This plain text part is an attachment.\n".into(),
                    ))
                ]
            ))]
        );
    }
}
