use concat_with::concat_line;
use pimalaya_email_tpl::{CompilerBuilder, TplBuilder};
use pimalaya_process::Cmd;
use regex::Regex;

fn main() {
    let tpl = TplBuilder::default()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Subject: Sending signed message with rust-mml")
        .text_plain_part(concat_line!(
            "<#part type=text/plain sign=command>",
            "This is an signed message!",
            "<#/part>",
        ))
        .build();

    let mime_msg = tpl.compile(
        CompilerBuilder::default()
            .pgp_sign_cmd("gpg -saq -o - --recipient-file ./examples/keys/alice.key"),
    );
    let mime_msg = String::from_utf8_lossy(&mime_msg.unwrap()).to_string();

    let signature = Regex::new(r"(-*BEGIN PGP MESSAGE-*(?s:.)*-*END PGP MESSAGE-*)").unwrap();
    let signature = signature.captures_iter(&mime_msg).next().unwrap();
    let code = Cmd::from("gpg --verify --recipient-file ./tests/keys/alice.pub")
        .run_with(signature[0].as_bytes())
        .unwrap()
        .code;

    println!("");
    println!("================================");
    println!("TEMPLATE");
    println!("================================");
    println!("");
    println!("{}", *tpl);

    println!("");
    println!("================================");
    println!("COMPILED MIME MESSAGE");
    println!("================================");
    println!("");
    println!("{}", mime_msg);

    println!("");
    println!("================================");
    println!("SIGNATURE VERIFIED");
    println!("================================");
    println!("");
    println!("{}", code == 0);
}
