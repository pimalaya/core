use std::collections::HashMap;

use crate::lexer::{
    part::Part,
    tpl::{Headers, Key, Tpl, Val},
};

use super::{attachment, multi_part, prelude::*, single_part, text_plain_part, COLON};

/// Represents the template MIME header parser.
fn header<T: ToString>(key: T) -> impl Parser<char, (Key, Val), Error = Simple<char>> {
    just(key.to_string())
        .then_ignore(just(COLON).padded())
        .then(text::newline().not().repeated().collect())
}

/// Represents the template MIME headers parser.
fn headers() -> impl Parser<char, Headers, Error = Simple<char>> {
    choice((
        header("Message-ID"),
        header("In-Reply-To"),
        header("Subject"),
        header("From"),
        header("To"),
        header("Reply-To"),
        header("Cc"),
        header("Bcc"),
    ))
    .separated_by(text::newline())
    .map(HashMap::from_iter)
}

/// Represents the template parser. It parses MIME headers followed by
/// parts.
pub(crate) fn tpl() -> impl Parser<char, Tpl, Error = Simple<char>> {
    headers()
        .then_ignore(text::newline().repeated().at_least(2))
        .then(
            choice((
                multi_part(),
                attachment(),
                single_part(),
                text_plain_part().map(Part::TextPlainPart),
            ))
            .repeated(),
        )
        .then_ignore(end())
        .map(Tpl::from)
}

#[cfg(test)]
mod tpl {
    use concat_with::concat_line;

    use crate::lexer::part::{DISPOSITION, FILENAME, NAME, TYPE};

    use super::*;

    // example from the [Emacs MML] module:
    //
    // [Emacs MML]: https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/Simple-MML-Example.html
    #[test]
    fn simple_mml() {
        assert_eq!(
            tpl().parse(concat_line!(
                "From: from",
                "To: to",
                "Subject: subject",
                "",
                "<#multipart type=alternative>",
                "This is a plain text part.",
                "<#part type=text/enriched>",
                "<center>This is a centered enriched part</center>",
                "<#/multipart>",
            )),
            Ok(Tpl::from((
                HashMap::from_iter([
                    ("From".into(), "from".into()),
                    ("To".into(), "to".into()),
                    ("Subject".into(), "subject".into())
                ]),
                vec![Part::MultiPart((
                    HashMap::from_iter([(TYPE.into(), "alternative".into())]),
                    vec![
                        Part::TextPlainPart("This is a plain text part.".into()),
                        Part::SinglePart((
                            HashMap::from_iter([(TYPE.into(), "text/enriched".into())]),
                            String::from("<center>This is a centered enriched part</center>")
                        ))
                    ]
                ))]
            ))),
        );
    }

    // example from the [Emacs MML] module:
    //
    // [Emacs MML]: https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/Advanced-MML-Example.html
    #[test]
    fn advanced_mml() {
        assert_eq!(
            tpl().parse(concat_line!(
                "From: from",
                "To: to",
                "Subject: subject",
                "",
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
            )),
            Ok(Tpl::from((
                HashMap::from_iter([
                    ("From".into(), "from".into()),
                    ("To".into(), "to".into()),
                    ("Subject".into(), "subject".into())
                ]),
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
                                Part::TextPlainPart("This is a plain text part.".into()),
                                Part::SinglePart((
                                    HashMap::from_iter([
                                        (TYPE.into(), "text/enriched".into()),
                                        (NAME.into(), "enriched.txt".into())
                                    ]),
                                    String::from(
                                        "<center>This is a centered enriched part</center>"
                                    )
                                ))
                            ]
                        )),
                        Part::TextPlainPart("This is a new plain text part.".into()),
                        Part::SinglePart((
                            HashMap::from_iter([(DISPOSITION.into(), "attachment".into())]),
                            String::from("This plain text part is an attachment.")
                        ))
                    ]
                ))]
            ))),
        );
    }
}
