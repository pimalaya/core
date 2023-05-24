use mail_builder::{mime::MimePart, MessageBuilder};
use mail_parser::Message;
use pimalaya_email_tpl::InterpreterBuilder;

pub fn main() {
    let raw_msg = MessageBuilder::new()
        .from("from@localhost")
        .to("to@localhost")
        .subject("subject")
        .body(MimePart::new_multipart(
            "multipart/mixed",
            vec![MimePart::new_multipart(
                "multipart/alternative",
                vec![
                    MimePart::new_text("Hello, world!"),
                    MimePart::new_html("<h1>Hello, world!</h1>"),
                ],
            )],
        ))
        .write_to_string()
        .unwrap();
    let msg = Message::parse(raw_msg.as_bytes()).unwrap();
    let tpl = InterpreterBuilder::new()
        .show_multiparts()
        .build()
        .interpret(&msg)
        .unwrap();

    println!("");
    println!("================================");
    println!("RAW MIME MESSAGE");
    println!("================================");
    println!("{raw_msg}");

    println!("");
    println!("================================");
    println!("INTERPRETED TPL");
    println!("================================");
    println!("{tpl}");
}
