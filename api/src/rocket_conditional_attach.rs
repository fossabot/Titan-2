use rocket::{fairing::Fairing, Rocket};

pub trait ConditionalAttach {
    fn attach_if(self, condition: bool, fairing: impl Fairing) -> Self;
}

impl ConditionalAttach for Rocket {
    fn attach_if(self, condition: bool, fairing: impl Fairing) -> Self {
        if condition {
            self.attach(fairing)
        } else {
            self
        }
    }
}
