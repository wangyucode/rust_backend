use thruster::{App, async_middleware, BasicContext, Request};

use crate::plaintext;

pub fn get_app() -> App<Request, BasicContext, ()> {
    let mut dota = App::<Request, BasicContext, ()>::new_basic();
    dota
        .post("/login", async_middleware!(BasicContext, [plaintext]));
    return dota;
}