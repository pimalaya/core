use concat_with::concat_line;
use pimalaya_email_tpl::{CompilerBuilder, TplBuilder};
use pimalaya_process::Cmd;
use regex::Regex;

fn main() {
    let tpl = TplBuilder::default()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Subject: Sending encrypted message with rust-mml")
        .text_plain_part(concat_line!(
            "<#part type=text/plain encrypt=command>",
            "This is an encrypted message!",
            "<#/part>",
        ))
        .build();

    let mime_msg = tpl
        .compile(CompilerBuilder::default().pgp_encrypt_cmd(
            "gpg -aeqr <recipient> -o - --recipient-file ./examples/keys/bob.pub",
        ));
    let mime_msg = String::from_utf8_lossy(&mime_msg.unwrap()).to_string();

    let encrypted_part = Regex::new(r"(-*BEGIN PGP MESSAGE-*(?s:.)*-*END PGP MESSAGE-*)").unwrap();
    let encrypted_part = encrypted_part.captures_iter(&mime_msg).next().unwrap();
    let decrypted_part = Cmd::from("gpg -dq --recipient-file ./tests/keys/bob.key")
        .run_with(encrypted_part[0].as_bytes())
        .unwrap()
        .stdout;
    let decrypted_part = String::from_utf8_lossy(&decrypted_part).to_string();

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
    println!("DECRYPTED PART");
    println!("================================");
    println!("");
    println!("{}", decrypted_part);
}
