use mail_parser::{Addr, ContentType, Group, HeaderValue};
use std::borrow::Cow;

pub(crate) fn display_value(val: &HeaderValue) -> String {
    match val {
        HeaderValue::Address(addr) => display_addr(addr),
        HeaderValue::AddressList(addrs) => display_addrs(addrs),
        HeaderValue::Group(group) => display_group(group),
        HeaderValue::GroupList(groups) => display_groups(groups),
        HeaderValue::Text(text) => text.to_string(),
        HeaderValue::TextList(texts) => display_texts(texts),
        HeaderValue::DateTime(datetime) => datetime.to_rfc822(),
        HeaderValue::ContentType(ctype) => display_content_type(ctype),
        HeaderValue::Empty => String::new(),
    }
}

fn display_addr(addr: &Addr) -> String {
    let email = match &addr.address {
        Some(addr) => addr.to_string(),
        None => "unknown".into(),
    };

    match &addr.name {
        Some(name) => format!("{name} <{email}>"),
        None => email.to_string(),
    }
}

fn display_addrs(addrs: &[Addr]) -> String {
    addrs.iter().fold(String::new(), |mut addrs, addr| {
        if !addrs.is_empty() {
            addrs.push_str(", ");
        }
        addrs.push_str(&display_addr(addr));
        addrs
    })
}

fn display_group(group: &Group) -> String {
    let name = match &group.name {
        Some(name) => name.to_string(),
        None => "unknown".into(),
    };

    let addrs = display_addrs(&group.addresses);
    format!("{name}:{addrs};")
}

fn display_groups(groups: &[Group]) -> String {
    groups.iter().fold(String::new(), |mut groups, group| {
        if !groups.is_empty() {
            groups.push(' ')
        }
        groups.push_str(&display_group(group));
        groups
    })
}

fn display_texts(texts: &[Cow<str>]) -> String {
    texts.iter().fold(String::new(), |mut texts, text| {
        if !texts.is_empty() {
            texts.push(' ');
        }
        texts.push_str(text);
        texts
    })
}

fn display_content_type(ctype: &ContentType) -> String {
    let attrs = ctype.attributes().unwrap_or_default().iter().fold(
        String::new(),
        |mut attrs, (key, val)| {
            attrs.push_str(&format!("; {key}={val}"));
            attrs
        },
    );
    let stype = ctype.subtype().unwrap_or("unknown");
    let ctype = ctype.ctype();

    format!("{ctype}/{stype}{attrs}")
}

#[cfg(test)]
mod tests {
    use mail_parser::{Addr, ContentType, Group};

    #[test]
    fn display_empty_addr() {
        let addr = Addr {
            name: None,
            address: None,
        };

        assert_eq!(super::display_addr(&addr), "unknown");
    }

    #[test]
    fn display_nameless_addr() {
        let addr = Addr {
            name: None,
            address: Some("test@localhost".into()),
        };

        assert_eq!(super::display_addr(&addr), "test@localhost");
    }

    #[test]
    fn display_named_addr() {
        let addr = Addr {
            name: Some("Test".into()),
            address: None,
        };

        assert_eq!(super::display_addr(&addr), "Test <unknown>");

        let addr = Addr {
            name: Some("Test".into()),
            address: Some("test@localhost".into()),
        };

        assert_eq!(super::display_addr(&addr), "Test <test@localhost>");
    }

    #[test]
    fn display_addrs() {
        let addrs = [
            Addr {
                name: None,
                address: None,
            },
            Addr {
                name: None,
                address: Some("test@localhost".into()),
            },
            Addr {
                name: Some("Test".into()),
                address: Some("test@localhost".into()),
            },
        ];

        assert_eq!(
            super::display_addrs(&addrs),
            "unknown, test@localhost, Test <test@localhost>"
        );
    }

    #[test]
    fn display_nameless_group() {
        let group = Group {
            name: None,
            addresses: Vec::new(),
        };

        assert_eq!(super::display_group(&group), "unknown:;");
    }

    #[test]
    fn display_named_group() {
        let group = Group {
            name: Some("Test".into()),
            addresses: vec![
                Addr {
                    name: None,
                    address: None,
                },
                Addr {
                    name: None,
                    address: Some("test@localhost".into()),
                },
                Addr {
                    name: Some("Test".into()),
                    address: Some("test@localhost".into()),
                },
            ],
        };

        assert_eq!(
            super::display_group(&group),
            "Test:unknown, test@localhost, Test <test@localhost>;"
        );
    }

    #[test]
    fn display_groups() {
        let groups = [
            Group {
                name: Some("Test".into()),
                addresses: vec![Addr {
                    name: None,
                    address: None,
                }],
            },
            Group {
                name: Some("Test".into()),
                addresses: vec![
                    Addr {
                        name: None,
                        address: Some("test@localhost".into()),
                    },
                    Addr {
                        name: Some("Test".into()),
                        address: Some("test@localhost".into()),
                    },
                ],
            },
        ];

        assert_eq!(
            super::display_groups(&groups),
            "Test:unknown; Test:test@localhost, Test <test@localhost>;"
        );
    }

    #[test]
    fn display_texts() {
        let texts = ["test".into(), "test".into(), "test".into()];
        assert_eq!(super::display_texts(&texts), "test test test");
    }

    #[test]
    fn display_subtypeless_content_type() {
        let ctype = ContentType {
            c_type: "text".into(),
            c_subtype: None,
            attributes: None,
        };

        assert_eq!(super::display_content_type(&ctype), "text/unknown");
    }

    #[test]
    fn display_content_type() {
        let ctype = ContentType {
            c_type: "text".into(),
            c_subtype: Some("plain".into()),
            attributes: Some(vec![
                ("key".into(), "val".into()),
                ("key2".into(), "val2".into()),
            ]),
        };

        assert_eq!(
            super::display_content_type(&ctype),
            "text/plain; key=val; key2=val2"
        );
    }
}
