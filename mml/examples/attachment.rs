use mml::MmlCompilerBuilder;

#[tokio::main]
async fn main() {
    env_logger::builder().is_test(true).init();

    let mml = include_str!("./attachment.eml");
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
