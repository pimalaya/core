use mml::MmlCompiler;

#[tokio::main]
async fn main() {
    env_logger::builder().is_test(true).init();

    let mml = include_str!("./html.eml");
    let mime = MmlCompiler::new()
        .compile(&mml)
        .await
        .unwrap()
        .write_to_string()
        .unwrap();

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
