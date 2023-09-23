use std::collections::HashMap;

use crate::message::body::{
    compiler::tokens::{Part, Props},
    GREATER_THAN, MULTI_PART_BEGIN, MULTI_PART_END,
};

use super::{description, disposition, filename, multipart_type, name, part_type, prelude::*};
#[cfg(feature = "pgp")]
use super::{encrypt, sign};

/// The parts parser.
///
/// It parses all parts the MML body is composed of.
pub(crate) fn parts<'a>() -> impl Parser<'a, &'a str, Vec<Part<'a>>, ParserError<'a>> + Clone {
    choice((
        multi_part(),
        single_part(),
        plain_text_part(1).map(Part::PlainText),
    ))
    .repeated()
    .collect()
    .then_ignore(end())
}

/// The plain text part parser.
///
/// It parses everything that is inside and outside (multi)parts.
pub(crate) fn plain_text_part<'a>(
    min: usize,
) -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone {
    any()
        .and_is(
            choice((
                single_part_begin(),
                single_part_end(),
                multi_part_begin(),
                multi_part_end(),
            ))
            .not(),
        )
        .repeated()
        .at_least(min)
        .slice()
}

/// The single part parser.
///
/// It parses a full part, including properties and content till the
/// next opening part or the next closing part/multipart.
pub(crate) fn single_part<'a>() -> impl Parser<'a, &'a str, Part<'a>, ParserError<'a>> + Clone {
    single_part_begin()
        .ignore_then(
            choice((
                part_type(),
                filename(),
                name(),
                description(),
                disposition(),
                #[cfg(feature = "pgp")]
                encrypt(),
                #[cfg(feature = "pgp")]
                sign(),
            ))
            .repeated()
            .collect::<HashMap<_, _>>()
            .then_ignore(just(GREATER_THAN))
            .then_ignore(new_line().or_not()),
        )
        .then(plain_text_part(0))
        .then_ignore(single_part_end().then(new_line().or_not()).or_not())
        .map(|(props, content)| Part::Single(props, content))
}

/// The multipart parser.
///
/// It parses everything between tags `<#multipart>` and
/// `<#/multipart>`. A multipart can contain multiple parts as well as
/// multiple other multiparts (recursively). This parser is useful
/// when you need to group parts together instead of having them at
/// the root level.
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
pub(crate) fn multi_part<'a>() -> impl Parser<'a, &'a str, Part<'a>, ParserError<'a>> + Clone {
    recursive(|multipart| {
        just(MULTI_PART_BEGIN)
            .ignore_then(
                choice((
                    multipart_type(),
                    description(),
                    #[cfg(feature = "pgp")]
                    encrypt(),
                    #[cfg(feature = "pgp")]
                    sign(),
                ))
                .repeated()
                .collect::<Props>(),
            )
            .then_ignore(just(GREATER_THAN))
            .then_ignore(new_line().or_not())
            .then(
                choice((
                    multipart,
                    single_part(),
                    plain_text_part(1).map(Part::PlainText),
                ))
                .repeated()
                .collect(),
            )
            .then_ignore(just(MULTI_PART_END))
            .then_ignore(new_line().or_not())
            .map(|(props, parts)| Part::Multi(props, parts))
    })
}

#[cfg(test)]
mod parts {
    use concat_with::concat_line;
    use std::collections::HashMap;

    use crate::message::body::{
        compiler::{parsers::prelude::*, tokens::Part},
        DISPOSITION, FILENAME, NAME, TYPE,
    };

    #[test]
    fn single_part_no_new_line() {
        assert_eq!(
            super::single_part()
                .parse("<#part>This is a plain text part.")
                .into_result(),
            Ok(Part::Single(
                HashMap::default(),
                "This is a plain text part."
            )),
        );

        assert_eq!(
            super::single_part()
                .parse("<#part>This is a plain text part.<#/part>")
                .into_result(),
            Ok(Part::Single(
                HashMap::default(),
                "This is a plain text part."
            )),
        );
    }

    #[test]
    fn single_part_new_line() {
        assert_eq!(
            super::single_part()
                .parse(concat_line!("<#part>", "This is a plain text part."))
                .into_result(),
            Ok(Part::Single(
                HashMap::default(),
                "This is a plain text part."
            )),
        );

        assert_eq!(
            super::single_part()
                .parse(concat_line!(
                    "<#part>",
                    "This is a plain text part.",
                    "",
                    "<#/part>",
                ))
                .into_result(),
            Ok(Part::Single(
                HashMap::default(),
                "This is a plain text part.\n\n"
            )),
        );
    }

    #[test]
    fn single_html_part() {
        assert_eq!(
            super::single_part()
                .parse(concat_line!(
                    "<#part type=text/html>",
                    "<h1>This is a HTML text part.</h1>",
                    "<#/part>"
                ))
                .into_result(),
            Ok(Part::Single(
                HashMap::from_iter([(TYPE.into(), "text/html".into())]),
                "<h1>This is a HTML text part.</h1>\n",
            )),
        );
    }

    #[test]
    fn attachment() {
        assert_eq!(
            super::single_part()
                .parse("<#part type=image/jpeg filename=~/rms.jpg disposition=inline><#/part>")
                .into_result(),
            Ok(Part::Single(
                HashMap::from_iter([
                    (TYPE, "image/jpeg"),
                    (FILENAME, "~/rms.jpg"),
                    (DISPOSITION, "inline")
                ]),
                ""
            )),
        );
    }

    #[test]
    fn multi_part() {
        assert_eq!(
            super::multi_part()
                .parse(concat_line!(
                    "<#multipart>",
                    "This is a plain text part.",
                    "<#/multipart>"
                ))
                .into_result(),
            Ok(Part::Multi(
                HashMap::default(),
                vec![Part::PlainText("This is a plain text part.\n")]
            )),
        );
    }

    #[test]
    fn nested_multi_part() {
        assert_eq!(
            super::multi_part()
                .parse(concat_line!(
                    "<#multipart>",
                    "<#multipart>",
                    "This is a plain text part.",
                    "<#/multipart>",
                    "<#/multipart>"
                ))
                .into_result(),
            Ok(Part::Multi(
                HashMap::default(),
                vec![Part::Multi(
                    HashMap::default(),
                    vec![Part::PlainText("This is a plain text part.\n")]
                )]
            )),
        );

        assert_eq!(
            super::multi_part()
                .parse(concat_line!(
                    "<#multipart>",
                    "<#multipart>",
                    "<#multipart>",
                    "<#multipart>",
                    "This is a plain text part.",
                    "<#/multipart>",
                    "<#/multipart>",
                    "<#/multipart>",
                    "<#/multipart>"
                ))
                .into_result(),
            Ok(Part::Multi(
                HashMap::default(),
                vec![Part::Multi(
                    HashMap::default(),
                    vec![Part::Multi(
                        HashMap::default(),
                        vec![Part::Multi(
                            HashMap::default(),
                            vec![Part::PlainText("This is a plain text part.\n")]
                        )]
                    )]
                )]
            )),
        );
    }

    #[test]
    fn adjacent_multi_part() {
        assert_eq!(
            super::multi_part()
                .parse(concat_line!(
                    "<#multipart>",
                    "<#multipart>",
                    "This is a plain text part.",
                    "<#/multipart>",
                    "<#multipart>",
                    "This is a new plain text part.",
                    "<#/multipart>",
                    "<#/multipart>"
                ))
                .into_result(),
            Ok(Part::Multi(
                HashMap::default(),
                vec![
                    Part::Multi(
                        HashMap::default(),
                        vec![Part::PlainText("This is a plain text part.\n")]
                    ),
                    Part::Multi(
                        HashMap::default(),
                        vec![Part::PlainText("This is a new plain text part.\n")]
                    )
                ]
            )),
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
                .into_result(),
            Ok(vec![Part::Multi(
                HashMap::from_iter([(TYPE.into(), "alternative")]),
                vec![
                    Part::PlainText("This is a plain text part.\n"),
                    Part::Single(
                        HashMap::from_iter([(TYPE, "text/enriched")]),
                        "<center>This is a centered enriched part</center>\n"
                    )
                ]
            )]),
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
            vec![Part::Multi(
                HashMap::from_iter([(TYPE, "mixed")]),
                vec![
                    Part::Single(
                        HashMap::from_iter([
                            (TYPE, "image/jpeg"),
                            (FILENAME, "~/rms.jpg"),
                            (DISPOSITION, "inline")
                        ]),
                        ""
                    ),
                    Part::Multi(
                        HashMap::from_iter([(TYPE, "alternative")]),
                        vec![
                            Part::PlainText("This is a plain text part.\n"),
                            Part::Single(
                                HashMap::from_iter([
                                    (TYPE, "text/enriched"),
                                    (NAME, "enriched.txt")
                                ]),
                                "<center>This is a centered enriched part</center>\n",
                            )
                        ]
                    ),
                    Part::PlainText("This is a new plain text part.\n"),
                    Part::Single(
                        HashMap::from_iter([(DISPOSITION, "attachment")]),
                        "This plain text part is an attachment.\n",
                    )
                ]
            )]
        );
    }
}
