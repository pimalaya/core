use mail_builder::MessageBuilder;
use pimalaya_email_tpl::TplInterpreter;

fn main() {
    let email_builder = MessageBuilder::new()
        .message_id("id@localhost")
        .date(0 as u64)
        .from("from@localhost")
        .to("to@localhost")
        .subject("subject")
        .text_body("Hello, world!");

    let raw_email = email_builder.write_to_string().unwrap();

    let tpl = TplInterpreter::new()
        .show_only_headers(["From", "Subject"])
        .interpret_bytes(raw_email.as_bytes())
        .unwrap();

    println!();
    println!("================================");
    println!("RAW EMAIL");
    println!("================================");
    println!("{raw_email}");

    println!();
    println!("================================");
    println!("INTERPRETED TEMPLATE");
    println!("================================");
    println!("{tpl}", tpl = *tpl);
}
