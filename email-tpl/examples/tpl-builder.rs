use pimalaya_email_tpl::TplBuilder;

fn main() {
    let tpl = TplBuilder::default()
        .from("alice@localhost")
        .to("bob@localhost")
        .cc("patrick@localhost")
        .cc("paul@localhost")
        .subject("Template builder")
        .text_plain_part("Hello from a text/plain part!")
        .build();

    println!("");
    println!("================================");
    println!("BUILT TEMPLATE");
    println!("================================");
    println!("{}", *tpl);
}
