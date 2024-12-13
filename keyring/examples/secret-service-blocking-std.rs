use keyring::{event::KeyringEvent, secret_service_blocking::std::progress, state::KeyringState2};
use secrecy::ExposeSecret;

fn main() {
    let mut state = KeyringState2::new("service", "account");

    println!("update secret");
    state.update_secret("caca");
    progress(&mut state).unwrap();

    state.read_secret();
    let KeyringEvent::SecretRead(secret) = progress(&mut state).unwrap().unwrap() else {
        unreachable!()
    };

    println!("secret: {secret:?}");
    println!("exposed secret: {:?}", secret.expose_secret());
}
