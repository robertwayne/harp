//! This is a wrapper around Sender<Action> for ensuring the returned inner
//! value of the `create_service` functions (Result<T, E) are used.
use std::ops::{Deref, DerefMut};

use crate::action::Action;

#[must_use = "The returned send channel hasn't been used anywhere. This means a socket is open to the Harp server on a seperate task, but never utilized."]
pub struct Sender(pub flume::Sender<Action>);

impl Deref for Sender {
    type Target = flume::Sender<Action>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Sender {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
