#[cfg(feature = "async-std")]
use async_std::main;
#[cfg(feature = "tokio")]
use tokio::main;

#[cfg(feature = "pgp-gpg")]
#[test_log::test(main)]
async fn main() {
    use std::path::PathBuf;

    use mml::{
        pgp::{Pgp, PgpGpg},
        MmlCompilerBuilder,
    };

    let mml = include_str!("./pgp.eml");
    let mml_compiler = MmlCompilerBuilder::new()
        .with_pgp(Pgp::Gpg(PgpGpg {
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

#[cfg(not(feature = "pgp-gpg"))]
#[test_log::test(main)]
async fn main() {
    panic!("The pgp-gpg cargo feature should be enabled to run this example.");
}
