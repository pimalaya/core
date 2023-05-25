use concat_with::concat_line;
use pimalaya_email_tpl::{CompilerBuilder, InterpreterBuilder, Tpl};

fn gpg(args: &str) -> String {
    String::from("gpg --no-default-keyring --keyring ../.keyring.gpg ") + args
}

#[test]
fn pgp() {
    let tpl = Tpl::from(concat_line!(
        "From: alice@localhost",
        "To: bob@localhost",
        "Subject: subject",
        "",
        "<#part type=text/plain encrypt=command sign=command>",
        "Encrypted and signed message!",
    ));

    let compiler = CompilerBuilder::new()
        .pgp_encrypt_cmd(gpg("-eaqr <recipient> -o -"))
        .pgp_encrypt_recipient("bob@localhost")
        .pgp_sign_cmd(gpg("-saqu alice -o -"));

    let raw_msg = tpl.compile(compiler).unwrap();

    let interpreter = InterpreterBuilder::new()
        .show_headers(["From", "To", "Subject"])
        .pgp_decrypt_cmd(gpg("-dq"))
        .pgp_verify_cmd(gpg("--verify -q"));

    let tpl = Tpl::interpret(interpreter, raw_msg).unwrap();

    let expected_tpl = Tpl::from(concat_line!(
        "From: alice@localhost",
        "To: bob@localhost",
        "Subject: subject",
        "",
        "Encrypted and signed message!",
        ""
    ));

    assert_eq!(tpl, expected_tpl);
}
