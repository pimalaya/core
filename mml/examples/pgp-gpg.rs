#[cfg(not(feature = "pgp-gpg"))]
fn main() {
    use std::process::exit;

    eprintln!("Cargo feature pgp-gpg is missing.");
    eprintln!("Please re-run the command with --features pgp-gpg.");
    exit(-1)
}

#[cfg(feature = "pgp-gpg")]
#[tokio::main]
async fn main() {
    use mml::{
        pgp::{Gpg, Pgp},
        MmlCompilerBuilder,
    };
    use std::path::PathBuf;

    env_logger::builder().is_test(true).init();

    let mml = include_str!("./pgp.eml");
    let mml_compiler = MmlCompilerBuilder::new()
        .with_pgp(Pgp::Gpg(Gpg {
            home_dir: Some(PathBuf::from("./tests/gpg-home")),
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
