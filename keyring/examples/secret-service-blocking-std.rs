use keyring::{secret_service_blocking::std::progress, state::KeyringState2};

fn main() {
    let mut state = KeyringState2::new("service", "account");

    state.update_secret("caca");
    progress(&mut state).unwrap();
}
