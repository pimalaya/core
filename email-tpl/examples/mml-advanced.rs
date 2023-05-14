fn main() {
    let tpl = r#"
From: alice@localhost
To: bob@localhost
Subject: MML advanced

See https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/Advanced-MML-Example.html.

<#multipart type=mixed>
<#part filename=./examples/attachment.png disposition=inline>
<#multipart type=alternative>
This is a plain text part.
<#part type=text/enriched name=enriched.txt>
<center>This is a centered enriched part</center>
<#/multipart>
This is a new plain text part.
<#part disposition=attachment>
This plain text part is an attachment.
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
