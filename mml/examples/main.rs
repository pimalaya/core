#[cfg(feature = "async-std")]
use async_std::main;
use mml::MmlCompilerBuilder;
#[cfg(feature = "tokio")]
use tokio::main;

#[test_log::test(main)]
async fn main() {
    let mml = include_str!("./main.mml.eml");
    let mml_compiler = MmlCompilerBuilder::new().build(mml).unwrap();
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
