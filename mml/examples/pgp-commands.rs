#[cfg(feature = "pgp-commands")]
#[tokio::main]
async fn main() {
    use mml::{
        pgp::{Pgp, PgpCommands},
        MmlCompilerBuilder,
    };
    use process::Command;

    let mml = include_str!("./pgp.eml");
    let mml_compiler = MmlCompilerBuilder::new()
        .with_pgp(Pgp::Commands(PgpCommands {
            encrypt_cmd: Some(Command::new(
                "gpg --homedir ./tests/gpg-home -eqa <recipients>",
            )),
            encrypt_recipient_fmt: Some(PgpCommands::default_encrypt_recipient_fmt()),
            encrypt_recipients_sep: Some(PgpCommands::default_encrypt_recipients_sep()),
            decrypt_cmd: Some(Command::new("gpg --homedir ./tests/gpg-home -dq")),
            sign_cmd: Some(Command::new("gpg --homedir ./tests/gpg-home -saq")),
            verify_cmd: Some(Command::new("gpg --homedir ./tests/gpg-home --verify -q")),
        }))
        .build(mml)
        .unwrap();
    let mime = mml_compiler.compile().await.unwrap().into_string().unwrap();

    println!("================================");
    println!("MML MESSAGE");
    println!("================================");
    println!();
    println!("{mml}");

    println!("================================");
    println!("COMPILED MIME MESSAGE");
    println!("================================");
    println!();
    println!("{mime}");
}

#[cfg(not(feature = "pgp-commands"))]
#[tokio::main]
async fn main() {
    panic!("The pgp-commands cargo feature should be enabled to run this example.");
}
