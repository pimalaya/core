fn main() {
    let tpl = r#"
From: alice@localhost
To: bob@localhost
Subject: Sending attachments with rust-mml

Default attachment
<#part filename=./examples/attachment.png>

Inline attachment
<#part filename=./examples/attachment.png disposition=inline>

Custom attachment name
<#part filename=./examples/attachment.png name=custom.png>
"#;

    let mime_msg = pimalaya_email_tpl::compile(tpl.trim_start()).unwrap();

    println!("");
    println!("================================");
    println!("TEMPLATE");
    println!("================================");
    println!("{}", tpl);

    println!("================================");
    println!("COMPILED MIME MESSAGE");
    println!("================================");
    println!("");
    println!("{}", String::from_utf8_lossy(&mime_msg));
}
