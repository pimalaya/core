fn main() {
    let tpl = r#"
From: alice@localhost
To: bob@localhost
Subject: MML simple

See https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/Simple-MML-Example.html.

<#multipart type=alternative>
This is a plain text part.
<#part type=text/enriched>
<center>This is a centered enriched part</center>
<#/multipart>
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
