use mml::MmlCompiler;

#[tokio::main]
async fn main() {
    env_logger::builder().is_test(true).init();

    let mml = include_str!("./html.eml");
    let mml_compile_res = MmlCompiler::new().compile(&mml).unwrap();
    let mime = mml_compile_res.to_string().await.unwrap();

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
