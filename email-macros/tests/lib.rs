use email_macros::EmailBackendContext;

#[test]
fn test() {
    struct Ctx1;
    struct Ctx2;

    #[derive(EmailBackendContext)]
    struct MyContext {
        #[context]
        ctx1: Ctx1,
        ctx2: Ctx2,
    }
}
